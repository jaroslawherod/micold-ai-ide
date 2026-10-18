//! The Container section fits at the narrowest Settings width (feature 806, T046, FR-001/FR-002).
//!
//! The section holds the runtime select, the two-entry image select, and for "My own image" the
//! reference field, the archive field and the SSH-server note. A text that wants more than its
//! field was given is painted over its neighbour, and a note that sits in another column than the
//! field it is about reads as the next field's. Neither shows in a record of nodes, so this asks
//! the renderer what it drew, at [`NARROW`] and at the canonical width, for both image choices.
//!
//! Builds its own states and reads no cache; it shares the `layout_snapshot` binary with the other
//! gates so that it uses the same `support::layout` passes.

use crate::support::layout as lay;
use iced::Size;
use micold_client::app::{State, MIN_WINDOW_SIZE};
use micold_client::features::connection::ConnectionStatus;
use micold_client::features::daemons::Msg as DaemonsMsg;
use micold_client::features::daemons::{DraftRuntime, Section};
use micold_client::features::settings::{SettingsDraft, SettingsSection};
use micold_core::daemons::{ContainerSettings, DaemonRuntime};
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::sandbox::image::{ImageChoice, OwnImage};
use micold_core::sandbox::SandboxProfile;
use micold_core::session::AiCli;

/// Half a pixel, matching every geometry gate beside this one.
const TOLERANCE: f32 = 0.5;

/// The narrowest supported window, tall enough to hold the whole form.
const NARROW: Size = Size::new(MIN_WINDOW_SIZE.width, 2400.0);
/// The canonical width, as tall.
const WIDE: Size = Size::new(1280.0, 2400.0);

const SSH_NOTE: &str = "Your image must run an SSH server; see the user guide.";
const REFERENCE_SUPPORTING: &str = "A digest or an exact tag; used when no image file is given";
const ARCHIVE_SUPPORTING: &str =
    "An archive made by `docker save`, loaded instead of the reference";

fn settings_editing(image: ImageChoice) -> State {
    let mut state = State::default();
    let id = state
        .settings
        .daemons
        .add(
            "Box",
            DaemonRuntime::Container(ContainerSettings {
                profile: SandboxProfile {
                    image,
                    ..SandboxProfile::default()
                },
                container_name: "box-ctr".into(),
                port: 7001,
                migration: None,
            }),
            false,
        )
        .unwrap();
    state.settings.settings_draft = Some(SettingsDraft {
        section: SettingsSection::Daemons,
        ..SettingsDraft::default()
    });
    let _ = state.update_daemons(DaemonsMsg::EditOpened(id), Vec::new());
    // The form opens on Connection; these gates look at the Container section (feature 890).
    let _ = state.update_daemons(DaemonsMsg::SectionToggled(Section::Container), Vec::new());
    state
}

fn painted(state: &State, window: Size) -> Vec<lay::Overflow> {
    let mut renderer = lay::renderer();
    let element = micold_client::ui::view(
        state,
        None,
        None,
        0,
        None,
        &EnvIncludeOutcome::Disabled,
        &ConnectionStatus::Connected,
        &micold_client::features::sandbox::Sandbox::default(),
    );
    lay::painted_text_settled_at(element, &mut renderer, window).0
}

fn x_of(drawn: &[lay::Overflow], needle: &str) -> f32 {
    let hits: Vec<f32> = drawn
        .iter()
        .filter(|d| d.content.contains(needle))
        .map(|d| d.origin.x)
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "expected exactly one painted string containing {needle:?} — painted: {:?}",
        drawn.iter().map(|d| &d.content).collect::<Vec<_>>()
    );
    hits[0]
}

fn own() -> ImageChoice {
    ImageChoice::Own(OwnImage::Reference("registry.example/own:1".into()))
}

#[test]
fn no_text_of_the_container_section_runs_out_of_its_field() {
    for (what, image) in [("basic image", ImageChoice::Basic), ("own image", own())] {
        for window in [NARROW, WIDE] {
            let drawn = painted(&settings_editing(image.clone()), window);
            assert!(
                drawn.iter().any(|d| d.content == "Image"),
                "{what}: the Container section must be on screen — painted: {:?}",
                drawn.iter().map(|d| &d.content).collect::<Vec<_>>()
            );
            let over: Vec<String> = drawn
                .iter()
                .filter(|d| d.excess() > TOLERANCE)
                .map(|d| {
                    format!(
                        "{:?} wants {} in {}",
                        d.content, d.natural_width, d.allowed_width
                    )
                })
                .collect();
            assert!(over.is_empty(), "{what} at {}: {over:?}", window.width);
        }
    }
}

#[test]
fn the_own_image_fields_and_note_share_one_column_at_the_narrowest_width() {
    let drawn = painted(&settings_editing(own()), NARROW);
    let reference = x_of(&drawn, REFERENCE_SUPPORTING);
    let archive = x_of(&drawn, ARCHIVE_SUPPORTING);
    assert!(
        (reference - archive).abs() <= TOLERANCE,
        "the reference field's supporting line starts at {reference}, the archive field's at {archive}"
    );
    // The SSH note is the image select's supporting line, so it sits in the fields' column too.
    let note = x_of(&drawn, SSH_NOTE);
    assert!(
        (note - reference).abs() <= TOLERANCE,
        "the SSH note starts at {note}, the fields' supporting lines at {reference}"
    );
}

#[test]
fn the_basic_image_shows_none_of_the_own_image_fields() {
    let drawn = painted(&settings_editing(ImageChoice::Basic), NARROW);
    for absent in [REFERENCE_SUPPORTING, ARCHIVE_SUPPORTING, SSH_NOTE] {
        assert!(
            !drawn.iter().any(|d| d.content.contains(absent)),
            "{absent:?} must not be painted for the basic image"
        );
    }
}

#[test]
fn the_five_ai_cli_checkboxes_and_their_note_fit_and_share_one_column() {
    for window in [NARROW, WIDE] {
        let drawn = painted(&settings_editing(ImageChoice::Basic), window);
        let x: Vec<f32> = AiCli::ALL
            .iter()
            .map(|cli| x_of(&drawn, cli.provider().display_name()))
            .collect();
        assert_eq!(x.len(), 5, "the app supports five AI CLIs");
        assert!(
            x.iter().all(|v| (v - x[0]).abs() <= TOLERANCE),
            "the checkbox labels start in one column at {}: {x:?}",
            window.width
        );
        let note = drawn
            .iter()
            .find(|d| d.content.starts_with("Changes apply at the next start"))
            .expect("the note under the checkboxes is painted");
        assert!(note.excess() <= TOLERANCE, "{:?}", note.content);
    }
}

#[test]
fn a_remote_ssh_daemon_shows_no_ai_cli_section() {
    let mut state = State::default();
    state.settings.settings_draft = Some(SettingsDraft {
        section: SettingsSection::Daemons,
        ..SettingsDraft::default()
    });
    let _ = state.update_daemons(DaemonsMsg::AddOpened, Vec::new());
    let _ = state.update_daemons(DaemonsMsg::RuntimeChosen(DraftRuntime::Ssh), Vec::new());
    let drawn = painted(&state, NARROW);
    assert!(
        drawn.iter().any(|d| d.content == "Host*"),
        "the SSH form must be on screen — painted: {:?}",
        drawn.iter().map(|d| &d.content).collect::<Vec<_>>()
    );
    assert!(
        !drawn.iter().any(|d| d.content == "AI tools in this daemon"),
        "an SSH daemon's AI CLIs are its own host's"
    );
}
