//! The GitHub page's token rows, rendered (#1529, verification I8-1..I8-3): each row's Replace and
//! Remove press their own message, Move up is dead on the first row and Move down on the last, a
//! token expiring soon is warned about, and the pasted token is never painted as typed.

use crate::support;

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::Tree;
use iced::advanced::{clipboard, mouse, Shell};
use iced::{Element, Event, Point, Rectangle, Size};
use micold_client::app::{Message, State};
use micold_client::features::settings::{SettingsDraft, SettingsSection, TokenInput};
use micold_core::daemon_config::RepoPattern;
use support::layout::{self as lay, StateUnderTest};

const TALL: Size = Size::new(1280.0, 3000.0);
const FAKE: &str = "github_pat_FAKE_0123456789";

fn page(token: &str) -> State {
    let soon = micold_core::unix_time::unix_secs() + 3600;
    let far = soon + 400 * 86_400;
    let mut state = State::default();
    state.settings.settings_draft = Some(SettingsDraft {
        section: SettingsSection::GithubIssues,
        ..SettingsDraft::default()
    });
    state.settings.github_tokens.load([
        ("first".to_string(), Vec::<RepoPattern>::new(), far),
        ("second".to_string(), Vec::<RepoPattern>::new(), soon),
    ]);
    state
        .settings
        .github_tokens
        .token_edited(TokenInput::new(token.to_string()));
    state.window.window_size = (1280, 3000);
    state
}

fn view(under: &StateUnderTest) -> Element<'_, Message> {
    lay::view_of(under)
}

fn painted(state: State) -> Vec<(String, Point)> {
    let under = StateUnderTest::new(state);
    lay::painted_text_settled_at(view(&under), &mut lay::renderer(), TALL)
        .0
        .into_iter()
        .map(|d| (d.content, d.origin))
        .collect()
}

/// Press and release at `at`; the messages the page published, as debug text.
fn click(state: State, at: Point) -> Vec<String> {
    let under = StateUnderTest::new(state);
    let mut element = view(&under);
    let renderer = lay::renderer();
    let mut tree = Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(
        &mut tree,
        &renderer,
        &layout::Limits::new(Size::ZERO, TALL),
    );
    let mut messages: Vec<Message> = Vec::new();
    for event in [
        mouse::Event::ButtonPressed(mouse::Button::Left),
        mouse::Event::ButtonReleased(mouse::Button::Left),
    ] {
        let mut shell = Shell::new(&mut messages);
        element.as_widget_mut().update(
            &mut tree,
            &Event::Mouse(event),
            Layout::new(&node),
            mouse::Cursor::Available(at),
            &renderer,
            &mut clipboard::Null,
            &mut shell,
            &Rectangle::with_size(TALL),
        );
    }
    messages.iter().map(|m| format!("{m:?}")).collect()
}

/// The `n`th (0-based, top to bottom) button painted as `label`, pressed.
fn press(label: &str, n: usize) -> Vec<String> {
    let mut hits: Vec<Point> = painted(page(""))
        .into_iter()
        .filter(|(c, _)| c == label)
        .map(|(_, p)| p)
        .collect();
    hits.sort_by(|a, b| a.y.total_cmp(&b.y));
    let at = hits[n];
    click(page(""), Point::new(at.x + 4.0, at.y + 4.0))
}

#[test]
fn replace_and_remove_each_press_their_own_message() {
    let replace = press("Replace", 1);
    let remove = press("Remove", 1);
    assert_eq!(replace.len(), 1, "{replace:?}");
    assert!(
        replace[0].contains("GithubPatReplaceRequested(\"second\")"),
        "{replace:?}"
    );
    assert_eq!(remove.len(), 1, "{remove:?}");
    assert!(
        remove[0].contains("GithubPatRemoveRequested(\"second\")"),
        "{remove:?}"
    );
}

#[test]
fn move_up_is_dead_on_the_first_row_and_move_down_on_the_last() {
    assert!(press("Move up", 0).is_empty(), "first row moves up");
    assert!(press("Move down", 1).is_empty(), "last row moves down");
    let down = press("Move down", 0);
    assert!(
        down.iter().any(|m| m.contains("GithubPatMoved(0, Next)")),
        "{down:?}"
    );
    let up = press("Move up", 1);
    assert!(
        up.iter().any(|m| m.contains("GithubPatMoved(1, Prev)")),
        "{up:?}"
    );
}

#[test]
fn only_the_token_expiring_soon_is_warned_about() {
    let texts: Vec<String> = painted(page("")).into_iter().map(|(c, _)| c).collect();
    let warned: Vec<_> = texts
        .iter()
        .filter(|c| c.contains("paste a new token"))
        .collect();
    assert_eq!(warned.len(), 1, "{texts:?}");
    assert!(warned[0].contains("second"), "{warned:?}");
}

#[test]
fn a_pasted_token_is_never_painted_as_typed() {
    let texts: Vec<String> = painted(page(FAKE)).into_iter().map(|(c, _)| c).collect();
    assert!(
        texts.iter().any(|c| c == "Token"),
        "the field is drawn: {texts:?}"
    );
    assert!(!texts.iter().any(|c| c.contains("FAKE")), "{texts:?}");
}
