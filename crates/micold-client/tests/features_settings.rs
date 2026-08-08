//! The Settings form's draft, in isolation (feature 021, SC-004).
//!
//! This file names exactly one feature module. It builds no `State`, references no other feature's
//! types, and needs no application shell.
//!
//! It is a short file, and that is itself the finding: `SettingsDraft` has no operations to test
//! because its validation lives in `main.rs`'s `SettingsSaved` arm rather than beside the type.
//! What can be asserted in isolation today is the draft's shape and its opening state. When Tier 3
//! brings the validation across, this file is where its tests belong.

use micold_client::features::settings::SettingsDraft;

#[test]
fn a_fresh_draft_shows_no_error() {
    let draft = SettingsDraft::default();

    assert!(
        draft.error.is_none(),
        "the form opens clean — an error is the result of a rejected save, not a starting state"
    );
}

#[test]
fn the_numeric_fields_are_held_as_text_so_a_half_typed_value_is_representable() {
    let draft = SettingsDraft {
        scrollback_lines: "1".into(),
        env_include_timeout: "".into(),
        ..SettingsDraft::default()
    };

    assert_eq!(
        draft.scrollback_lines, "1",
        "typing the first digit of 10000 must not be rejected mid-keystroke, which is why the \
         field is a String and not a usize"
    );
    assert!(
        draft.env_include_timeout.is_empty(),
        "an empty field is a state the user passes through, not an invalid parse to report yet"
    );
}

#[test]
fn clearing_the_enable_toggle_leaves_the_script_path_intact() {
    let draft = SettingsDraft {
        env_include_enabled: false,
        env_include_script_path: "/p/env.sh".into(),
        ..SettingsDraft::default()
    };

    assert_eq!(
        draft.env_include_script_path, "/p/env.sh",
        "turning the feature off must not discard what was configured — re-enabling it should \
         not mean re-typing the path"
    );
}
