//! Bringing the window to the front (feature 039, research R7, contract N6).
//!
//! Which steps are taken is decided by `features::attention`: `raise_plan` for the plan,
//! `after_send` for what follows the attempt to send a Wayland activation request, and
//! `after_activation` for what follows once the window's focus is known. This module asks the
//! facts those need — whether the window is a Wayland surface, whether the request went out — and
//! turns each step into its one `iced::window` task. It decides nothing.
//!
//! A compositor says nothing about an activation token it declines (research R7). So a request
//! that went out is followed, `ACTIVATION_SETTLE` later, by `ConnectionMsg::ActivationSettled`;
//! the shell answers it with [`settled`] and the window's keyboard focus at that moment.

use iced::window::{self, Id, UserAttention};
use iced::Task;
use micold_client::app::Message;
use micold_client::features::attention::{
    after_activation, after_send, raise_plan, AfterSend, RaiseStep,
};
use micold_client::features::connection::Msg as ConnectionMsg;

/// Bring the window to the front, by the steps of [`raise_plan`], in order. `activation` is the
/// Wayland activation token of the click that asked for it, when there was one.
pub fn raise(activation: Option<String>) -> Task<Message> {
    window::latest().and_then(move |id| {
        let activation = activation.clone();
        on_wayland(id).then(move |wayland| {
            raise_plan(wayland, activation.clone())
                .into_iter()
                .fold(Task::none(), |done, next| done.chain(step(id, next)))
        })
    })
}

/// The wait after an activation request is over: `focused` is whether the window has keyboard
/// focus now. Takes the step [`after_activation`] gives for it, if any.
pub fn settled(focused: bool) -> Task<Message> {
    match after_activation(focused) {
        Some(next) => window::latest().and_then(move |id| step(id, next.clone())),
        None => Task::none(),
    }
}

/// The task of `step`.
fn step(id: Id, step: RaiseStep) -> Task<Message> {
    match step {
        RaiseStep::Unminimize => window::minimize(id, false),
        RaiseStep::Focus => window::gain_focus(id),
        RaiseStep::Activate(token) => {
            send_activation(id, token).then(move |sent| match after_send(sent) {
                AfterSend::CheckFocusAfter(wait) => {
                    Task::perform(async move { tokio::time::sleep(wait).await }, |()| {
                        Message::Connection(ConnectionMsg::ActivationSettled)
                    })
                }
                AfterSend::Now(Some(next)) => self::step(id, next),
                AfterSend::Now(None) => Task::none(),
            })
        }
        RaiseStep::RequestAttention => {
            window::request_user_attention(id, Some(UserAttention::Informational))
        }
    }
}

/// Whether the window is a Wayland surface, which cannot take keyboard focus itself.
#[cfg(target_os = "linux")]
fn on_wayland(id: Id) -> Task<bool> {
    use iced::window::raw_window_handle::RawWindowHandle;
    window::run(id, |window| {
        window
            .window_handle()
            .is_ok_and(|handle| matches!(handle.as_raw(), RawWindowHandle::Wayland(_)))
    })
}

/// Whether the window is a Wayland surface: never, off Linux.
#[cfg(not(target_os = "linux"))]
fn on_wayland(_id: Id) -> Task<bool> {
    Task::done(false)
}

/// Ask the compositor to give the window keyboard focus for `token`. Answers whether the request
/// went out; whether the compositor honoured it is not known here (see the module's comment).
#[cfg(target_os = "linux")]
fn send_activation(id: Id, token: String) -> Task<bool> {
    window::run(id, move |window| match wayland::activate(window, &token) {
        Ok(()) => true,
        Err(why) => {
            crate::log_line(&format!(
                "window not activated ({why}); asking for attention instead"
            ));
            false
        }
    })
}

/// Off Linux there is no Wayland surface to activate: the request never goes out.
#[cfg(not(target_os = "linux"))]
fn send_activation(_id: Id, _token: String) -> Task<bool> {
    Task::done(false)
}

/// The `xdg_activation_v1` request, over the application's own Wayland connection (research R7,
/// probed in T091).
#[cfg(target_os = "linux")]
mod wayland {
    use iced::window::raw_window_handle::{RawDisplayHandle, RawWindowHandle};
    use iced::window::Window;
    use wayland_client::backend::{Backend, ObjectId};
    use wayland_client::globals::{registry_queue_init, GlobalListContents};
    use wayland_client::protocol::{wl_registry, wl_surface::WlSurface};
    use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
    use wayland_protocols::xdg::activation::v1::client::xdg_activation_v1::{
        self, XdgActivationV1,
    };

    /// The state of the event queue this call makes: neither object it holds has an event this
    /// window reads.
    struct Queue;

    impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for Queue {
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

    impl Dispatch<XdgActivationV1, ()> for Queue {
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

    /// Send `xdg_activation_v1.activate(token, surface)` for `window`'s surface and wait for the
    /// compositor to have read it. `Err` says why the request did not go out: the window is not a
    /// Wayland surface, the compositor has no `xdg_activation_v1`, or a call failed.
    ///
    /// Must be called with the live window, as `iced::window::run` does: its handles are used
    /// here and not kept.
    pub(super) fn activate(window: &dyn Window, token: &str) -> Result<(), String> {
        let display = window.display_handle().map_err(|e| e.to_string())?;
        let surface = window.window_handle().map_err(|e| e.to_string())?;
        let (RawDisplayHandle::Wayland(display), RawWindowHandle::Wayland(surface)) =
            (display.as_raw(), surface.as_raw())
        else {
            return Err("not a Wayland window".to_string());
        };
        // SAFETY: `display` is the `wl_display` and `surface` the `wl_surface` proxy of `window`,
        // which winit made with libwayland and keeps alive for as long as the window lives; the
        // window is borrowed for this whole call, and neither pointer, nor anything made from
        // one, outlives it. `from_foreign_display` does not take ownership: dropping the backend
        // leaves the application's connection open. `from_ptr` checks that the proxy is a
        // `wl_surface` before it is used as one.
        let (backend, surface) = unsafe {
            (
                Backend::from_foreign_display(display.display.as_ptr().cast()),
                ObjectId::from_ptr(WlSurface::interface(), surface.surface.as_ptr().cast()),
            )
        };
        let connection = Connection::from_backend(backend);
        let surface = surface
            .and_then(|id| WlSurface::from_id(&connection, id))
            .map_err(|e| format!("the window's surface: {e}"))?;
        // A queue of this call's own: the application's events stay on the application's queue.
        let (globals, mut queue) =
            registry_queue_init::<Queue>(&connection).map_err(|e| format!("the registry: {e}"))?;
        let activation: XdgActivationV1 = globals
            .bind(&queue.handle(), 1..=1, ())
            .map_err(|e| format!("xdg_activation_v1: {e}"))?;
        activation.activate(token.to_owned(), &surface);
        let sent = queue.roundtrip(&mut Queue);
        activation.destroy();
        let _ = connection.flush();
        sent.map(|_| ()).map_err(|e| format!("the request: {e}"))
    }
}
