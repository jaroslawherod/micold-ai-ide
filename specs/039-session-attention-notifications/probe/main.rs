use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use iced::window;
use iced::{Subscription, Task};
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use wayland_client::backend::{Backend, ObjectId};
use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::{wl_registry, wl_surface::WlSurface};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols::xdg::activation::v1::client::xdg_activation_v1::{self, XdgActivationV1};

static ROLE: OnceLock<String> = OnceLock::new();
static MODE: OnceLock<String> = OnceLock::new();
static HANDLES: OnceLock<(usize, usize)> = OnceLock::new();

fn log(what: impl AsRef<str>) {
    let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    eprintln!(
        "{}.{:03} [{}] {}",
        t.as_secs(),
        t.subsec_millis(),
        ROLE.get().map(String::as_str).unwrap_or("?"),
        what.as_ref()
    );
}

struct Wl;
impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for Wl {
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
impl Dispatch<XdgActivationV1, ()> for Wl {
    fn event(
        _: &mut Self,
        _: &XdgActivationV1,
        _: xdg_activation_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

/// The thing under test: activate our surface over a second connection on the foreign display.
static DONE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn activate(token: &str) {
    if DONE.swap(true, std::sync::atomic::Ordering::SeqCst) {
        log("activate: skipped (already issued once)");
        return;
    }
    let Some(&(display, surface)) = HANDLES.get() else {
        log("activate: no handles");
        return;
    };
    let backend = unsafe { Backend::from_foreign_display(display as *mut _) };
    let conn = Connection::from_backend(backend);
    log("activate: from_foreign_display ok");
    let (globals, mut queue) = match registry_queue_init::<Wl>(&conn) {
        Ok(v) => v,
        Err(e) => {
            log(format!("activate: registry_queue_init FAILED: {e}"));
            return;
        }
    };
    let qh = queue.handle();
    let activation: XdgActivationV1 = match globals.bind(&qh, 1..=1, ()) {
        Ok(a) => a,
        Err(e) => {
            log(format!("activate: bind xdg_activation_v1 FAILED: {e}"));
            return;
        }
    };
    log("activate: xdg_activation_v1 bound");
    let id = match unsafe { ObjectId::from_ptr(WlSurface::interface(), surface as *mut _) } {
        Ok(id) => id,
        Err(e) => {
            log(format!("activate: surface ObjectId FAILED: {e}"));
            return;
        }
    };
    let surface = match WlSurface::from_id(&conn, id) {
        Ok(s) => s,
        Err(e) => {
            log(format!("activate: WlSurface::from_id FAILED: {e}"));
            return;
        }
    };
    activation.activate(token.to_owned(), &surface);
    log(format!("activate: activate({token:?}) sent"));
    match queue.roundtrip(&mut Wl) {
        Ok(_) => log("activate: roundtrip ok (no protocol error)"),
        Err(e) => log(format!("activate: roundtrip FAILED: {e}")),
    }
    log(format!("activate: protocol_error={:?}", conn.protocol_error()));
    activation.destroy();
    let _ = queue.roundtrip(&mut Wl);
}

fn dbus_thread() {
    let mode = MODE.get().unwrap().clone();
    let delay: u64 = std::env::var("T091_DELAY").ok().and_then(|v| v.parse().ok()).unwrap_or(8);
    let addr = std::env::var("DBUS_SESSION_BUS_ADDRESS").unwrap_or_default();
    log(format!("dbus: session bus {addr}"));
    assert!(addr.contains("t091-probe"), "refusing a bus that is not the private one");
    let conn = zbus::blocking::Connection::session().expect("session bus");
    let rule = zbus::MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .interface("org.freedesktop.Notifications")
        .unwrap()
        .build();
    let iter = zbus::blocking::MessageIterator::for_match_rule(rule, &conn, None).expect("match");
    std::thread::sleep(Duration::from_secs(delay));
    let hints: HashMap<&str, zbus::zvariant::Value<'_>> = HashMap::new();
    let reply = conn
        .call_method(
            Some("org.freedesktop.Notifications"),
            "/org/freedesktop/Notifications",
            Some("org.freedesktop.Notifications"),
            "Notify",
            &("t091-probe", 0u32, "", "T091 probe", "click me", vec!["default", "Open"], hints, 0i32),
        )
        .expect("Notify");
    let nid: u32 = reply.body().deserialize().expect("id");
    log(format!("dbus: Notify -> id {nid}"));
    let mut token: Option<String> = None;
    let mut invoked = false;
    for msg in iter {
        let Ok(msg) = msg else { continue };
        let header = msg.header();
        let member = header.member().map(|m| m.to_string()).unwrap_or_default();
        log(format!("dbus: signal {member} sender={:?} path={:?}", header.sender().map(|s| s.to_string()), header.path().map(|p| p.to_string())));
        match member.as_str() {
            "ActivationToken" => {
                let (id, tok): (u32, String) = msg.body().deserialize().expect("token body");
                log(format!("dbus: ActivationToken id={id} token={tok:?} (after ActionInvoked: {invoked})"));
                if id == nid {
                    token = Some(tok.clone());
                    if invoked && mode == "real" {
                        activate(&tok);
                    }
                }
            }
            "ActionInvoked" => {
                let (id, key): (u32, String) = msg.body().deserialize().expect("action body");
                log(format!("dbus: ActionInvoked id={id} key={key:?} token_seen={}", token.is_some()));
                if id != nid {
                    continue;
                }
                invoked = true;
                match mode.as_str() {
                    "real" => match &token {
                        Some(t) => activate(t),
                        None => log("real: no token before ActionInvoked; waiting for it"),
                    },
                    "fake" => activate("t091-made-up-token"),
                    _ => log("none: no activation issued"),
                }
            }
            other => {
                log(format!("dbus: signal {other} body_sig={:?}", msg.body().signature()));
            }
        }
    }
}

#[derive(Debug, Clone)]
enum Msg {
    Win(window::Id, window::Event),
    Handles(Option<(usize, usize)>),
}

#[derive(Default)]
struct St {
    asked: bool,
    focused: bool,
}

fn update(st: &mut St, msg: Msg) -> Task<Msg> {
    match msg {
        Msg::Win(id, ev) => {
            match &ev {
                window::Event::Focused => {
                    st.focused = true;
                    log("window: FOCUSED");
                }
                window::Event::Unfocused => {
                    st.focused = false;
                    log("window: UNFOCUSED");
                }
                window::Event::Opened { .. } => log("window: opened"),
                _ => {}
            }
            if !st.asked {
                st.asked = true;
                return window::run(id, |w| {
                    let d = w.display_handle().ok()?.as_raw();
                    let s = w.window_handle().ok()?.as_raw();
                    match (d, s) {
                        (RawDisplayHandle::Wayland(d), RawWindowHandle::Wayland(s)) => {
                            Some((d.display.as_ptr() as usize, s.surface.as_ptr() as usize))
                        }
                        _ => None,
                    }
                })
                .map(Msg::Handles);
            }
            Task::none()
        }
        Msg::Handles(h) => {
            match h {
                Some(h) => {
                    let _ = HANDLES.set(h);
                    log(format!("window: wayland handles display={:#x} surface={:#x}", h.0, h.1));
                }
                None => log("window: NOT a wayland window (no handles)"),
            }
            Task::none()
        }
    }
}

fn view(st: &St) -> iced::Element<'_, Msg> {
    iced::widget::text(format!(
        "T091 probe window {} focused={}",
        ROLE.get().unwrap(),
        st.focused
    ))
    .size(40)
    .into()
}

fn main() -> iced::Result {
    let mut args = std::env::args().skip(1);
    let role = args.next().expect("role A|B");
    let mode = args.next().unwrap_or_else(|| "none".into());
    ROLE.set(role.clone()).unwrap();
    MODE.set(mode.clone()).unwrap();
    log(format!(
        "start mode={mode} WAYLAND_DISPLAY={:?} DISPLAY={:?} XDG_RUNTIME_DIR={:?}",
        std::env::var("WAYLAND_DISPLAY").ok(),
        std::env::var("DISPLAY").ok(),
        std::env::var("XDG_RUNTIME_DIR").ok()
    ));
    assert_eq!(std::env::var("WAYLAND_DISPLAY").as_deref(), Ok("t091-wl"), "private display only");
    assert!(std::env::var("DISPLAY").is_err(), "DISPLAY must be unset");
    if role == "A" {
        std::thread::spawn(dbus_thread);
    }
    iced::application(St::default, update, view)
        .title(|_: &St| format!("T091-{}", ROLE.get().unwrap()))
        .subscription(|_| -> Subscription<Msg> { window::events().map(|(id, ev)| Msg::Win(id, ev)) })
        .window_size((520.0, 260.0))
        .run()
}
