//! The effectful half of the settings feature: shape B (feature 028, contract M2).
//!
//! `features/settings.rs` holds every settings transition, and all of them are pure. Three
//! additionally need something done at the I/O boundary — `settings.json` written, or the values
//! the shell owns read back into the freshly opened draft — and an `iced::Task` is what says so.
//! That is the line M2 draws: an arm belongs here when it must return a `Task`, and in the feature
//! otherwise.
//!
//! The bodies themselves did not move. They are still [`crate::shell::persist`]'s, called from
//! here rather than from a per-variant arm in `main.rs`; what changed is that `main.rs` now has
//! one settings arm instead of three, and the routing decision — which of them need an effect —
//! is stated once, in this file, next to the effects.

use iced::Task;
use micold_client::app::Message;
use micold_client::features::settings::Msg;

use crate::shell::persist;
use crate::App;

/// This feature's effectful entry point: one arm in `main.rs` routes here (contract M2).
///
/// The `pure` arm is not a fallback that might be wrong — it is every transition other than the
/// three named above, each of which finishes in the reducer. They reach `State::update` through the same wrapper variant they arrived
/// under, so the pure path is identical whether or not the shell was in the way.
pub fn update(app: &mut App, msg: Msg) -> Task<Message> {
    match msg {
        Msg::Opened => persist::on_settings_opened(app),
        Msg::Saved => persist::on_settings_saved(app),
        // The theme variant does not bind, so `msg` is still whole to re-wrap:
        // `on_theme_changed` applies it through the reducer and then persists.
        Msg::ThemePreferenceChanged(_) => persist::on_theme_changed(app, Message::Settings(msg)),
        pure => {
            app.core.update(Message::Settings(pure));
            Task::none()
        }
    }
}
