//! The Material top app bar, composed from reusable primitives (Principle VIII): the shared
//! [`toolbar`] with the application title and, on the right, a single overflow-menu trigger
//! (three dots). The menu's items float as an overlay (see
//! [`crate::ui::material::menu_overlay`], rendered in `ui::view`).
//!
//! # The menu offers actions, not settings (feature 027, FR-026e)
//!
//! It used to carry a theme-mode cycle and a "Keep sessions after logout" command, both of which
//! Settings also offers. Two controls for one setting is not a convenience; it is two writers, and
//! BUG-001 is what that cost — the app bar applied a theme immediately while the open Settings
//! form held a copy taken when it opened, so Save wrote the stale one back over the choice the
//! user had just watched take effect. That was unreachable while Settings was a modal covering the
//! app bar, and reachable the moment FR-026 made it a full-surface view.
//!
//! What is left here is the three things that are not settings: opening Settings, asking for
//! diagnostics, and About. Every one of them *does* something rather than storing a value.

use crate::app::{Message, State};
use crate::features::connection::Msg as ConnectionMsg;
use crate::features::help::help_actions;
use crate::features::help::Msg as HelpMsg;
use crate::features::project::Msg as ProjectMsg;
use crate::features::settings::Msg as SettingsMsg;
use crate::icons::Icon;
use crate::ui::material::{
    unread_total_tooltip, Button, MenuItem, MenuTrigger, Toolbar, UsageIndicator,
};
use iced::Element;
use micold_core::metadata::APP_NAME;
use micold_core::theme::ColorScheme;
use micold_core::tokens;

/// The overflow menu's items — actions only (FR-026e). Rendered as a floating overlay by
/// `ui::view` via `menu_overlay`.
///
/// Takes the state it no longer reads: the signature is the seam between the bar and its menu, and
/// the next item added here will want it. `_state` rather than dropping the parameter, so that
/// adding one is an edit to one line instead of to every call site.
pub fn overflow_items(_state: &State) -> Vec<MenuItem<Message>> {
    vec![
        MenuItem::new(
            Icon::Settings,
            "Settings",
            Message::Settings(SettingsMsg::Opened),
        ),
        MenuItem::new(
            Icon::Help,
            "Session service diagnostics",
            Message::Connection(ConnectionMsg::DiagnosticsRequested),
        ),
        MenuItem::new(
            Icon::About,
            help_actions()[0],
            Message::Help(HelpMsg::AboutOpened),
        ),
    ]
}

/// Render the top app bar: title (left), then on the trailing edge the project switcher
/// trigger immediately left of the overflow-menu trigger (feature 008, FR-004). Both panels
/// are floated as overlays by `ui::view`, so opening either never reflows the bar.
pub fn view<'a>(state: &State, scheme: ColorScheme) -> Element<'a, Message> {
    let r = tokens::roles(scheme);

    // The switcher names the project it will switch away from, so it is a **labelled** button, and
    // it is the shared one: `Button::text` with §7.3's leading-icon slot (018 FR-029c). It used to
    // assemble that shape itself — its own `button`, its own style, its own ripple, and
    // `row![icon(.., Action.size()), Text]` for the content — which is how it drew a 14dp glyph in
    // a 28dp box beside the overflow trigger's 24dp glyph in 48dp (BUG-007). Every figure it now
    // draws is §7.3's, because the component owns them: the 40dp height, the 12dp ends, the 18dp
    // glyph the leading slot applies whatever the label's role, and the press ripple.
    let switcher_label = state
        .workspace
        .active_project()
        .map(|p| p.display_name.clone())
        .unwrap_or_else(|| "Select project".to_string());
    let mut switcher = Button::text(switcher_label, r)
        .leading(Icon::OpenProject)
        .on_press(Message::Project(ProjectMsg::SwitcherToggled));
    // The unread sessions of the *other* projects, shown whether the panel is closed or open: the
    // sidebar already marks the active project's own (feature 039, FR-023; contract
    // `unread-mark.md` U7).
    let unread = state.other_projects_unread();
    if unread > 0 {
        switcher = switcher.trailing_mark(unread, unread_total_tooltip(unread));
    }
    let menu = MenuTrigger::new(Icon::Menu, Message::Help(HelpMsg::MenuToggled), r);
    let mut bar = Toolbar::new(APP_NAME, r)
        // Raised once the sidebar has content scrolled under it (FR-025a). The flag is derived from
        // the sidebar's offset rather than stored, so nothing else can set it.
        .elevated(state.app_bar_elevated());
    // Feature 490 (contract U3): the plan's usage leads the trailing actions, and only while there
    // is a current reading to show; without one the bar is exactly as before (FR-015).
    // The clock and zone are read only when a reading is held: most redraws have none.
    if state.plan_usage.reading.is_some() {
        let now = micold_core::unix_time::unix_secs() as i64;
        let offset = micold_core::local_time::UtcOffset::local_at(now);
        if let Some(usage) = usage_indicator(state, now, offset, &r) {
            bar = bar.action(usage);
        }
    }
    bar.action(switcher).action(menu).into()
}

/// The usage indicator for `now` in the zone `offset`, or `None` when nothing is shown (feature
/// 490, FR-004, FR-015).
fn usage_indicator<'a>(
    state: &State,
    now: i64,
    offset: micold_core::local_time::UtcOffset,
    roles: &micold_core::tokens::Roles,
) -> Option<Element<'a, Message>> {
    let view = state.plan_usage.view(now, offset)?;
    Some(
        UsageIndicator::new(view.label, roles)
            .warning(view.warning)
            .details(view.details)
            .into(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use micold_core::plan_usage::{LimitWindow, UsageReading, WarnPercent};

    const NOON: i64 = 1_791_720_000;

    fn utc() -> micold_core::local_time::UtcOffset {
        micold_core::local_time::UtcOffset::UTC
    }

    fn reading() -> UsageReading {
        UsageReading {
            windows: vec![LimitWindow {
                key: "five_hour".into(),
                used_tenths: 420,
                resets_at: Some(NOON + 3600),
            }],
            obtained_at: NOON,
        }
    }

    /// U3: the indicator is in the bar only with the switch on and a current reading.
    #[test]
    fn the_bar_carries_the_indicator_only_for_a_current_reading() {
        let r = tokens::roles(ColorScheme::Light);
        let mut state = State::default();
        assert!(
            usage_indicator(&state, NOON, utc(), &r).is_none(),
            "no reading"
        );

        state.plan_usage.received(Some(reading()));
        assert!(usage_indicator(&state, NOON, utc(), &r).is_some());
        assert!(
            usage_indicator(&state, NOON + 7200, utc(), &r).is_none(),
            "every window has reset"
        );

        state.plan_usage.settings(false, WarnPercent::default());
        assert!(
            usage_indicator(&state, NOON, utc(), &r).is_none(),
            "switch off"
        );
    }
}
