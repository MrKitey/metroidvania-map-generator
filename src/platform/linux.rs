//! Wayland data-device client. One state machine for file drops.

use super::drop::file_uri;
use std::fs::File;
use std::io::{ErrorKind, Read};
use std::os::fd::{AsFd, AsRawFd, FromRawFd, OwnedFd};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;
use wayland_client::protocol::wl_data_device::WlDataDevice;
use wayland_client::protocol::wl_data_device_manager::{DndAction, WlDataDeviceManager};
use wayland_client::protocol::wl_data_offer::WlDataOffer;
use wayland_client::protocol::wl_registry;
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::protocol::{wl_data_device, wl_data_offer};
use wayland_client::{
    Connection, Dispatch, EventQueue, Proxy as _, QueueHandle, delegate_noop, event_created_child,
};

const URI_LIST: &str = "text/uri-list";
/// Cap on one offer, so a bad source cannot fill memory.
const MAX_BYTES: usize = 1024 * 1024;

pub struct LinuxDrop {
    queue: EventQueue<DropState>,
    state: DropState,
    ctx: egui::Context,
}

struct MimeList {
    types: Mutex<Vec<String>>,
}

impl Default for MimeList {
    fn default() -> MimeList {
        MimeList {
            types: Mutex::new(Vec::new()),
        }
    }
}

struct Incoming {
    offer: WlDataOffer,
    file: File,
    buf: Vec<u8>,
}

struct Drag {
    offer: WlDataOffer,
    serial: u32,
    accepted: bool,
}

struct DropState {
    conn: Connection,
    seat: Option<WlSeat>,
    manager: Option<WlDataDeviceManager>,
    device: Option<WlDataDevice>,
    drag: Option<Drag>,
    reading: Option<Incoming>,
    ready: Vec<PathBuf>,
}

impl LinuxDrop {
    pub fn start(ctx: egui::Context, cc: &eframe::CreationContext<'_>) -> Option<LinuxDrop> {
        use raw_window_handle::{HasDisplayHandle as _, RawDisplayHandle};
        let handle = cc.display_handle().ok()?;
        let RawDisplayHandle::Wayland(wayland) = handle.as_raw() else {
            return None;
        };
        let backend = unsafe {
            wayland_client::backend::Backend::from_foreign_display(wayland.display.as_ptr().cast())
        };
        let conn = Connection::from_backend(backend);
        let mut queue = conn.new_event_queue();
        let qh = queue.handle();
        let mut state = DropState {
            conn: conn.clone(),
            seat: None,
            manager: None,
            device: None,
            drag: None,
            reading: None,
            ready: Vec::new(),
        };
        let _registry = conn.display().get_registry(&qh, ());
        let _ = conn.flush();
        let fd = conn.backend().poll_fd().try_clone_to_owned().ok()?;
        spawn_waker(fd, ctx.clone());
        // The registry reply arrives on a later frame, once winit reads the socket.
        let _ = queue.dispatch_pending(&mut state);
        Some(LinuxDrop { queue, state, ctx })
    }

    pub fn poll(&mut self) -> Vec<PathBuf> {
        if self.queue.dispatch_pending(&mut self.state).is_err() {
            return Vec::new();
        }
        let _ = self.queue.flush();
        self.state.pump_read();
        if self.state.drag.is_some() || self.state.reading.is_some() {
            self.ctx.request_repaint_after(Duration::from_millis(16));
        }
        std::mem::take(&mut self.state.ready)
    }
}

fn spawn_waker(fd: OwnedFd, ctx: egui::Context) {
    std::thread::Builder::new()
        .name("wayland-drop".to_owned())
        .spawn(move || {
            loop {
                let mut pollfd = libc::pollfd {
                    fd: fd.as_raw_fd(),
                    events: libc::POLLIN,
                    revents: 0,
                };
                let ready = unsafe { libc::poll(&mut pollfd, 1, -1) };
                if ready < 0 {
                    break;
                }
                if pollfd.revents & (libc::POLLHUP | libc::POLLERR | libc::POLLNVAL) != 0
                    && pollfd.revents & libc::POLLIN == 0
                {
                    break;
                }
                // Twice, so a frame that runs before winit reads the socket is followed
                // by one that sees the queued events.
                ctx.request_repaint();
                std::thread::sleep(Duration::from_millis(30));
                ctx.request_repaint();
            }
        })
        .ok();
}

impl DropState {
    fn ensure_device(&mut self, qh: &QueueHandle<DropState>) {
        if self.device.is_some() {
            return;
        }
        let (Some(manager), Some(seat)) = (self.manager.clone(), self.seat.clone()) else {
            return;
        };
        self.device = Some(manager.get_data_device(&seat, qh, ()));
        let _ = self.conn.flush();
    }

    fn accept(&mut self, offer: &WlDataOffer) {
        let Some(drag) = &self.drag else {
            return;
        };
        if drag.accepted || drag.offer != *offer {
            return;
        }
        let serial = drag.serial;
        let wants_files = offer.data::<MimeList>().is_some_and(|list| {
            list.types
                .lock()
                .unwrap()
                .iter()
                .any(|mime| mime == URI_LIST)
        });
        if !wants_files {
            return;
        }
        offer.accept(serial, Some(URI_LIST.to_owned()));
        if offer.version() >= 3 {
            offer.set_actions(DndAction::Copy, DndAction::Copy);
        }
        if let Some(drag) = &mut self.drag {
            drag.accepted = true;
        }
        let _ = self.conn.flush();
    }

    fn start_read(&mut self, offer: WlDataOffer) {
        let mut pipes = [0; 2];
        if unsafe { libc::pipe2(pipes.as_mut_ptr(), libc::O_CLOEXEC) } != 0 {
            offer.destroy();
            return;
        }
        let read = unsafe { File::from_raw_fd(pipes[0]) };
        let write = unsafe { OwnedFd::from_raw_fd(pipes[1]) };
        offer.receive(URI_LIST.to_owned(), write.as_fd());
        drop(write);
        let _ = read.set_nonblocking();
        let _ = self.conn.flush();
        self.reading = Some(Incoming {
            offer,
            file: read,
            buf: Vec::new(),
        });
    }

    fn pump_read(&mut self) {
        let Some(incoming) = &mut self.reading else {
            return;
        };
        let mut tmp = [0u8; 8192];
        let done = loop {
            if incoming.buf.len() >= MAX_BYTES {
                break true;
            }
            match incoming.file.read(&mut tmp) {
                Ok(0) => break true,
                Ok(n) => incoming.buf.extend_from_slice(&tmp[..n]),
                Err(err) if err.kind() == ErrorKind::WouldBlock => break false,
                Err(_) => break true,
            }
        };
        if !done {
            return;
        }
        let incoming = self.reading.take().unwrap();
        if incoming.offer.version() >= 3 {
            incoming.offer.finish();
        }
        incoming.offer.destroy();
        let text = String::from_utf8_lossy(&incoming.buf);
        self.ready.extend(text.lines().filter_map(file_uri));
        let _ = self.conn.flush();
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for DropState {
    fn event(
        state: &mut DropState,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<DropState>,
    ) {
        let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        else {
            return;
        };
        match interface.as_str() {
            "wl_seat" if state.seat.is_none() && version >= 1 => {
                state.seat = Some(registry.bind(name, 1, qh, ()));
                state.ensure_device(qh);
            }
            "wl_data_device_manager" if state.manager.is_none() && version >= 1 => {
                state.manager = Some(registry.bind(name, version.min(3), qh, ()));
                state.ensure_device(qh);
            }
            _ => {}
        }
    }
}

impl Dispatch<WlDataDevice, ()> for DropState {
    fn event(
        state: &mut DropState,
        _: &WlDataDevice,
        event: wl_data_device::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<DropState>,
    ) {
        match event {
            wl_data_device::Event::Enter {
                serial,
                id: Some(offer),
                ..
            } => {
                if let Some(previous) = state.drag.take() {
                    previous.offer.destroy();
                }
                state.drag = Some(Drag {
                    offer: offer.clone(),
                    serial,
                    accepted: false,
                });
                state.accept(&offer);
            }
            wl_data_device::Event::Leave => {
                if let Some(drag) = state.drag.take()
                    && state.reading.is_none()
                {
                    drag.offer.destroy();
                }
            }
            wl_data_device::Event::Drop => {
                let Some(drag) = state.drag.take() else {
                    return;
                };
                if drag.accepted {
                    state.start_read(drag.offer);
                } else {
                    drag.offer.destroy();
                }
            }
            wl_data_device::Event::Selection { id: Some(offer) } => offer.destroy(),
            _ => {}
        }
    }

    event_created_child!(DropState, WlDataDevice, [
        wl_data_device::EVT_DATA_OFFER_OPCODE => (WlDataOffer, MimeList::default()),
    ]);
}

impl Dispatch<WlDataOffer, MimeList> for DropState {
    fn event(
        state: &mut DropState,
        offer: &WlDataOffer,
        event: wl_data_offer::Event,
        data: &MimeList,
        _: &Connection,
        _: &QueueHandle<DropState>,
    ) {
        if let wl_data_offer::Event::Offer { mime_type } = event {
            data.types.lock().unwrap().push(mime_type);
            state.accept(offer);
        }
    }
}

delegate_noop!(DropState: ignore WlSeat);
delegate_noop!(DropState: ignore WlDataDeviceManager);

trait NonBlocking {
    fn set_nonblocking(&self) -> std::io::Result<()>;
}

impl NonBlocking for File {
    fn set_nonblocking(&self) -> std::io::Result<()> {
        let fd = self.as_raw_fd();
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if flags < 0 {
            return Err(std::io::Error::last_os_error());
        }
        let rc = unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) };
        if rc < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
}
