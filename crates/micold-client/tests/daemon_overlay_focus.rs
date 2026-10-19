//! Tab stays inside the daemon dialogs (feature 1092, research R5, FR-009).
//!
//! `Message::FocusMoved` is answered by iced's `focus_next` / `focus_previous`, which walk every
//! focusable widget of the whole view. The probe below drives that operation against the real
//! view with a dialog open and records where the keyboard lands.
//!
//! ## Probe outcome (T001)
//!
//! Unscoped, Tab escaped. With the removal dialog open the view held 21 focusables, 2 of them the
//! dialog's; iced's `focus_next` walked the header and the sidebar behind the scrim (the first Tab
//! landed on the header's project button). The application therefore scopes the traversal to the
//! container every dialog `Surface` carries (`ui::dialog_focus_move`) while a daemon dialog is open
//! (`features::daemons::traps_focus`), and T013 applies. The tests below drive that same scoped
//! operation, so they fail if the dialog loses its scope container.

#[path = "support/mod.rs"]
mod support;

use std::sync::{Arc, Mutex};

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::operation::{Focusable, Outcome};
use iced::advanced::widget::{Id, Operation, Tree};
use iced::{Element, Rectangle, Size};
use micold_client::app::{Message, State};
use micold_client::features::connection::ConnectionStatus;
use micold_client::features::daemons::Msg as DaemonsMsg;
use micold_client::features::settings::{SettingsDraft, SettingsSection};
use micold_client::links::DaemonLinks;
use micold_core::daemons::{DaemonId, DaemonRegistry, DaemonRuntime};
use micold_core::env_include::EnvIncludeOutcome;
use support::layout as lay;

const WINDOW: Size = Size::new(1280.0, 900.0);

/// Every focusable the traversal reaches, in traversal order, with whether it holds the keyboard.
#[derive(Clone, Default)]
struct Reached(Arc<Mutex<Vec<(Rectangle, bool)>>>);

struct Record(Reached);

impl Operation for Record {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn focusable(&mut self, _id: Option<&Id>, bounds: Rectangle, state: &mut dyn Focusable) {
        (self.0)
            .0
            .lock()
            .unwrap()
            .push((bounds, state.is_focused()));
    }
}

/// A mounted view whose focus can be moved and read, the way the runtime does it.
struct Mounted<'a> {
    element: Element<'a, Message>,
    tree: Tree,
    node: layout::Node,
    renderer: iced::Renderer,
}

impl<'a> Mounted<'a> {
    fn new(mut element: Element<'a, Message>) -> Self {
        let renderer = lay::renderer();
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, WINDOW),
        );
        Self {
            element,
            tree,
            node,
            renderer,
        }
    }

    fn operate(&mut self, operation: Box<dyn Operation>) {
        let mut current = operation;
        loop {
            self.element.as_widget_mut().operate(
                &mut self.tree,
                Layout::new(&self.node),
                &self.renderer,
                current.as_mut(),
            );
            match current.finish() {
                Outcome::Chain(next) => current = next,
                Outcome::Some(()) | Outcome::None => break,
            }
        }
    }

    fn tab(&mut self) {
        self.operate(Box::new(micold_client::ui::dialog_focus_move::<()>(true)));
    }

    fn reached(&mut self) -> Vec<(Rectangle, bool)> {
        let reached = Reached::default();
        self.operate(Box::new(Record(reached.clone())));
        let out = reached.0.lock().unwrap().clone();
        out
    }
}

fn container(name: &str, port: u16) -> DaemonRuntime {
    DaemonRuntime::Container(micold_core::daemons::ContainerSettings {
        profile: micold_core::sandbox::SandboxProfile::default(),
        container_name: name.into(),
        port,
        migration: None,
    })
}

/// Settings on the Daemons page with two daemons.
fn state() -> (State, [DaemonId; 2]) {
    let mut state = State::default();
    let mut registry = DaemonRegistry::new(Vec::new(), 1);
    let host = registry.add("Host", DaemonRuntime::Host, false).unwrap();
    let boxed = registry.add("Box", container("box", 7001), false).unwrap();
    state.settings.links = DaemonLinks::at_launch(&registry);
    state.settings.daemons = registry;
    state.settings.settings_draft = Some(SettingsDraft {
        section: SettingsSection::Daemons,
        ..SettingsDraft::default()
    });
    state.window.window_size = (1280, 900);
    (state, [host, boxed])
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

/// How many focusables the open dialog has on its own.
fn dialog_focusables(state: &State) -> usize {
    let outcome = EnvIncludeOutcome::Disabled;
    let open = micold_client::overlay::registry::open_dialog(state).expect("a dialog is open");
    let body =
        open.view().expect("a dialog view")(state, micold_core::theme::ColorScheme::Dark, &outcome)
            .expect("the dialog draws");
    let count = Mounted::new(body).reached().len();
    count
}

/// Tab through the whole view and return the traversal positions the keyboard visited that are
/// not in the dialog (the dialog's focusables are the last ones the traversal reaches).
fn escapes(state: &State) -> Vec<usize> {
    let own = dialog_focusables(state);
    let mut mounted = Mounted::new(view(state));
    let total = mounted.reached().len();
    assert!(own > 0 && total > own, "setup: {own} of {total} focusables");
    let mut left = Vec::new();
    for _ in 0..total + 2 {
        mounted.tab();
        // iced's traversal does not wrap: past the last control nothing holds the keyboard, and the
        // next Tab starts again at the first. Neither is an escape.
        let Some(at) = mounted.reached().iter().position(|(_, focused)| *focused) else {
            continue;
        };
        if at < total - own {
            left.push(at);
        }
    }
    left
}

#[test]
fn tab_never_leaves_the_removal_dialog() {
    let (mut state, [host, _]) = state();
    state.update(Message::Daemons(DaemonsMsg::RemovalRequested(host)));
    let left = escapes(&state);
    assert!(
        left.is_empty(),
        "Tab walked {} step(s) onto the page behind the dialog: {left:?}",
        left.len()
    );
}

#[test]
fn both_daemon_dialogs_ask_for_the_scoped_traversal() {
    let (mut state, [host, _]) = state();
    state.update(Message::Daemons(DaemonsMsg::RemovalRequested(host)));
    let open = micold_client::overlay::registry::open_dialog(&state).expect("removal dialog");
    assert!(micold_client::features::daemons::traps_focus(open.id()));
    state.update(Message::Daemons(DaemonsMsg::RemovalCancelled));
    state.update(Message::Daemons(DaemonsMsg::AddOpened));
    let open = micold_client::overlay::registry::open_dialog(&state).expect("form dialog");
    assert!(micold_client::features::daemons::traps_focus(open.id()));
}

#[test]
fn tab_never_leaves_the_daemon_form() {
    let (mut state, _) = state();
    state.update(Message::Daemons(DaemonsMsg::AddOpened));
    let left = escapes(&state);
    assert!(
        left.is_empty(),
        "Tab walked {} step(s) onto the page behind the form: {left:?}",
        left.len()
    );
}
