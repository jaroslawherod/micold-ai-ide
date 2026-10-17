//! The Daemons feature in isolation (feature 491, SC-004): it constructs only its own types and
//! the core's, and names no other feature.

use micold_client::features::daemons::{self, State};

#[test]
fn a_fresh_state_holds_nothing() {
    let state = State::default();
    assert!(state.draft.is_none());
    assert!(state.prompt().is_none());
    assert!(state.ended_by_removal().is_empty());
    assert!(daemons::details(&state, micold_core::daemons::DaemonId(1))
        .errors
        .is_empty());
}

mod required {
    use micold_client::features::daemons::{self, DraftRuntime, State};
    use micold_client::features::window::FieldId;
    use micold_core::daemons::DaemonId;
    use micold_core::required::FieldRequirement;
    use micold_core::sandbox::image::ImageSourceKind;

    fn open() -> State {
        let mut state = State::default();
        daemons::begin_add(&mut state);
        state
    }

    fn listed(state: &State) -> Vec<FieldId> {
        state
            .draft
            .as_ref()
            .unwrap()
            .required_fields()
            .into_iter()
            .filter(|(_, r)| r.required)
            .map(|(id, _)| id)
            .collect()
    }

    fn empty(state: &State) -> Vec<FieldId> {
        state
            .draft
            .as_ref()
            .unwrap()
            .required_fields()
            .into_iter()
            .filter(|(_, r)| r.required && r.empty)
            .map(|(id, _)| id)
            .collect()
    }

    #[test]
    fn a_container_daemon_requires_name_container_name_and_image() {
        let mut state = open();
        let draft = state.draft.as_mut().unwrap();
        draft.runtime = DraftRuntime::Container;
        draft.profile.image.reference.clear();
        assert_eq!(
            listed(&state),
            vec![
                FieldId::DaemonName,
                FieldId::DaemonContainerName,
                FieldId::DaemonImage
            ]
        );
        // All three are empty at once, and each reads empty (none hides behind the first).
        assert_eq!(empty(&state).len(), 3);
        let draft = state.draft.as_mut().unwrap();
        draft.name = "  ".into();
        assert!(empty(&state).contains(&FieldId::DaemonName));
    }

    #[test]
    fn the_image_file_is_required_only_for_the_imported_file_source() {
        let mut state = open();
        let draft = state.draft.as_mut().unwrap();
        draft.runtime = DraftRuntime::Container;
        assert!(!listed(&state).contains(&FieldId::DaemonImagePath));
        let draft = state.draft.as_mut().unwrap();
        draft.profile.image.kind = ImageSourceKind::ImportedFile;
        draft.image_path = " ".into();
        assert!(listed(&state).contains(&FieldId::DaemonImagePath));
        assert!(empty(&state).contains(&FieldId::DaemonImagePath));
        state.draft.as_mut().unwrap().image_path = "/tmp/image.tar".into();
        assert!(!empty(&state).contains(&FieldId::DaemonImagePath));
    }

    #[test]
    fn the_ssh_host_is_required_only_while_the_runtime_is_ssh() {
        let mut state = open();
        state.draft.as_mut().unwrap().runtime = DraftRuntime::Ssh;
        assert_eq!(
            listed(&state),
            vec![FieldId::DaemonName, FieldId::DaemonSshHost]
        );
        assert!(empty(&state).contains(&FieldId::DaemonSshHost));
        // Port, user and key file are never required.
        for never in [
            FieldId::DaemonPort,
            FieldId::DaemonSshUser,
            FieldId::DaemonSshPort,
            FieldId::DaemonSshKeyFile,
        ] {
            assert!(!listed(&state).contains(&never));
        }
        state.draft.as_mut().unwrap().runtime = DraftRuntime::Host;
        assert_eq!(listed(&state), vec![FieldId::DaemonName]);
    }

    #[test]
    fn the_repository_step_is_its_own_required_field_per_daemon() {
        let mut state = State::default();
        let (a, b) = (DaemonId(1), DaemonId(2));
        state.repositories.insert(b, "/srv/app".into());
        let (id_a, req_a) = state.repository_field(a);
        let (id_b, req_b) = state.repository_field(b);
        assert_ne!(id_a, id_b);
        assert_eq!(id_a, FieldId::DaemonRepository(a));
        assert!(req_a.required && req_a.empty);
        assert!(req_b.required && !req_b.empty);
        // Leaving one daemon's field marks that daemon's only.
        state.repository_marks.leave(id_a);
        assert_eq!(
            state.repository_marks.state(id_a, req_a),
            FieldRequirement::Missing
        );
        let empty_b = micold_core::required::Requirement::text(true, "");
        assert_eq!(
            state.repository_marks.state(id_b, empty_b),
            FieldRequirement::Required
        );
    }
}
