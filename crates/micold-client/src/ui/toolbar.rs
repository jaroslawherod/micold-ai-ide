//! The title bar, composed from reusable primitives (Principle VIII; feature 1127): the shared
//! `Toolbar` with, from the leading end, the app mark, the project switcher and the Overview
//! button, and at the trailing end a Settings button and the overflow-menu trigger (three dots).
//! No title text and no usage (FR-002). The menu's items float as an overlay (see
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
use crate::features::overview::{self, Msg as OverviewMsg, Tone};
use crate::features::project::Msg as ProjectMsg;
use crate::features::settings::Msg as SettingsMsg;
use crate::icons::Icon;
use crate::ui::cdk::context_area::ContextArea;
use crate::ui::material::{
    unread_total_tooltip, AppMark, Button, ButtonVariant, Ellipsized, IconButton, MenuItem,
    MenuTrigger, Toolbar, Tooltip, TypeRole, UnreadMark,
};
use iced::widget::image;
use iced::Element;
use micold_core::metadata::APP_NAME;
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, anatomy};
use std::sync::LazyLock;

/// The application's icon as raw RGBA, [`APP_ICON_SIDE`] square (generated from
/// `assets/icon/icon.svg` by `assets/icon/generate.py`). Embedded directly so no runtime image
/// decoder is needed. The window's icon (`shell/startup.rs`) and the app mark both read it.
pub const APP_ICON_RGBA: &[u8] = include_bytes!("../../../../assets/icon/icon-64.rgba");

/// The side of [`APP_ICON_RGBA`], in pixels.
pub const APP_ICON_SIDE: u32 = 64;

/// The app mark's picture, made once: one handle keeps one texture across frames, so the mark costs
/// no upload per frame (R9, `tests/idle_requests_no_frames.rs`).
static APP_MARK: LazyLock<image::Handle> = LazyLock::new(|| {
    image::Handle::from_rgba(APP_ICON_SIDE, APP_ICON_SIDE, APP_ICON_RGBA.to_vec())
});

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

/// Render the title bar (feature 1127, contract B2): the app mark, the project switcher, the
/// Overview button, then Settings and the overflow trigger at the trailing end. Every panel floats
/// as an overlay from `ui::view`, so opening one never reflows the bar.
pub fn view<'a>(state: &State, scheme: ColorScheme) -> Element<'a, Message> {
    let r = tokens::roles(scheme);

    // The switcher names the project it will switch away from, so it is a **labelled** button, and
    // it is the shared one: §7.3's 40dp height, 12dp ends, 18dp leading glyph and press ripple are
    // the component's (BUG-007). Its label is ellipsized, since it is the bar's child that gives
    // way when the window is narrow (contract B4): a wrapped name would spill out of the bar.
    let switcher_label = state
        .workspace
        .active_project()
        .map(|p| p.display_name.clone())
        .unwrap_or_else(|| "Select project".to_string());
    let label = Ellipsized::at_role(
        switcher_label,
        TypeRole::Action,
        ButtonVariant::Text.content(r, None),
    )
    .shrink();
    let mut switcher = Button::with_content(label, ButtonVariant::Text, r)
        .leading(Icon::OpenProject)
        .on_press(Message::Project(ProjectMsg::SwitcherToggled));
    // The unread sessions of the *other* projects, shown whether the panel is closed or open: the
    // sidebar already marks the active project's own (feature 039, FR-023; contract
    // `unread-mark.md` U7).
    let unread = state.other_projects_unread();
    if unread > 0 {
        switcher = switcher.trailing_mark(unread, unread_total_tooltip(unread));
    }
    // Where the panel hangs from: the trigger's own bottom-left corner, reported on the press
    // ahead of the toggle (BUG-1023). Only its `x` is kept: the panel's top is the bar's bottom
    // edge (R8).
    let switcher = ContextArea::new(switcher)
        .on_primary_bounds(|corner| Message::Project(ProjectMsg::SwitcherAnchored(corner)));

    let settings_open = state.settings.settings_draft.is_some();
    let settings = IconButton::new(Icon::Settings, r)
        .target(anatomy::app_bar::ICON_TARGET)
        .selected(settings_open)
        .keyboard(true)
        .on_press_maybe(settings_press(state));
    let menu = MenuTrigger::new(Icon::Menu, Message::Help(HelpMsg::MenuToggled), r).tooltip("More");

    Toolbar::new(r)
        .mark(AppMark::new(APP_MARK.clone(), APP_NAME, r))
        .leading(switcher)
        .leading_fixed(overview_button(state, r))
        .action(Tooltip::new(settings, "Settings", r))
        .action(menu)
        // Raised once the sidebar has content scrolled under it (FR-025a). The flag is derived from
        // the sidebar's offset rather than stored, so nothing else can set it.
        .elevated(state.app_bar_elevated())
        .into()
}

/// What the Settings button sends (FR-011, R10): `Opened` while Settings is closed, nothing while
/// it is open — re-sending `Opened` is not a no-op, since it clears the dialogs and reports
/// `SettingsOpened` again. The button stays drawn selected while open.
pub(crate) fn settings_press(state: &State) -> Option<Message> {
    state
        .settings
        .settings_draft
        .is_none()
        .then_some(Message::Settings(SettingsMsg::Opened))
}

/// The Overview button (feature 813, contract U2): after the project switcher, shown whether or not
/// the Overview is open, and inert while Settings is open (research R2).
fn overview_button<'a>(state: &State, r: tokens::Roles) -> Element<'a, Message> {
    let mut button = Button::text("Overview", r)
        .leading(Icon::Overview)
        .selected(state.overview.open)
        .on_press_maybe(overview_press(state));
    let badge = overview::badge(&crate::ui::overview::cards(state));
    if let Some(badge) = &badge {
        let (glyph, tint) = badge_look(badge.tone, &r);
        button = button.mark(
            UnreadMark::new(r)
                .icon(glyph)
                .count(badge.count)
                .role(TypeRole::Action)
                .tint(tint),
        );
    }
    Tooltip::new(button, overview::overview_tooltip(badge.as_ref()), r).into()
}

/// The glyph and tint the Overview badge is drawn in: action is the `error` family, attention the
/// `warning` family.
fn badge_look(tone: Tone, r: &tokens::Roles) -> (Icon, tokens::Rgb) {
    match tone {
        Tone::Error => (Icon::CardNeedsAction, r.error),
        Tone::Warning => (Icon::CardNeedsAttention, r.warning),
    }
}

/// What the Overview button sends: nothing while Settings is open or no project is active, since
/// the Overview only draws inside an active project.
fn overview_press(state: &State) -> Option<Message> {
    let settings_open = state.settings.settings_draft.is_some();
    (!settings_open && state.workspace.active.is_some())
        .then_some(Message::Overview(OverviewMsg::Toggled { settings_open }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// T010 (FR-011, R10): Settings closed → the button opens it; open → it sends nothing.
    #[test]
    fn the_settings_button_opens_settings_only_while_it_is_closed() {
        let mut state = State::default();
        assert!(matches!(
            settings_press(&state),
            Some(Message::Settings(SettingsMsg::Opened))
        ));
        state.settings.settings_draft = Some(Default::default());
        assert!(settings_press(&state).is_none(), "settings open");
    }

    /// The badge is red with the action glyph for an error, yellow with the attention glyph for a
    /// warning, and the tooltip carries the chord and the badge's words.
    #[test]
    fn the_overview_badge_maps_tone_to_glyph_and_tint_and_the_tooltip_names_the_chord() {
        let r = tokens::roles(ColorScheme::Light);
        assert_eq!(
            badge_look(Tone::Error, &r),
            (Icon::CardNeedsAction, r.error)
        );
        assert_eq!(
            badge_look(Tone::Warning, &r),
            (Icon::CardNeedsAttention, r.warning)
        );
        let plain = overview::overview_tooltip(None);
        assert!(plain.contains(crate::keymap::overview_chord_label()));
        let badge = overview::Badge {
            tone: Tone::Error,
            count: 2,
            label: "2 need action".into(),
        };
        let with = overview::overview_tooltip(Some(&badge));
        assert!(with.starts_with(&plain) && with.ends_with("2 need action"));
    }

    /// F3: the Overview button is inert with no active project, and while Settings is open.
    #[test]
    fn the_overview_button_needs_an_active_project_and_no_settings() {
        let mut state = State::default();
        assert!(overview_press(&state).is_none(), "no project");
        state.workspace.active = Some(std::path::PathBuf::from("/repo"));
        assert!(overview_press(&state).is_some());
        state.settings.settings_draft = Some(Default::default());
        assert!(overview_press(&state).is_none(), "settings open");
    }

    /// Finding 12 (FR-003): full screen leaves the top bar, and its Overview button, in place.
    #[test]
    fn the_overview_button_is_still_built_in_full_screen() {
        let mut state = State::default();
        state.workspace.active = Some(std::path::PathBuf::from("/repo"));
        state.overview.open = true;
        state.overview.full_screen = true;
        assert!(state.overview.full_screen());
        assert!(overview_press(&state).is_some(), "button still pressable");
        let _bar = view(&state, ColorScheme::Light);
    }
}
