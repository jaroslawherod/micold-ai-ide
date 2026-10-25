//! The title bar, composed from reusable primitives (Principle VIII; feature 1127): the shared
//! `Toolbar` with, from the leading end, the app mark, the `project / place` breadcrumb and the
//! Overview button, and at the trailing end a Settings button and the overflow-menu trigger (three dots).
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
    unread_total_tooltip, AppMark, Breadcrumb, Button, IconButton, MenuItem, MenuTrigger, Toolbar,
    Tooltip, TypeRole, UnreadMark,
};
use iced::widget::image;
use iced::Element;
use micold_core::metadata::APP_NAME;
use micold_core::session::SessionLocation;
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, anatomy};
use std::sync::LazyLock;

/// The application's icon as raw RGBA, [`APP_ICON_SIDE`] square (generated from
/// `assets/icon/icon.svg` by `assets/icon/generate.py`). Embedded directly so no runtime image
/// decoder is needed. The window's icon (`shell/startup.rs`) and the app mark both read it.
pub const APP_ICON_RGBA: &[u8] = include_bytes!("../../../../assets/icon/icon-64.rgba");

/// The side of [`APP_ICON_RGBA`], in pixels.
pub const APP_ICON_SIDE: u32 = 64;

// The bytes are a square of `APP_ICON_SIDE` RGBA pixels, or every reader decodes them wrongly.
const _: () = assert!(APP_ICON_RGBA.len() == (APP_ICON_SIDE * APP_ICON_SIDE * 4) as usize);

/// The app mark's picture, made once: one handle keeps one texture across frames, so the mark costs
/// no upload per frame (R9, `tests/idle_requests_no_frames.rs`).
static APP_MARK_HANDLE: LazyLock<image::Handle> = LazyLock::new(|| {
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

/// The app mark's picture: the one handle every caller shares (the bar and the showcase).
pub(crate) fn app_mark_handle() -> image::Handle {
    APP_MARK_HANDLE.clone()
}

/// The project segment's tooltip: what pressing it does (contract C2). The breadcrumb adds the full
/// project name on a second line once it shortens the segment.
pub const SWITCH_PROJECT: &str = "Switch project";

/// What the project segment reads with no project open (FR-004, US1 s4).
pub const SELECT_PROJECT: &str = "Select project";

/// One breadcrumb segment (data-model *Segment*).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    /// What the segment says at full width.
    pub text: String,
    /// The project segment's: what pressing it does. The place segment's: its full text, where it
    /// differs from `text` (a short branch's full branch). The bar adds the full `text` itself
    /// when it shortens the segment (FR-007).
    pub tooltip: Option<String>,
}

/// What the title bar's breadcrumb shows, derived from the state on every view (data-model
/// *TitleBarModel*; research R7). Render-free, so the rules are tested without a renderer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TitleBarModel {
    /// The active project's name, else `Select project` (FR-004).
    pub project: Segment,
    /// The other projects' unread sessions; a mark is drawn when above zero (feature 039).
    pub unread: usize,
    /// Whether the project switcher is open: the segment is drawn selected.
    pub switcher_open: bool,
    /// Where the main area is; `None` hides the segment and its `/` (FR-006).
    pub place: Option<Segment>,
}

/// The title bar's model (data-model *TitleBarModel*, *Place*: first match wins).
pub fn model(state: &State) -> TitleBarModel {
    let project = state.workspace.active_project();
    TitleBarModel {
        project: Segment {
            text: project.map_or_else(|| SELECT_PROJECT.to_string(), |p| p.display_name.clone()),
            tooltip: Some(SWITCH_PROJECT.to_string()),
        },
        unread: state.other_projects_unread(),
        switcher_open: state.project.switcher_open,
        place: project.and_then(|_| place(state)),
    }
}

/// The place segment inside an active project (FR-006): Settings, then the Overview, then the
/// location the main area shows — the shown review's, else the current session's, which Compare
/// keeps (R7).
fn place(state: &State) -> Option<Segment> {
    let named = |text: &str| Segment {
        text: text.to_string(),
        tooltip: None,
    };
    if state.settings.settings_draft.is_some() {
        return Some(named("Settings"));
    }
    if state.overview.open {
        return Some(named("Overview"));
    }
    location_segment(state, &state.shown_location()?)
}

/// A location's segment, from its Overview card so the bar and the session area's header agree
/// (SC-005): the short branch with the full branch on hover, else the display name. `None` once
/// the location no longer resolves to a card.
fn location_segment(state: &State, location: &SessionLocation) -> Option<Segment> {
    let card = crate::ui::overview::location_card(state, location)?;
    Some(match card.branch.as_deref() {
        Some(branch) => {
            let chip = micold_core::context_header::branch_chip(branch);
            Segment {
                tooltip: (chip.tooltip != chip.text).then_some(chip.tooltip),
                text: chip.text,
            }
        }
        None => Segment {
            text: card.display_name,
            tooltip: None,
        },
    })
}

/// Render the title bar (feature 1127, contract B2): the app mark, the breadcrumb, the Overview
/// button, then Settings and the overflow trigger at the trailing end. Every panel floats as an
/// overlay from `ui::view`, so opening one never reflows the bar.
pub fn view<'a>(state: &State, scheme: ColorScheme) -> Element<'a, Message> {
    let r = tokens::roles(scheme);
    let model = model(state);

    // The project segment names the project it will switch away from, and the place says where
    // the main area is (FR-004, FR-006). It is the bar's child that gives way when the window is
    // narrow, place first (contract B4, FR-007). The unread sessions of the *other* projects are
    // shown whether the panel is closed or open: the sidebar already marks the active project's own
    // (feature 039, FR-023; contract `unread-mark.md` U7).
    let mut crumb = Breadcrumb::new(model.project.text, r)
        .on_press(Message::Project(ProjectMsg::SwitcherToggled))
        .trailing_mark(model.unread, unread_total_tooltip(model.unread))
        .open(model.switcher_open);
    if let Some(tooltip) = model.project.tooltip {
        crumb = crumb.tooltip(tooltip);
    }
    if let Some(place) = model.place {
        crumb = crumb.place(place.text, place.tooltip);
    }
    // Where the panel hangs from: the segment's own bottom-left corner, reported on the press
    // ahead of the toggle (BUG-1023). Only its `x` is kept: the panel's top is the bar's bottom
    // edge, and a keyboard press, which reports none, hangs it from the segment's computed edge
    // (R8, `ui::switcher_hung_from`).
    let switcher = ContextArea::new(crumb)
        .on_primary_bounds(|corner| Message::Project(ProjectMsg::SwitcherAnchored(corner)));

    let settings_open = state.settings.settings_draft.is_some();
    let settings = IconButton::new(Icon::Settings, r)
        .target(anatomy::app_bar::ICON_TARGET)
        .selected(settings_open)
        .keyboard(true)
        .on_press_maybe(settings_press(state));
    let menu = MenuTrigger::new(Icon::Menu, Message::Help(HelpMsg::MenuToggled), r).tooltip("More");

    Toolbar::new(r)
        .mark(AppMark::new(app_mark_handle(), APP_NAME, r))
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
    use micold_core::project::{Availability, Project};
    use micold_core::session::{AiCli, Session};
    use micold_core::worktree::{Worktree, WorktreeStatus};
    use std::path::PathBuf;

    const REPO: &str = "/repo/micold-ai-ide";

    fn worktree(dir: &str, branch: Option<&str>) -> Worktree {
        Worktree {
            dir_name: dir.into(),
            path: format!("{REPO}/.claude/worktrees/{dir}").into(),
            branch: branch.map(Into::into),
            status: WorktreeStatus::Valid,
            included: false,
        }
    }

    /// `micold-ai-ide` open with the worktrees `fix-1117-x` on `fix/1117-x` and `feat-2000` on
    /// `feat/2000-y`, and no session shown.
    fn project() -> State {
        let mut state = State::default();
        let path = PathBuf::from(REPO);
        state
            .workspace
            .projects
            .push(Project::new(path.clone(), true, Availability::Available));
        state.workspace.active = Some(path);
        state.worktree.worktrees = vec![
            worktree("fix-1117-x", Some("fix/1117-x")),
            worktree("feat-2000", Some("feat/2000-y")),
        ];
        state
    }

    /// [`project`] with a session at `location`, shown.
    fn showing(location: SessionLocation) -> State {
        let mut state = project();
        let session = Session::start_new(location, AiCli::ClaudeCode);
        state.session.active = Some(session.id);
        state
            .workspace
            .sessions
            .insert(PathBuf::from(REPO), vec![session]);
        state
    }

    fn on_fix_1117() -> State {
        showing(SessionLocation::Worktree("fix-1117-x".into()))
    }

    fn place_text(state: &State) -> Option<String> {
        model(state).place.map(|p| p.text)
    }

    /// T025 (FR-004, C2): the project segment reads the project's name, its tooltip says what it
    /// does; with no project it reads `Select project` and has no place.
    #[test]
    fn the_project_segment_reads_the_project_or_select_project() {
        let m = model(&on_fix_1117());
        assert_eq!(m.project.text, "micold-ai-ide");
        assert_eq!(m.project.tooltip.as_deref(), Some(SWITCH_PROJECT));
        let mut none = on_fix_1117();
        none.workspace.active = None;
        let m = model(&none);
        assert_eq!(m.project.text, SELECT_PROJECT);
        assert_eq!(m.project.tooltip.as_deref(), Some(SWITCH_PROJECT));
        assert_eq!(m.place, None, "no project, no place (US1 s4)");
        none.settings.settings_draft = Some(Default::default());
        assert_eq!(
            model(&none).place,
            None,
            "Settings without a project: still none"
        );
    }

    /// T025 (FR-006, SC-005): a session on `fix/1117-x` → the short branch, the full branch on
    /// hover; no location shown → no place (US1 s5).
    #[test]
    fn a_session_on_a_branch_reads_its_short_branch() {
        let place = model(&on_fix_1117()).place.expect("a place");
        assert_eq!(place.text, "fix/1117");
        assert_eq!(place.tooltip.as_deref(), Some("fix/1117-x"));
        assert_eq!(model(&project()).place, None, "no location selected");
    }

    /// T025: the Default location, which has no branch, reads its display name.
    #[test]
    fn the_default_location_reads_its_display_name() {
        let place = model(&showing(SessionLocation::Default))
            .place
            .expect("a place");
        assert_eq!(place.text, "Default");
        assert_eq!(place.tooltip, None);
    }

    /// T025 (US1 s3): the Overview, normal and full screen, and Settings name themselves; Settings
    /// wins over the Overview.
    #[test]
    fn the_overview_and_settings_name_themselves() {
        let mut state = on_fix_1117();
        state.overview.open = true;
        assert_eq!(place_text(&state).as_deref(), Some("Overview"));
        state.overview.full_screen = true;
        assert_eq!(place_text(&state).as_deref(), Some("Overview"));
        state.settings.settings_draft = Some(Default::default());
        assert_eq!(place_text(&state).as_deref(), Some("Settings"));
    }

    /// T025: a shown review reads its entry's location; Compare keeps the session's.
    #[test]
    fn changes_reads_its_entry_and_compare_keeps_the_session() {
        let mut state = on_fix_1117();
        let _ = state.update_changes(crate::features::changes::Msg::Opened {
            project: PathBuf::from(REPO),
            entry: SessionLocation::Worktree("feat-2000".into()),
        });
        assert_eq!(place_text(&state).as_deref(), Some("feat/2000"));

        let mut state = on_fix_1117();
        state.runs.compare = Some(crate::features::runs::CompareView {
            group: micold_core::runs::GroupId(uuid::Uuid::from_u128(7)),
            project: PathBuf::from(REPO),
            reads: Default::default(),
        });
        assert_eq!(place_text(&state).as_deref(), Some("fix/1117"));
    }

    /// T025 (Principle II edge case): a location removed from the state takes the place with it on
    /// the same update.
    #[test]
    fn a_removed_location_has_no_place() {
        let mut state = on_fix_1117();
        state
            .worktree
            .worktrees
            .retain(|w| w.dir_name != "fix-1117-x");
        assert_eq!(model(&state).place, None);
    }

    /// T025 (FR-004): `unread` counts the other projects' unread sessions; `switcher_open` follows
    /// the switcher.
    #[test]
    fn unread_and_switcher_open_follow_the_state() {
        let mut state = on_fix_1117();
        assert_eq!(model(&state).unread, 0);
        assert!(!model(&state).switcher_open);
        let other = PathBuf::from("/repo/other");
        state
            .workspace
            .projects
            .push(Project::new(other.clone(), true, Availability::Available));
        let mut unread = Session::start_new(SessionLocation::Default, AiCli::ClaudeCode);
        unread.unread = true;
        state.workspace.sessions.insert(other, vec![unread]);
        state.project.switcher_open = true;
        let m = model(&state);
        assert_eq!(m.unread, state.other_projects_unread());
        assert_eq!(m.unread, 1);
        assert!(m.switcher_open);
    }

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
