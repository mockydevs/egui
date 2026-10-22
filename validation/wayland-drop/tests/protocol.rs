//! Protocol-level regression tests using an isolated Wayland server, never the user's display.
#![cfg(target_os = "linux")]
use std::io::Write;
use std::sync::Arc;
use wayland_protocols::xdg::shell::server::{
    xdg_positioner, xdg_surface, xdg_toplevel, xdg_wm_base,
};
use wayland_server::protocol::{
    wl_buffer, wl_compositor, wl_data_device, wl_data_device_manager, wl_data_offer,
    wl_data_source, wl_pointer, wl_region, wl_seat, wl_shm, wl_shm_pool, wl_surface,
};
use wayland_server::{
    Client, DataInit, Dispatch, Display, DisplayHandle, GlobalDispatch, New, Resource,
};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

#[derive(Default)]
struct State {
    surface: Option<wl_surface::WlSurface>,
    device: Option<wl_data_device::WlDataDevice>,
    offered: bool,
    dropped: bool,
    bytes: Vec<u8>,
    finished: bool,
    finish_count: usize,
    receives: usize,
    case: String,
    held: Option<std::fs::File>,
}
macro_rules! global {
    ($t:ty) => {
        impl GlobalDispatch<$t, ()> for State {
            fn bind(
                _: &mut Self,
                _: &DisplayHandle,
                _: &Client,
                r: New<$t>,
                _: &(),
                i: &mut DataInit<'_, Self>,
            ) {
                i.init(r, ());
            }
        }
    };
}
global!(wl_compositor::WlCompositor);
global!(xdg_wm_base::XdgWmBase);
global!(wl_data_device_manager::WlDataDeviceManager);
impl GlobalDispatch<wl_shm::WlShm, ()> for State {
    fn bind(
        _: &mut Self,
        _: &DisplayHandle,
        _: &Client,
        r: New<wl_shm::WlShm>,
        _: &(),
        i: &mut DataInit<'_, Self>,
    ) {
        let r = i.init(r, ());
        r.format(wl_shm::Format::Argb8888);
        r.format(wl_shm::Format::Xrgb8888);
    }
}
impl GlobalDispatch<wl_seat::WlSeat, ()> for State {
    fn bind(
        _: &mut Self,
        _: &DisplayHandle,
        _: &Client,
        r: New<wl_seat::WlSeat>,
        _: &(),
        i: &mut DataInit<'_, Self>,
    ) {
        let r = i.init(r, ());
        r.capabilities(wl_seat::Capability::Pointer);
        r.name("test-seat".into());
    }
}
impl Dispatch<wl_compositor::WlCompositor, ()> for State {
    fn request(
        s: &mut Self,
        _: &Client,
        _: &wl_compositor::WlCompositor,
        r: wl_compositor::Request,
        _: &(),
        _: &DisplayHandle,
        i: &mut DataInit<'_, Self>,
    ) {
        match r {
            wl_compositor::Request::CreateSurface { id } => {
                let surface = i.init(id, ());
                if s.surface.is_none() {
                    s.surface = Some(surface);
                }
            }
            wl_compositor::Request::CreateRegion { id } => {
                i.init(id, ());
            }
            _ => {}
        }
    }
}
impl Dispatch<wl_seat::WlSeat, ()> for State {
    fn request(
        _: &mut Self,
        _: &Client,
        _: &wl_seat::WlSeat,
        r: wl_seat::Request,
        _: &(),
        _: &DisplayHandle,
        i: &mut DataInit<'_, Self>,
    ) {
        if let wl_seat::Request::GetPointer { id } = r {
            i.init(id, ());
        }
    }
}
impl Dispatch<wl_shm::WlShm, ()> for State {
    fn request(
        _: &mut Self,
        _: &Client,
        _: &wl_shm::WlShm,
        r: wl_shm::Request,
        _: &(),
        _: &DisplayHandle,
        i: &mut DataInit<'_, Self>,
    ) {
        if let wl_shm::Request::CreatePool { id, .. } = r {
            i.init(id, ());
        }
    }
}
impl Dispatch<wl_shm_pool::WlShmPool, ()> for State {
    fn request(
        _: &mut Self,
        _: &Client,
        _: &wl_shm_pool::WlShmPool,
        r: wl_shm_pool::Request,
        _: &(),
        _: &DisplayHandle,
        i: &mut DataInit<'_, Self>,
    ) {
        if let wl_shm_pool::Request::CreateBuffer { id, .. } = r {
            i.init(id, ());
        }
    }
}
impl Dispatch<xdg_wm_base::XdgWmBase, ()> for State {
    fn request(
        s: &mut Self,
        _: &Client,
        _: &xdg_wm_base::XdgWmBase,
        r: xdg_wm_base::Request,
        _: &(),
        _: &DisplayHandle,
        i: &mut DataInit<'_, Self>,
    ) {
        match r {
            xdg_wm_base::Request::GetXdgSurface { id, surface } => {
                s.surface = Some(surface);
                i.init(id, ());
            }
            xdg_wm_base::Request::CreatePositioner { id } => {
                i.init(id, ());
            }
            _ => {}
        }
    }
}
impl Dispatch<xdg_surface::XdgSurface, ()> for State {
    fn request(
        s: &mut Self,
        c: &Client,
        rsc: &xdg_surface::XdgSurface,
        r: xdg_surface::Request,
        _: &(),
        d: &DisplayHandle,
        i: &mut DataInit<'_, Self>,
    ) {
        match r {
            xdg_surface::Request::GetToplevel { id } => {
                let top = i.init(id, ());
                top.configure(800, 600, vec![]);
                rsc.configure(1);
            }
            xdg_surface::Request::AckConfigure { .. } if !s.offered => {
                if let (Some(dev), Some(surface)) = (&s.device, &s.surface) {
                    let offer = c
                        .create_resource::<wl_data_offer::WlDataOffer, (), Self>(
                            d,
                            if s.case == "legacy" { 2 } else { 3 },
                            (),
                        )
                        .unwrap();
                    dev.data_offer(&offer);
                    offer.offer(
                        if s.case == "unsupported" {
                            "image/unknown"
                        } else if s.case == "portal" {
                            "application/vnd.portal.filetransfer"
                        } else {
                            "text/uri-list"
                        }
                        .into(),
                    );
                    dev.enter(2, surface, 123., 234., Some(&offer));
                    if offer.version() >= 3 {
                        offer.source_actions(wl_data_device_manager::DndAction::Copy);
                    }
                    if s.case == "clipboard" {
                        let selection = c
                            .create_resource::<wl_data_offer::WlDataOffer, (), Self>(d, 3, ())
                            .unwrap();
                        dev.data_offer(&selection);
                        selection.offer("text/plain;charset=utf-8".into());
                        dev.selection(Some(&selection));
                    }
                    s.offered = true;
                }
            }
            _ => {}
        }
    }
}
impl Dispatch<wl_data_device_manager::WlDataDeviceManager, ()> for State {
    fn request(
        s: &mut Self,
        _: &Client,
        _: &wl_data_device_manager::WlDataDeviceManager,
        r: wl_data_device_manager::Request,
        _: &(),
        _: &DisplayHandle,
        i: &mut DataInit<'_, Self>,
    ) {
        match r {
            wl_data_device_manager::Request::GetDataDevice { id, .. } => {
                s.device = Some(i.init(id, ()))
            }
            wl_data_device_manager::Request::CreateDataSource { id } => {
                i.init(id, ());
            }
            _ => {}
        }
    }
}
impl Dispatch<wl_data_offer::WlDataOffer, ()> for State {
    fn request(
        s: &mut Self,
        client: &Client,
        o: &wl_data_offer::WlDataOffer,
        r: wl_data_offer::Request,
        _: &(),
        display: &DisplayHandle,
        _: &mut DataInit<'_, Self>,
    ) {
        match r {
            wl_data_offer::Request::Accept {
                mime_type: Some(_), ..
            } if s.case == "legacy" && !s.dropped => {
                s.device.as_ref().unwrap().motion(50, 321., 432.);
                s.device.as_ref().unwrap().drop();
                s.device.as_ref().unwrap().leave();
                s.dropped = true;
            }
            wl_data_offer::Request::SetActions { dnd_actions, .. } if !s.dropped => {
                if dnd_actions.into_result().unwrap().is_empty() && s.case != "cancel" {
                    return;
                }
                if s.case == "cancel" {
                    s.device.as_ref().unwrap().leave();
                    s.dropped = true;
                    return;
                }
                s.device.as_ref().unwrap().motion(50, 321., 432.);
                o.action(if s.case == "invalid_action" {
                    wl_data_device_manager::DndAction::Move
                } else {
                    wl_data_device_manager::DndAction::Copy
                });
                s.device.as_ref().unwrap().drop();
                s.device.as_ref().unwrap().leave();
                s.dropped = true;
            }
            wl_data_offer::Request::Receive { fd, mime_type } => {
                if mime_type.starts_with("text/plain") {
                    write_payload(
                        std::fs::File::from(fd),
                        b"clipboard before and after drop\r\n".to_vec(),
                    );
                    return;
                }
                let mut f = std::fs::File::from(fd);
                s.receives += 1;
                if s.case == "overlap" && s.receives == 1 {
                    let bytes = s.bytes.clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(300));
                        f.write_all(&bytes).unwrap();
                    });
                    let next = client
                        .create_resource::<wl_data_offer::WlDataOffer, (), Self>(
                            display,
                            if s.case == "legacy" { 2 } else { 3 },
                            (),
                        )
                        .unwrap();
                    let device = s.device.as_ref().unwrap();
                    device.data_offer(&next);
                    next.offer("text/uri-list".into());
                    device.enter(3, s.surface.as_ref().unwrap(), 100., 200., Some(&next));
                    next.source_actions(wl_data_device_manager::DndAction::Copy);
                    s.dropped = false;
                } else if s.case == "overlap" {
                    f.write_all(b"file:///tmp/second.png\n").unwrap();
                } else if s.case == "stall" {
                    s.held = Some(f);
                } else {
                    write_payload(f, s.bytes.clone());
                }
            }
            wl_data_offer::Request::Finish => {
                s.finished = true;
                s.finish_count += 1;
            }
            _ => {}
        }
    }
}
macro_rules! ignore { ($($t:ty),*) => {$(impl Dispatch<$t,()> for State{fn request(_: &mut Self,_:&Client,_:&$t,_:<$t as Resource>::Request,_:&(),_:&DisplayHandle,_:&mut DataInit<'_,Self>) {}})*}; }
ignore!(
    wl_surface::WlSurface,
    wl_region::WlRegion,
    wl_buffer::WlBuffer,
    wl_pointer::WlPointer,
    wl_data_device::WlDataDevice,
    wl_data_source::WlDataSource,
    xdg_toplevel::XdgToplevel,
    xdg_positioner::XdgPositioner
);
fn write_payload(mut file: std::fs::File, bytes: Vec<u8>) {
    std::thread::spawn(move || {
        let mut remaining = bytes.as_slice();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while !remaining.is_empty() && std::time::Instant::now() < deadline {
            match file.write(remaining) {
                Ok(0) | Err(_) if std::time::Instant::now() >= deadline => break,
                Ok(n) => remaining = &remaining[n..],
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(1))
                }
                Err(_) => break,
            }
        }
    });
}
struct App {
    window: Option<Box<dyn Window>>,
    adapter: Option<egui_winit::State>,
    paths: Vec<std::path::PathBuf>,
    deadline: std::time::Instant,
    expected: usize,
    pasted: Vec<String>,
    requested_pastes: usize,
    errors: Vec<std::io::ErrorKind>,
    hover_seen: bool,
    hovering: bool,
    position: Option<winit::dpi::PhysicalPosition<f64>>,
}
impl ApplicationHandler for App {
    fn can_create_surfaces(&mut self, e: &dyn ActiveEventLoop) {
        let window = e
            .create_window(winit::window::WindowAttributes::default().with_decorations(false))
            .unwrap();
        self.adapter = Some(egui_winit::State::new(
            egui::Context::default(),
            egui::ViewportId::ROOT,
            &e,
            Some(1.),
            None,
            None,
        ));
        self.window = Some(window);
    }
    fn window_event(&mut self, e: &dyn ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        let adapter = self.adapter.as_mut().unwrap();
        eprintln!(
            "event: {}",
            egui_winit::short_window_event_description(&event)
        );
        if let WindowEvent::DataTransferReceived { value, .. } = &event {
            if let Err(error) = value.try_as_bytes() {
                self.errors.push(error.kind());
            }
        }
        let _ = adapter.on_window_event(window.as_ref(), e, &event);
        let input = adapter.take_egui_input(window.as_ref());
        self.hovering = !input.hovered_files.is_empty();
        self.hover_seen |= self.hovering;
        for event in &input.events {
            if let egui::Event::Paste(text) = event {
                self.pasted.push(text.clone());
            }
            if let egui::Event::PointerMoved(pos) = event {
                self.position = Some((pos.x as f64, pos.y as f64).into());
            }
        }
        self.paths.extend(
            input
                .dropped_files
                .iter()
                .map(|file| file.path().to_owned()),
        );
        if self.expected > 0
            && self.paths.len() == self.expected
            && (std::env::var("DROP_CASE").as_deref() != Ok("clipboard") || self.pasted.len() >= 2)
        {
            self.deadline = std::time::Instant::now() + std::time::Duration::from_millis(50);
        }
    }
    fn about_to_wait(&mut self, e: &dyn ActiveEventLoop) {
        if std::env::var("DROP_CASE").as_deref() == Ok("clipboard")
            && self.requested_pastes < 2
            && self.paths.len() == 2
        {
            eprintln!(
                "paste request {} clipboard {:?}",
                self.requested_pastes,
                e.clipboard()
            );
            self.adapter.as_mut().unwrap().request_paste(e);
            self.requested_pastes += 1;
        }
        if std::time::Instant::now() >= self.deadline {
            e.exit();
        } else {
            e.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(self.deadline));
        }
    }
}
#[test]
#[ignore = "subprocess helper; run by native_file_drop_protocol"]
fn native_client() {
    use winit::platform::wayland::EventLoopBuilderExtWayland;
    let e = EventLoop::builder().with_any_thread(true).build().unwrap();
    let case = std::env::var("DROP_CASE").unwrap_or_default();
    let expected = if case == "overlap" {
        3
    } else {
        usize::from(case == "valid" || case == "portal" || case == "legacy" || case == "clipboard")
            * 2
    };
    let mut a = App {
        window: None,
        adapter: None,
        paths: vec![],
        deadline: std::time::Instant::now()
            + std::time::Duration::from_millis(if case == "stall" {
                11500
            } else if case == "portal" {
                7000
            } else {
                1000
            }),
        expected,
        pasted: vec![],
        requested_pastes: 0,
        errors: vec![],
        hover_seen: false,
        hovering: false,
        position: None,
    };
    use winit::event_loop::run_on_demand::EventLoopExtRunOnDemand;
    let mut e = e;
    e.run_app_on_demand(&mut a).unwrap();
    assert!(!a.hovering, "hover state must clear on leave/drop");
    if case == "valid" {
        assert!(a.hover_seen);
    }
    if case == "stall" {
        assert_eq!(a.errors, vec![std::io::ErrorKind::TimedOut]);
    }
    if case == "oversize" {
        assert_eq!(a.errors, vec![std::io::ErrorKind::InvalidData]);
    }
    if case == "clipboard" {
        assert_eq!(a.pasted, vec!["clipboard before and after drop\n"; 2]);
    }
    if case == "overlap" {
        assert_eq!(
            a.paths,
            vec![
                std::path::PathBuf::from("/tmp/second.png"),
                std::path::PathBuf::from("/tmp/a b.png"),
                std::path::PathBuf::from("/tmp/✓.jpg")
            ]
        );
        println!("overlapping drops: both transfers completed");
        return;
    }
    if expected == 0 {
        assert!(a.paths.is_empty());
        println!("native protocol rejected {case}");
        return;
    }
    if case == "portal" {
        let expected = std::env::var("DROP_HASHES").unwrap();
        let got = a
            .paths
            .iter()
            .map(|p| hash(&std::fs::read(p).unwrap_or_else(|e| panic!("portal path {p:?}: {e}"))))
            .collect::<Vec<_>>()
            .join(",");
        assert_eq!(got, expected);
        println!("portal files readable by native receiver");
        return;
    }
    assert_eq!(
        a.position,
        Some(winit::dpi::PhysicalPosition::new(321., 432.))
    );
    assert_eq!(
        a.paths,
        vec![
            std::path::PathBuf::from("/tmp/a b.png"),
            std::path::PathBuf::from("/tmp/✓.jpg")
        ]
    );
    println!("native protocol drop: 2 paths delivered");
}

fn run_case(case: &str) {
    let case = case.to_owned();
    let dir = tempfile::tempdir().unwrap();
    let socket =
        wayland_server::ListeningSocket::bind_absolute(dir.path().join("test-wayland")).unwrap();
    let mut display = Display::<State>::new().unwrap();
    let h = display.handle();
    h.create_global::<State, wl_compositor::WlCompositor, _>(4, ());
    h.create_global::<State, wl_shm::WlShm, _>(1, ());
    h.create_global::<State, wl_seat::WlSeat, _>(7, ());
    h.create_global::<State, xdg_wm_base::XdgWmBase, _>(3, ());
    h.create_global::<State, wl_data_device_manager::WlDataDeviceManager, _>(
        if case == "legacy" { 2 } else { 3 },
        (),
    );
    let portal_connection = if case == "portal" {
        Some(dbus::blocking::Connection::new_session().unwrap())
    } else {
        None
    };
    let mut files = vec![];
    let mut hashes = vec![];
    let portal_key = portal_connection.as_ref().map(|conn| {
        let proxy = conn.with_proxy(
            "org.freedesktop.portal.Documents",
            "/org/freedesktop/portal/documents",
            std::time::Duration::from_secs(5),
        );
        let (key,): (String,) = proxy
            .method_call(
                "org.freedesktop.portal.FileTransfer",
                "StartTransfer",
                (dbus::arg::PropMap::new(),),
            )
            .unwrap();
        for n in 0..2 {
            let mut file = tempfile::NamedTempFile::new().unwrap();
            let bytes = format!("original test image {n}").into_bytes();
            file.write_all(&bytes).unwrap();
            hashes.push(hash(&bytes));
            files.push(file);
        }
        let fds: Vec<dbus::arg::OwnedFd> = files
            .iter()
            .map(|file| file.as_file().try_clone().unwrap().into())
            .collect();
        let _: () = proxy
            .method_call(
                "org.freedesktop.portal.FileTransfer",
                "AddFiles",
                (&key, fds, dbus::arg::PropMap::new()),
            )
            .unwrap();
        key
    });
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    let mut child = command
        .arg("--exact")
        .arg("native_client")
        .arg("--ignored")
        .arg("--nocapture")
        .env("DROP_HASHES", hashes.join(","))
        .env("DROP_CASE", &case)
        .env("WAYLAND_DISPLAY", dir.path().join("test-wayland"))
        .env_remove("DISPLAY")
        .spawn()
        .unwrap();
    let bytes = match case.as_str() {
        "portal" => portal_key.unwrap().into_bytes(),
        "remote" => b"file://remote/private.png".to_vec(),
        "nul" => b"file:///tmp/a%00.png".to_vec(),
        "oversize" => vec![b'x'; 1024 * 1024 + 1],
        "empty" => vec![],
        _ => b"file:///tmp/a%20b.png\r\nfile:///tmp/%E2%9C%93.jpg\r\n".to_vec(),
    };
    let mut state = State {
        bytes,
        case: case.clone(),
        ..Default::default()
    };
    let start = std::time::Instant::now();
    loop {
        if let Some(stream) = socket.accept().unwrap() {
            display
                .handle()
                .insert_client(stream, Arc::new(ClientState))
                .unwrap();
        }
        display.dispatch_clients(&mut state).unwrap();
        display.flush_clients().unwrap();
        if let Some(status) = child.try_wait().unwrap() {
            assert!(status.success());
            if case == "overlap" {
                assert_eq!(state.finish_count, 2);
            }
            // Winit finishes a successful byte transfer before egui validates the paths.
            // Copy-only negotiation means finishing never authorizes source-file deletion.
            if matches!(case.as_str(), "valid" | "overlap") {
                assert!(state.finished);
            }
            if matches!(
                case.as_str(),
                "cancel" | "unsupported" | "oversize" | "stall" | "invalid_action"
            ) {
                assert!(!state.finished);
            }
            break;
        }
        if start.elapsed().as_secs() > 15 {
            let _ = child.kill();
            panic!("native protocol timed out");
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
}

struct ClientState;
impl wayland_server::backend::ClientData for ClientState {
    fn initialized(&self, _: wayland_server::backend::ClientId) {}
    fn disconnected(
        &self,
        _: wayland_server::backend::ClientId,
        _: wayland_server::backend::DisconnectReason,
    ) {
    }
}

fn hash(bytes: &[u8]) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut h);
    format!("{:x}", h.finish())
}

#[test]
fn native_file_drop_protocol() {
    for case in [
        "valid",
        "clipboard",
        "cancel",
        "unsupported",
        "remote",
        "nul",
        "oversize",
        "empty",
        "overlap",
        "invalid_action",
        "stall",
    ] {
        if std::env::var("ONLY_CASE").is_ok_and(|only| only != case) {
            continue;
        }
        run_case(case);
    }
}

/// Uses the real desktop portal with temporary files. Requires a running user session bus.
#[test]
#[ignore = "requires org.freedesktop.portal.Documents on the user session bus"]
fn native_file_drop_portal() {
    run_case("portal");
}
