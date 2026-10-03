//! Bringing the window to the front (feature 039, research R7, contract N6).
//!
//! Which steps are taken is decided by `features::attention::raise_plan`; this module asks the
//! one fact the plan needs — whether the window is a Wayland surface — and turns each step into
//! its one `iced::window` task. It decides nothing.

use iced::window::{self, Id, UserAttention};
use iced::Task;
use micold_client::features::attention::{raise_plan, RaiseStep};

/// Bring the window to the front, by the steps of [`raise_plan`], in order.
pub fn raise<T: Send + 'static>() -> Task<T> {
    window::latest().and_then(|id| {
        on_wayland(id).then(move |wayland| {
            raise_plan(wayland)
                .into_iter()
                .fold(Task::none(), |done, next| done.chain(step(id, next)))
        })
    })
}

/// The one `iced::window` task of `step`.
fn step<T: Send + 'static>(id: Id, step: RaiseStep) -> Task<T> {
    match step {
        RaiseStep::Unminimize => window::minimize(id, false),
        RaiseStep::Focus => window::gain_focus(id),
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
