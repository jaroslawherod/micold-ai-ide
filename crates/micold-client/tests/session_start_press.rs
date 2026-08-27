//! Pressing start when the stored default is not installed offers the CLIs that are (feature 026,
//! T085 — FR-004 scenario 4, Clarifications 2026-08-16).
//!
//! `State::start_intent` decided this in T032a and `features_session.rs` has covered the decision
//! since; what was missing is the dispatch. The sidebar's primary half published
//! `SessionStartRequested` on every press, resolved by `provider_for_start(None)` — so an
//! uninstalled stored default was sent to the daemon as a spawn of a binary that is not there, and
//! the user got FR-010's failure on a row that was never going to run, where the clarification says
//! to tell them and offer what is available *before* anything is created.
//!
//! So this is a view-level test, and it has to be: the pure branch is already green either way, and
//! the defect lives entirely in which message a press publishes. It presses the real `+` — found by
//! its own glyph rather than by a hardcoded node path, so a sidebar that is rearranged makes this
//! test say so instead of quietly pressing something else — and reads what comes back.
//!
//! Both directions, because "no `SessionStartRequested`" is also what a press that reached nothing
//! looks like: with the default installed the press still starts it in one interaction (SC-001),
//! and with the default missing the same press opens the list and starts nothing.

#[path = "support/mod.rs"]
mod support;

use iced::advanced::widget::Tree;
use iced::advanced::{clipboard, layout, mouse, Layout, Shell};
use iced::{Element, Event, Point, Rectangle, Size};
use micold_client::app::{Message, State};
use micold_client::features::connection::ConnectionStatus;
use micold_client::features::session::Msg as SessionMsg;
use micold_client::icons::Icon;
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::session::{AiCli, SessionLocation};
use support::layout as lay;

const PROJECT: &str = "/fixture/start-press";

/// A project open with no worktrees, so the only start affordance on screen is the Default row's.
fn with_project(default_ai_cli: AiCli, available: &[AiCli]) -> State {
    let mut workspace = support::workspace_with(vec![(PROJECT, vec![])]);
    workspace.active = workspace.projects.first().map(|p| p.path.clone());
    let mut state = State {
        workspace,
        ..State::default()
    };
    state.sidebar.width = 300;
    state.window.window_size = (lay::WINDOW.width as u16, lay::WINDOW.height as u16);
    state.session.default_ai_cli = default_ai_cli;
    state.session.available_providers = available.to_vec();
    state
}

fn view(state: &State) -> Element<'_, Message> {
    micold_client::ui::view(
        state,
        None,
        None,
        0,
        None,
        &EnvIncludeOutcome::Disabled,
        &ConnectionStatus::Connected,
        &micold_client::features::sandbox::Sandbox::default(),
    )
}

/// Where the start affordance's `+` is drawn, in window pixels.
///
/// Located by the glyph the button paints rather than by a node path, so a sidebar that is
/// rearranged makes this test say so instead of quietly pressing something else. The point is the
/// paragraph's own draw origin: `node_path` attributes text by clip, so a glyph inside the
/// sidebar's scrollable attributes to the scrollable — pressing that node's centre presses empty
/// list, which is exactly the "nothing was published" this test's negative assertion looks for.
///
/// Asserting there is exactly one is half the point — a second `+` on screen would mean this test
/// is pressing a worktree row's affordance and reporting it as the Default row's.
fn start_glyph(state: &State) -> Point {
    let mut renderer = lay::renderer();
    let glyph = Icon::AddSession.glyph().to_string();
    let painted = lay::painted_text_settled(view(state), &mut renderer);
    let drawn: Vec<&lay::Overflow> = painted.iter().filter(|t| t.content == glyph).collect();
    assert_eq!(
        drawn.len(),
        1,
        "exactly one start affordance is expected on screen; painted: {:?}",
        painted.iter().map(|t| &t.content).collect::<Vec<_>>()
    );
    drawn[0].origin
}

/// Left-press and release at `at`, and return everything the view published.
///
/// The frames before the press are not optional: a panel mounts at opacity zero and `Fade` declines
/// every non-`Window` event below `HIDDEN`, so a press into a freshly built tree is swallowed and
/// the test reads "nothing was published" about a control that answers perfectly well. The count is
/// `support::layout`'s, for the same reason it is there.
fn press_at(state: &State, at: Point) -> Vec<Message> {
    let renderer = lay::renderer();
    let mut element = view(state);
    let mut tree = Tree::new(element.as_widget());
    let limits = layout::Limits::new(Size::ZERO, lay::WINDOW);
    let node = element
        .as_widget_mut()
        .layout(&mut tree, &renderer, &limits);

    const SETTLE_FRAMES: u32 = 8;
    let origin = std::time::Instant::now();
    let mut settling: Vec<Message> = Vec::new();
    for frame in 0..SETTLE_FRAMES {
        let mut shell = Shell::new(&mut settling);
        element.as_widget_mut().update(
            &mut tree,
            &Event::Window(iced::window::Event::RedrawRequested(
                origin + lay::FRAME * frame,
            )),
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &renderer,
            &mut clipboard::Null,
            &mut shell,
            &Rectangle::with_size(lay::WINDOW),
        );
    }

    // A full click: the library's button claims the press and publishes on the release.
    let mut messages: Vec<Message> = Vec::new();
    for event in [
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
    ] {
        let mut shell = Shell::new(&mut messages);
        element.as_widget_mut().update(
            &mut tree,
            &event,
            Layout::new(&node),
            mouse::Cursor::Available(at),
            &renderer,
            &mut clipboard::Null,
            &mut shell,
            &Rectangle::with_size(lay::WINDOW),
        );
    }
    messages
}

/// What the `+` publishes when pressed, for a state.
fn press_start(state: &State) -> Vec<Message> {
    let at = start_glyph(state);
    press_at(state, at)
}

#[test]
fn pressing_start_with_an_uninstalled_default_offers_the_choice_and_starts_nothing() {
    // The stored default is Copilot; only Claude Code is installed. FR-002 keeps the stored value
    // as the user left it, so this is a state the application really sits in.
    let state = with_project(AiCli::Copilot, &[AiCli::ClaudeCode]);

    let published = press_start(&state);

    assert!(
        !published
            .iter()
            .any(|m| matches!(m, Message::Session(SessionMsg::StartRequested { .. }))),
        "nothing may be started on a CLI that is not installed — not the missing default, and not \
         a silent substitution of the one that is (FR-002/FR-004). Published: {published:?}"
    );
    assert!(
        published.iter().any(|m| matches!(
            m,
            Message::Session(SessionMsg::StartMenuOpened(SessionLocation::Default))
        )),
        "and the press has to *do* something: the available CLIs are offered at that moment, which \
         is the same list the chevron opens. Published: {published:?}"
    );
    assert!(
        published
            .iter()
            .any(|m| matches!(m, Message::Session(SessionMsg::StartMenuAnchored(_)))),
        "with a point to hang the list from, since the primary half can now open one and a sidebar \
         row's position is not something the view holds (018 BUG-008). Published: {published:?}"
    );
}

#[test]
fn pressing_start_with_the_default_installed_still_starts_it_in_one_interaction() {
    // The other direction, and the one SC-001 is about: the majority case must be untouched.
    let state = with_project(AiCli::Copilot, &[AiCli::ClaudeCode, AiCli::Copilot]);

    let published = press_start(&state);

    assert!(
        published.iter().any(|m| matches!(
            m,
            Message::Session(SessionMsg::StartRequested {
                location: SessionLocation::Default,
                provider: AiCli::Copilot,
            })
        )),
        "one press, the stored default, started (SC-001). Published: {published:?}"
    );
    assert!(
        !published
            .iter()
            .any(|m| matches!(m, Message::Session(SessionMsg::StartMenuOpened(_)))),
        "and no list in the way of it. Published: {published:?}"
    );
}

#[test]
fn the_choice_is_offered_from_the_availability_set_the_press_can_still_refresh() {
    // Why the offer arrives as `SessionStartMenuOpened` rather than as a message of its own: that
    // message is one of research R11's two named events, and the binary re-probes `PATH` on it
    // before the reducer opens the list. A separate message would open the list on the set as it
    // was at the last of those events, which for a user who has installed nothing since launch is
    // the set from launch.
    //
    // The binary's handler is not reachable from here; what is, and what keeps the two joined, is
    // that the press publishes exactly the message that handler is written against — the same one
    // the chevron publishes.
    let state = with_project(AiCli::Copilot, &[AiCli::ClaudeCode]);

    let published = press_start(&state);
    let opened: Vec<&Message> = published
        .iter()
        .filter(|m| matches!(m, Message::Session(SessionMsg::StartMenuOpened(_))))
        .collect();

    assert_eq!(
        opened,
        vec![&Message::Session(SessionMsg::StartMenuOpened(SessionLocation::Default))],
        "one open, for this row's location — the chevron's message, so the refresh the binary does \
         on it happens for this press too. Published: {published:?}"
    );
}

/// Where a glyph is drawn, in window pixels, asserting it is the only one of its kind on screen.
///
/// [`start_glyph`]'s rule, generalised so the chevron can be found the same way: by what it paints
/// rather than by a node path.
fn only_glyph(state: &State, icon: Icon) -> Point {
    let mut renderer = lay::renderer();
    let glyph = icon.glyph().to_string();
    let painted = lay::painted_text_settled(view(state), &mut renderer);
    let drawn: Vec<&lay::Overflow> = painted.iter().filter(|t| t.content == glyph).collect();
    assert_eq!(
        drawn.len(),
        1,
        "exactly one {icon:?} is expected on screen; painted: {:?}",
        painted.iter().map(|t| &t.content).collect::<Vec<_>>()
    );
    drawn[0].origin
}

/// Press at `at`, apply everything it published in the order it published it, and return the state.
///
/// The order is the whole subject of these two tests, so it is a real reducer run rather than a
/// scan of the message list: `press_at` returns what the widgets emitted, and `State::update` is
/// what the runtime would do with it.
fn state_after_press(mut state: State, at: Point) -> State {
    for message in press_at(&state, at) {
        state.update(message);
    }
    state
}

#[test]
fn the_list_the_primary_half_opens_hangs_from_the_press() {
    // The stored default is not installed, so the primary half opens the list rather than starting
    // anything — the same press that opens it is the only thing that knows where the row is.
    let state = with_project(AiCli::Copilot, &[AiCli::ClaudeCode]);
    let at = only_glyph(&state, Icon::AddSession);

    let state = state_after_press(state, at);

    let menu = state
        .session
        .start_menu
        .expect("the press opens the list (the assertions above); this reads where it hung it");
    assert_eq!(
        menu.anchor,
        (at.x as u16, at.y as u16),
        "the list hangs from the press that opened it, not from the window origin (FR-029d). \
         `SessionStartMenuAnchored` is published on the *press* and `SessionStartMenuOpened` on the \
         *release*, so a reducer that writes an anchor when it opens overwrites the point with a \
         constant and the panel lands over the sidebar header."
    );
}

#[test]
fn the_list_the_chevron_opens_hangs_from_the_press() {
    // Both installed, so the chevron is drawn and is the half that offers the choice.
    let state = with_project(AiCli::Copilot, &[AiCli::ClaudeCode, AiCli::Copilot]);
    let at = only_glyph(&state, Icon::SelectChevron);

    let state = state_after_press(state, at);

    let menu = state
        .session
        .start_menu
        .expect("the chevron opens the list; this reads where it hung it");
    assert_eq!(
        menu.anchor,
        (at.x as u16, at.y as u16),
        "and the chevron's press point is the chevron's, for the same reason: both halves of the \
         split can open this list and neither position is something the view holds."
    );
}
