//! The Kubernetes part of the Daemons form in isolation (feature 686, SC-004): it constructs only
//! its own types.

use micold_client::features::daemons_kube::{AllowedKind, ContextList, KubeDraft};
use micold_core::session::AiCli;

#[test]
fn a_new_draft_installs_the_given_clis_and_allows_nothing() {
    let draft = KubeDraft::new(&[AiCli::Pi]);
    assert_eq!(
        draft.providers.iter().copied().collect::<Vec<_>>(),
        [AiCli::Pi]
    );
    assert!(draft.allowed.is_empty());
    assert_eq!(ContextList::default(), ContextList::NotLoaded);
}

#[test]
fn a_draft_with_a_context_and_namespace_makes_settings_and_round_trips() {
    let mut draft = KubeDraft::new(&[]);
    draft.context = "prod".into();
    draft.namespace = "dev".into();
    draft.allowed.insert(AllowedKind::GitConfig);
    let settings = draft.settings().unwrap();
    assert_eq!(KubeDraft::of(&settings), draft);
}

#[test]
fn a_blank_context_is_refused_naming_it() {
    let error = KubeDraft::new(&[]).settings().unwrap_err();
    assert!(
        ["context", "namespace"].contains(&error.field),
        "{}",
        error.field
    );
}
