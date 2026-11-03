//! The Start script part of the Daemons form in isolation (#1533, SC-004): it constructs only its
//! own types.

use micold_client::features::daemons_start::{
    startup_restart_pending, StartDraft, StartSource,
};

#[test]
fn a_new_draft_has_no_script_and_makes_no_settings() {
    let draft = StartDraft::default();
    assert_eq!(draft.source, StartSource::None);
    assert_eq!(draft.settings().unwrap(), None);
    assert_eq!(draft.inline_text(), None);
}

#[test]
fn an_inline_draft_keeps_its_text_for_saving() {
    let mut draft = StartDraft::default();
    draft.source = StartSource::Inline;
    draft.set_text("echo hi\n".into());
    assert_eq!(draft.inline_text(), Some("echo hi\n"));
    assert!(draft.settings().unwrap().is_some());
}

#[test]
fn a_file_source_with_no_path_is_refused_naming_the_script() {
    let mut draft = StartDraft::default();
    draft.source = StartSource::File;
    assert_eq!(draft.settings().unwrap_err().field, "startup_script");
}

#[test]
fn only_startup_keys_ask_for_a_restart() {
    assert!(startup_restart_pending(&["startup.shell".into()]));
    assert!(!startup_restart_pending(&["port".into()]));
}
