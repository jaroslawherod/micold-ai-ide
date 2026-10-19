//! The Daemons feature in isolation (feature 491, SC-004): it constructs only its own types and
//! the core's, and names no other feature.

#[path = "support/daemons_fixture.rs"]
mod daemons_fixture;

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
    use micold_core::sandbox::image::{ImageChoice, OwnImage};

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
    fn a_container_daemon_requires_name_and_container_name_and_its_own_image_only_when_chosen() {
        let mut state = open();
        let draft = state.draft.as_mut().unwrap();
        draft.runtime = DraftRuntime::Container;
        // The basic image names nothing to fill in.
        assert_eq!(
            listed(&state),
            vec![FieldId::DaemonName, FieldId::DaemonContainerName]
        );
        let draft = state.draft.as_mut().unwrap();
        draft.profile.image = ImageChoice::Own(OwnImage::Reference(String::new()));
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
    fn an_own_image_needs_a_reference_or_an_archive_not_both() {
        let mut state = open();
        let draft = state.draft.as_mut().unwrap();
        draft.runtime = DraftRuntime::Container;
        draft.profile.image = ImageChoice::Own(OwnImage::Reference(String::new()));
        assert!(empty(&state).contains(&FieldId::DaemonImage));
        state.draft.as_mut().unwrap().image_path = "/tmp/image.tar".into();
        assert!(!empty(&state).contains(&FieldId::DaemonImage));
        assert!(!listed(&state).contains(&FieldId::DaemonImagePath));
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

/// The Kubernetes form (feature 686, M4a: T024, T130, T066).
mod kubernetes {
    use super::daemons_fixture::{kubernetes, World};
    use micold_client::features::daemons::{
        self, runtime_choices, AllowedKind, ContextList, DraftRuntime, Msg, State, VOLUME_NOTICE,
    };
    use micold_client::features::window::FieldId;
    use micold_client::features::Outcome;
    use micold_client::kube::live::ContextEntry;
    use micold_core::daemons::{DaemonEvent, DaemonId, DaemonRuntime, DaemonState};
    use micold_core::kube::credentials::KubeCredential;
    use micold_core::kube::start::DriftChoice;
    use micold_core::session::AiCli;

    pub(super) fn world() -> (World, DaemonId) {
        let mut w = World::new();
        let id = w
            .registry
            .add("Cluster", kubernetes("prod", "dev"), false)
            .unwrap();
        w.links = micold_client::links::DaemonLinks::at_launch(&w.registry);
        (w, id)
    }

    fn open(w: &mut World, id: DaemonId) -> State {
        let mut state = State::default();
        daemons::update(&mut state, &mut w.model(), Msg::EditOpened(id));
        state
    }

    pub(super) fn send(state: &mut State, w: &mut World, msg: Msg) -> Vec<Outcome> {
        daemons::update(state, &mut w.model(), msg)
    }

    fn stored(w: &World, id: DaemonId) -> micold_core::kube::settings::KubeSettings {
        match &w.registry.get(id).unwrap().runtime {
            DaemonRuntime::Kubernetes(k) => k.clone(),
            other => panic!("not Kubernetes: {other:?}"),
        }
    }

    #[test]
    fn the_add_form_lists_kubernetes_with_the_shared_ssh_image_available() {
        let mut state = State::default();
        daemons::begin_add(&mut state);
        let choices = runtime_choices(state.draft.as_ref().unwrap());
        assert!(choices.contains(&DraftRuntime::Kubernetes));
        assert!(choices.contains(&DraftRuntime::Container));
    }

    #[test]
    fn a_kubernetes_daemon_opens_in_edit_with_its_runtime_offered() {
        let (mut w, id) = world();
        let state = open(&mut w, id);
        let draft = state.draft.as_ref().unwrap();
        assert_eq!(draft.runtime, DraftRuntime::Kubernetes);
        assert_eq!(draft.kube.context, "prod");
        assert_eq!(draft.kube.namespace, "dev");
        assert!(runtime_choices(draft).contains(&DraftRuntime::Kubernetes));
    }

    #[test]
    fn opening_the_form_asks_for_the_kubeconfig_contexts_and_lists_them() {
        let (mut w, id) = world();
        let mut state = State::default();
        let out = send(&mut state, &mut w, Msg::EditOpened(id));
        assert_eq!(out, vec![Outcome::KubeContextsRequested]);
        let listed = vec![ContextEntry {
            name: "staging".into(),
            namespace: Some("apps".into()),
        }];
        send(&mut state, &mut w, Msg::ContextsListed(Ok(listed.clone())));
        assert_eq!(state.contexts, ContextList::Found(listed));
    }

    #[test]
    fn choosing_a_context_fills_an_empty_namespace_only() {
        let (mut w, id) = world();
        let mut state = open(&mut w, id);
        let listed = vec![ContextEntry {
            name: "staging".into(),
            namespace: Some("apps".into()),
        }];
        send(&mut state, &mut w, Msg::ContextsListed(Ok(listed)));
        send(&mut state, &mut w, Msg::ContextChosen("staging".into()));
        let kube = &state.draft.as_ref().unwrap().kube;
        assert_eq!(
            (kube.context.as_str(), kube.namespace.as_str()),
            ("staging", "dev")
        );
        send(&mut state, &mut w, Msg::NamespaceEdited(String::new()));
        send(&mut state, &mut w, Msg::ContextChosen("staging".into()));
        assert_eq!(state.draft.as_ref().unwrap().kube.namespace, "apps");
    }

    #[test]
    fn no_kubeconfig_is_said_with_where_it_looked() {
        let (mut w, id) = world();
        let mut state = open(&mut w, id);
        let message = "no kubeconfig found; looked in /home/u/.kube/config".to_string();
        send(
            &mut state,
            &mut w,
            Msg::ContextsListed(Err(message.clone())),
        );
        assert_eq!(state.contexts, ContextList::Failed(message));
    }

    #[test]
    fn the_ai_clis_default_to_the_providers_enabled_in_settings() {
        let (mut w, _) = world();
        let mut state = State::default();
        state.default_providers = vec![AiCli::Pi, AiCli::Copilot];
        send(&mut state, &mut w, Msg::AddOpened);
        send(
            &mut state,
            &mut w,
            Msg::RuntimeChosen(DraftRuntime::Kubernetes),
        );
        let kube = &state.draft.as_ref().unwrap().kube;
        assert_eq!(
            kube.providers.iter().copied().collect::<Vec<_>>(),
            vec![AiCli::Copilot, AiCli::Pi]
        );
        send(&mut state, &mut w, Msg::ProviderToggled(AiCli::Pi, false));
        assert!(!state
            .draft
            .as_ref()
            .unwrap()
            .kube
            .providers
            .contains(&AiCli::Pi));
    }

    #[test]
    fn the_volume_size_notice_shows_only_for_a_daemon_that_exists() {
        let (mut w, id) = world();
        let state = open(&mut w, id);
        assert_eq!(
            state.draft.as_ref().unwrap().volume_notice(),
            Some(VOLUME_NOTICE)
        );
        assert_eq!(VOLUME_NOTICE, "Applies only to a new volume, after Remove");
        let mut fresh = State::default();
        send(&mut fresh, &mut w, Msg::AddOpened);
        send(
            &mut fresh,
            &mut w,
            Msg::RuntimeChosen(DraftRuntime::Kubernetes),
        );
        assert_eq!(fresh.draft.as_ref().unwrap().volume_notice(), None);
    }

    #[test]
    fn save_refuses_an_invalid_field_naming_it_and_changes_nothing() {
        let (mut w, id) = world();
        let mut state = open(&mut w, id);
        let before = stored(&w, id);
        send(&mut state, &mut w, Msg::NamespaceEdited("Not_Valid".into()));
        send(&mut state, &mut w, Msg::Saved);
        assert_eq!(state.form_error.as_ref().unwrap().field, "namespace");
        send(&mut state, &mut w, Msg::NamespaceEdited("dev".into()));
        send(&mut state, &mut w, Msg::ContextChosen(String::new()));
        send(&mut state, &mut w, Msg::Saved);
        // An empty context is the required error on the field, not a sentence (feature 937).
        assert!(state.form_error.is_none());
        let draft = state.draft.as_ref().expect("the form stays open");
        assert_eq!(
            draft
                .required
                .state_in(&draft.required_fields(), FieldId::DaemonKubeContext),
            micold_core::required::FieldRequirement::Missing
        );
        assert_eq!(stored(&w, id), before);
    }

    #[test]
    fn an_empty_image_saves_as_the_default_and_a_typed_one_is_kept() {
        let (mut w, id) = world();
        let mut state = open(&mut w, id);
        send(
            &mut state,
            &mut w,
            Msg::KubeImageEdited("ghcr.io/me/ssh:1.2".into()),
        );
        send(&mut state, &mut w, Msg::Saved);
        assert!(state.form_error.is_none());
        assert_eq!(
            stored(&w, id).image.unwrap().to_reference(),
            "ghcr.io/me/ssh:1.2"
        );
        let mut state = open(&mut w, id);
        send(&mut state, &mut w, Msg::KubeImageEdited("  ".into()));
        send(&mut state, &mut w, Msg::Saved);
        assert!(stored(&w, id).image.is_none());
    }

    #[test]
    fn the_row_shows_kubernetes_and_the_endpoint() {
        let (mut w, id) = world();
        let state = State::default();
        let view = daemons::views(&state, &w.model());
        let row = view.rows.iter().find(|r| r.id == id).unwrap();
        assert_eq!(row.runtime_label, "Kubernetes");
        assert!(
            row.endpoint.starts_with("prod / dev / "),
            "{}",
            row.endpoint
        );
    }

    // T057 (U103): a daemon whose pod is gone shows `stopped` with the reason; Start clears it.
    #[test]
    fn a_stopped_row_shows_the_reason_the_pod_is_gone() {
        let (mut w, id) = world();
        let reason = "stopped: the pod was deleted";
        let shown = |w: &mut World| {
            let view = daemons::views(&State::default(), &w.model());
            view.rows.iter().find(|r| r.id == id).unwrap().clone()
        };
        dial(&mut w, id);
        assert_eq!(shown(&mut w).stopped_reason, None);
        w.links.apply(id, DaemonEvent::Lost("eof".into()));
        w.links.apply(id, DaemonEvent::Gone(reason.into()));
        let row = shown(&mut w);
        assert_eq!(row.state, DaemonState::Stopped);
        assert_eq!(row.stopped_reason.as_deref(), Some(reason));
        w.links.apply(id, DaemonEvent::Start);
        assert_eq!(shown(&mut w).stopped_reason, None);
    }

    // T130: the container form's limits keep their behaviour; the Kubernetes form's four fields
    // are edited on their own.
    #[test]
    fn the_container_limits_are_edited_as_before() {
        let mut w = World::new();
        let boxed = w.boxed;
        let mut state = State::default();
        send(&mut state, &mut w, Msg::EditOpened(boxed));
        send(&mut state, &mut w, Msg::CpuLimitEdited("2".into()));
        send(&mut state, &mut w, Msg::MemoryLimitEdited("4096".into()));
        send(&mut state, &mut w, Msg::Saved);
        let DaemonRuntime::Container(c) = &w.registry.get(boxed).unwrap().runtime else {
            panic!("not a container");
        };
        assert_eq!(c.profile.budget.cpus_milli.unwrap().0, 2000);
        assert_eq!(c.profile.budget.memory_bytes.unwrap().as_mib(), 4096);
    }

    #[test]
    fn the_four_kubernetes_resource_fields_are_edited_on_their_own() {
        let (mut w, id) = world();
        let mut state = open(&mut w, id);
        send(&mut state, &mut w, Msg::CpuRequestEdited("0.5".into()));
        send(&mut state, &mut w, Msg::KubeCpuLimitEdited("2".into()));
        send(&mut state, &mut w, Msg::MemoryRequestEdited("1024".into()));
        send(
            &mut state,
            &mut w,
            Msg::KubeMemoryLimitEdited("4096".into()),
        );
        send(&mut state, &mut w, Msg::Saved);
        assert!(state.form_error.is_none(), "{:?}", state.form_error);
        let r = stored(&w, id).resources;
        assert_eq!(r.cpu_request.unwrap().0, 500);
        assert_eq!(r.cpu_limit.unwrap().0, 2000);
        assert_eq!(r.memory_request.unwrap().as_mib(), 1024);
        assert_eq!(r.memory_limit.unwrap().as_mib(), 4096);
    }

    #[test]
    fn a_request_above_its_limit_is_refused_naming_the_request_field() {
        let (mut w, id) = world();
        let mut state = open(&mut w, id);
        send(&mut state, &mut w, Msg::KubeCpuLimitEdited("1".into()));
        send(&mut state, &mut w, Msg::CpuRequestEdited("2".into()));
        send(&mut state, &mut w, Msg::Saved);
        assert_eq!(state.form_error.as_ref().unwrap().field, "cpu_request");
        send(&mut state, &mut w, Msg::CpuRequestEdited(String::new()));
        send(
            &mut state,
            &mut w,
            Msg::KubeMemoryLimitEdited("1024".into()),
        );
        send(&mut state, &mut w, Msg::MemoryRequestEdited("2048".into()));
        send(&mut state, &mut w, Msg::Saved);
        assert_eq!(state.form_error.as_ref().unwrap().field, "memory_request");
    }

    #[test]
    fn a_resource_that_is_not_a_number_names_its_own_field() {
        let (mut w, id) = world();
        let mut state = open(&mut w, id);
        send(&mut state, &mut w, Msg::MemoryRequestEdited("lots".into()));
        send(&mut state, &mut w, Msg::Saved);
        assert_eq!(state.form_error.as_ref().unwrap().field, "memory_request");
    }

    // T066: the allowed-credentials list.
    #[test]
    fn the_allowed_items_include_no_git_credentials_or_ssh_key() {
        let labels: Vec<String> = AllowedKind::all()
            .into_iter()
            .map(AllowedKind::label)
            .collect();
        assert!(
            labels.iter().all(|l| !l.to_lowercase().contains("ssh")
                && !l.to_lowercase().contains("credentials")
                || l.to_lowercase().contains("registry")),
            "{labels:?}"
        );
        assert_eq!(
            AllowedKind::all().len(),
            2 + AiCli::ALL.len(),
            "git config, one sign-in per CLI, registry pull: {labels:?}"
        );
    }

    #[test]
    fn nothing_is_allowed_by_default() {
        let (mut w, _) = world();
        let mut state = State::default();
        send(&mut state, &mut w, Msg::AddOpened);
        send(
            &mut state,
            &mut w,
            Msg::RuntimeChosen(DraftRuntime::Kubernetes),
        );
        assert!(state.draft.as_ref().unwrap().kube.allowed.is_empty());
    }

    #[test]
    fn ticked_credentials_are_saved_and_unticked_ones_withdrawn() {
        let (mut w, id) = world();
        let mut state = open(&mut w, id);
        send(
            &mut state,
            &mut w,
            Msg::AllowedToggled(AllowedKind::GitConfig, true),
        );
        send(
            &mut state,
            &mut w,
            Msg::AllowedToggled(AllowedKind::SignIn(AiCli::Pi), true),
        );
        send(&mut state, &mut w, Msg::Saved);
        let allowed = stored(&w, id).allowed;
        assert!(allowed.contains(&KubeCredential::GitConfig));
        assert!(allowed.contains(&KubeCredential::SignIn { cli: AiCli::Pi }));
        assert_eq!(allowed.len(), 2);
        let mut state = open(&mut w, id);
        assert!(state
            .draft
            .as_ref()
            .unwrap()
            .kube
            .allowed
            .contains(&AllowedKind::GitConfig));
        send(
            &mut state,
            &mut w,
            Msg::AllowedToggled(AllowedKind::GitConfig, false),
        );
        send(&mut state, &mut w, Msg::Saved);
        assert_eq!(stored(&w, id).allowed.len(), 1);
    }

    #[test]
    fn registry_auth_requires_a_file() {
        let (mut w, id) = world();
        let mut state = open(&mut w, id);
        send(
            &mut state,
            &mut w,
            Msg::AllowedToggled(AllowedKind::RegistryPull, true),
        );
        send(&mut state, &mut w, Msg::Saved);
        assert_eq!(state.form_error.as_ref().unwrap().field, "pull_file");
        send(
            &mut state,
            &mut w,
            Msg::PullFileEdited("/home/u/.docker/config.json".into()),
        );
        send(&mut state, &mut w, Msg::Saved);
        assert!(state.form_error.is_none(), "{:?}", state.form_error);
        let allowed = stored(&w, id).allowed;
        assert!(allowed.contains(&KubeCredential::RegistryPull {
            auths: "/home/u/.docker/config.json".into()
        }));
    }

    // M4b: placement fields (U107), applies on next start (A19, A20), Logs (U108).

    #[test]
    fn the_placement_fields_open_filled_and_save() {
        let (mut w, id) = world();
        let mut state = open(&mut w, id);
        let draft = state.draft.as_ref().unwrap();
        assert_eq!(draft.kube.volume_size, "10Gi");
        send(
            &mut state,
            &mut w,
            Msg::NodeSelectorEdited("pool=gpu\ndisk".into()),
        );
        send(
            &mut state,
            &mut w,
            Msg::TolerationsEdited("gpu=true:NoSchedule".into()),
        );
        send(&mut state, &mut w, Msg::VolumeSizeEdited("50Gi".into()));
        send(&mut state, &mut w, Msg::StorageClassEdited("fast".into()));
        send(&mut state, &mut w, Msg::Saved);
        assert!(state.form_error.is_none(), "{:?}", state.form_error);
        let k = stored(&w, id);
        assert_eq!(k.node_selector.len(), 2);
        assert_eq!(k.tolerations.len(), 1);
        assert_eq!(k.volume_size.to_k8s(), "50Gi");
        assert_eq!(k.storage_class.as_deref(), Some("fast"));
        let reopened = open(&mut w, id);
        let kube = &reopened.draft.as_ref().unwrap().kube;
        assert_eq!(kube.node_selector, "disk=\npool=gpu");
        assert_eq!(kube.storage_class, "fast");
    }

    #[test]
    fn the_list_editors_feed_the_draft_and_follow_a_replaced_text() {
        use iced::widget::text_editor::{Action, Edit};
        use micold_client::app::EditorAction;
        let (mut w, id) = world();
        let mut state = open(&mut w, id);
        let typed = |c| EditorAction(Action::Edit(Edit::Insert(c)));
        for c in "pool=gpu".chars() {
            send(&mut state, &mut w, Msg::NodeSelectorAction(typed(c)));
        }
        send(&mut state, &mut w, Msg::TolerationsAction(typed('x')));
        let kube = &state.draft.as_ref().unwrap().kube;
        assert_eq!(kube.node_selector.trim_end(), "pool=gpu");
        assert_eq!(kube.tolerations.trim_end(), "x");
        send(&mut state, &mut w, Msg::NodeSelectorEdited("a=b".into()));
        let kube = &state.draft.as_ref().unwrap().kube;
        assert_eq!(kube.node_selector_editor.content().text().trim_end(), "a=b");
    }

    #[test]
    fn each_placement_field_refuses_an_invalid_value_naming_it() {
        for (msg, field) in [
            (Msg::NodeSelectorEdited("=x".into()), "node_selector"),
            (Msg::TolerationsEdited("nonsense".into()), "tolerations"),
            (Msg::VolumeSizeEdited("500Mi".into()), "volume_size"),
            (Msg::VolumeSizeEdited("lots".into()), "volume_size"),
            (Msg::StorageClassEdited("Not Valid".into()), "storage_class"),
        ] {
            let (mut w, id) = world();
            let before = w.registry.clone();
            let mut state = open(&mut w, id);
            send(&mut state, &mut w, msg);
            send(&mut state, &mut w, Msg::Saved);
            assert_eq!(state.form_error.as_ref().unwrap().field, field);
            assert_eq!(w.registry, before);
        }
    }

    pub(super) fn dial(w: &mut World, id: DaemonId) {
        w.links.apply(id, micold_core::daemons::DaemonEvent::Start);
        assert!(w.links.should_dial(id));
    }

    #[test]
    fn editing_a_running_daemon_saves_asks_nothing_and_applies_on_next_start() {
        let (mut w, id) = world();
        dial(&mut w, id);
        w.sessions
            .insert(id, vec![micold_core::session::SessionId::new()]);
        let mut state = open(&mut w, id);
        send(&mut state, &mut w, Msg::VolumeSizeEdited("30Gi".into()));
        let out = send(&mut state, &mut w, Msg::Saved);
        assert!(
            matches!(out.as_slice(), [Outcome::DaemonRegistryChanged(_)]),
            "{out:?}"
        );
        assert!(state.prompt().is_none());
        assert_eq!(stored(&w, id).volume_size.to_k8s(), "30Gi");
        assert!(daemons::details(&state, id).applies_on_next_start);
        assert_eq!(daemons::APPLIES_ON_NEXT_START, "Applies on next start");
        send(&mut state, &mut w, Msg::Started(id));
        assert!(!daemons::details(&state, id).applies_on_next_start);
    }

    #[test]
    fn logs_fetch_the_last_200_lines_on_demand_and_keep_the_answer() {
        let (mut w, id) = world();
        let mut state = State::default();
        assert_eq!(daemons::details(&state, id).pod_log, None);
        let out = send(&mut state, &mut w, Msg::LogsRequested(id));
        assert_eq!(
            out,
            vec![Outcome::KubeLogsRequested {
                daemon: id,
                settings: stored(&w, id),
                tail: 200
            }]
        );
        send(
            &mut state,
            &mut w,
            Msg::LogsFetched(id, Ok("line\n".into())),
        );
        assert_eq!(
            daemons::details(&state, id).pod_log,
            Some(Ok("line\n".into()))
        );
        send(&mut state, &mut w, Msg::LogsFetched(id, Err("boom".into())));
        assert_eq!(
            daemons::details(&state, id).pod_log,
            Some(Err("boom".into()))
        );
    }

    // M4b: Stop, Remove, Forget locally (U105, U106, U143, A23).

    fn names(w: &World, id: DaemonId) -> micold_core::kube::names::ResourceNames {
        let k = stored(w, id);
        micold_core::kube::names::ResourceNames::new(&k.context.0, &k.namespace.0, "Cluster")
    }

    fn with_sessions(
        w: &mut World,
        id: DaemonId,
        n: usize,
    ) -> Vec<micold_core::session::SessionId> {
        let s: Vec<_> = (0..n)
            .map(|_| micold_core::session::SessionId::new())
            .collect();
        w.sessions.insert(id, s.clone());
        s
    }

    #[test]
    fn stop_with_running_sessions_confirms_naming_how_many_end() {
        let (mut w, id) = world();
        dial(&mut w, id);
        let sessions = with_sessions(&mut w, id, 2);
        let mut state = State::default();
        assert!(send(&mut state, &mut w, Msg::Stopped(id)).is_empty());
        assert_eq!(
            state.prompt().unwrap().text,
            "Stop Cluster? 2 running sessions will end. Worktrees on its volume are kept."
        );
        assert!(w.links.should_dial(id));
        let out = send(&mut state, &mut w, Msg::RemovalConfirmed);
        assert_eq!(
            out,
            vec![
                Outcome::DaemonSessionsEnded {
                    daemon: id,
                    sessions
                },
                Outcome::KubeStopRequested {
                    daemon: id,
                    settings: stored(&w, id),
                    name: "Cluster".into()
                },
            ]
        );
        assert!(!w.links.should_dial(id));
        assert!(w.registry.get(id).is_some());
    }

    #[test]
    fn stop_with_one_session_says_so_and_declining_changes_nothing() {
        let (mut w, id) = world();
        dial(&mut w, id);
        with_sessions(&mut w, id, 1);
        let mut state = State::default();
        send(&mut state, &mut w, Msg::Stopped(id));
        assert!(state
            .prompt()
            .unwrap()
            .text
            .contains("1 running session will end"));
        assert!(send(&mut state, &mut w, Msg::RemovalCancelled).is_empty());
        assert!(w.links.should_dial(id));
    }

    #[test]
    fn stop_without_sessions_goes_straight_to_the_cluster() {
        let (mut w, id) = world();
        dial(&mut w, id);
        let mut state = State::default();
        let out = send(&mut state, &mut w, Msg::Stopped(id));
        assert!(state.prompt().is_none());
        assert_eq!(
            out,
            vec![Outcome::KubeStopRequested {
                daemon: id,
                settings: stored(&w, id),
                name: "Cluster".into()
            }]
        );
    }

    #[test]
    fn remove_names_what_is_deleted_and_deletes_nothing_before_confirmation() {
        let (mut w, id) = world();
        let n = names(&w, id);
        let before = w.registry.clone();
        let mut state = State::default();
        assert!(send(&mut state, &mut w, Msg::RemovalRequested(id)).is_empty());
        let text = &state.prompt().unwrap().text;
        let secrets = n.all_secrets(&stored(&w, id).allowed).join(", ");
        assert!(
            text.starts_with(&format!(
                "Remove Cluster? This deletes from prod/dev: pod {}, volume {} with its worktrees, secrets {secrets}. This computer's SSH key for Cluster is deleted too.",
                n.pod(),
                n.pvc()
            )),
            "{text}"
        );
        assert!(text.contains("0 worktrees"), "{text}");
        assert_eq!(w.registry, before);
        send(&mut state, &mut w, Msg::RemovalCancelled);
        assert_eq!(w.registry, before);
    }

    #[test]
    fn remove_confirmed_asks_the_cluster_and_forgets_only_when_it_is_done() {
        let (mut w, id) = world();
        dial(&mut w, id);
        let sessions = with_sessions(&mut w, id, 1);
        let mut state = State::default();
        send(&mut state, &mut w, Msg::RemovalRequested(id));
        let out = send(&mut state, &mut w, Msg::RemovalConfirmed);
        assert_eq!(
            out,
            vec![
                Outcome::DaemonSessionsEnded {
                    daemon: id,
                    sessions
                },
                Outcome::KubeRemoveRequested {
                    daemon: id,
                    settings: stored(&w, id),
                    name: "Cluster".into()
                },
            ]
        );
        assert!(w.registry.get(id).is_some());
        assert!(!w.links.should_dial(id));
        let out = send(&mut state, &mut w, Msg::KubeRemoved(id, Ok(())));
        assert!(w.registry.get(id).is_none());
        assert_eq!(
            out,
            vec![
                Outcome::DaemonRegistryChanged(w.registry.clone()),
                Outcome::KubeKeysDelete { daemon: id }
            ]
        );
    }

    #[test]
    fn an_unreachable_cluster_offers_forget_locally_naming_what_stays() {
        let (mut w, id) = world();
        let n = names(&w, id);
        let mut state = State::default();
        send(&mut state, &mut w, Msg::RemovalRequested(id));
        send(&mut state, &mut w, Msg::RemovalConfirmed);
        let out = send(
            &mut state,
            &mut w,
            Msg::KubeRemoved(id, Err("timed out".into())),
        );
        assert!(out.is_empty());
        assert!(w.registry.get(id).is_some());
        let secrets = n.all_secrets(&stored(&w, id).allowed).join(", ");
        assert_eq!(
            state.prompt().unwrap().text,
            format!(
                "The cluster could not be reached (timed out). Forget Cluster on this computer only? These stay in prod/dev: {}, {}, {secrets}; this computer's key stays authorized in {} until they are deleted.",
                n.pod(), n.pvc(), n.access_secret()
            )
        );
        let out = send(&mut state, &mut w, Msg::RemovalConfirmed);
        assert!(w.registry.get(id).is_none());
        assert_eq!(
            out,
            vec![
                Outcome::DaemonRegistryChanged(w.registry.clone()),
                Outcome::KubeKeysDelete { daemon: id }
            ]
        );
    }

    #[test]
    fn each_kubernetes_confirmation_labels_its_button_with_its_action() {
        let (mut w, id) = world();
        for (msg, label) in [
            (Msg::RemovalRequested(id), "Remove"),
            (Msg::ForgetLocallyRequested(id), "Forget locally"),
        ] {
            let mut state = State::default();
            send(&mut state, &mut w, msg);
            assert_eq!(state.prompt().unwrap().confirm_label, label);
        }
    }

    #[test]
    fn forget_locally_can_be_asked_for_directly_and_touches_no_cluster() {
        let (mut w, id) = world();
        let mut state = State::default();
        send(&mut state, &mut w, Msg::ForgetLocallyRequested(id));
        assert!(state
            .prompt()
            .unwrap()
            .text
            .starts_with("Forget Cluster on this computer only? These stay in prod/dev: "));
        let out = send(&mut state, &mut w, Msg::RemovalConfirmed);
        assert!(out.contains(&Outcome::KubeKeysDelete { daemon: id }));
        assert!(!out.iter().any(|o| matches!(
            o,
            Outcome::KubeRemoveRequested { .. } | Outcome::KubeStopRequested { .. }
        )));
        assert!(w.registry.get(id).is_none());
    }

    #[test]
    fn stop_of_a_non_kubernetes_daemon_is_unchanged() {
        let mut w = World::new();
        let boxed = w.boxed;
        w.links
            .apply(boxed, micold_core::daemons::DaemonEvent::Start);
        let mut state = State::default();
        let out = send(&mut state, &mut w, Msg::Stopped(boxed));
        assert!(out.is_empty());
        assert!(state.prompt().is_none());
        assert!(!w.links.should_dial(boxed));
    }

    // M6: the drift prompt and Cancel (T139, U141).

    fn drifted(w: &mut World, id: DaemonId, sessions: usize) -> State {
        dial(w, id);
        with_sessions(w, id, sessions);
        let mut state = State::default();
        let fields = vec!["Node selector".to_string(), "Image".to_string()];
        assert!(send(&mut state, w, Msg::KubeDrift(id, fields)).is_empty());
        state
    }

    #[test]
    fn a_drifted_pod_asks_whether_to_recreate_it_counting_the_sessions_that_end() {
        let (mut w, id) = world();
        let state = drifted(&mut w, id, 2);
        let prompt = state.prompt().unwrap();
        assert_eq!(
            prompt.text,
            "The running pod differs from the settings (Node selector, Image). Recreate it? \
             Its 2 running sessions will end; the volume is kept."
        );
        assert_eq!(prompt.confirm_label, "Recreate pod");
        assert_eq!(prompt.decline_label, "Keep running pod");
    }

    #[test]
    fn recreate_continues_the_start_with_recreate_and_ends_the_sessions() {
        let (mut w, id) = world();
        let mut state = drifted(&mut w, id, 1);
        assert!(state
            .prompt()
            .unwrap()
            .text
            .contains("Its 1 running session will end"));
        let sessions = w.sessions[&id].clone();
        let out = send(&mut state, &mut w, Msg::RemovalConfirmed);
        assert_eq!(
            out,
            vec![Outcome::DaemonSessionsEnded {
                daemon: id,
                sessions
            }]
        );
        assert_eq!(state.drift_answers().get(&id), Some(&DriftChoice::Recreate));
        assert!(state.prompt().is_none());
        assert!(w.links.should_dial(id), "the start goes on");
    }

    #[test]
    fn keep_continues_the_start_with_keep_and_so_does_dismissing_the_prompt() {
        let (mut w, id) = world();
        let mut state = drifted(&mut w, id, 2);
        assert!(send(&mut state, &mut w, Msg::RemovalCancelled).is_empty());
        assert_eq!(state.drift_answers().get(&id), Some(&DriftChoice::Keep));
        assert!(state.prompt().is_none());
        assert!(w.links.should_dial(id));
    }

    #[test]
    fn starting_again_forgets_an_earlier_answer() {
        let (mut w, id) = world();
        let mut state = drifted(&mut w, id, 0);
        send(&mut state, &mut w, Msg::RemovalCancelled);
        send(&mut state, &mut w, Msg::Stopped(id));
        send(&mut state, &mut w, Msg::Started(id));
        assert!(state.drift_answers().is_empty());
    }

    #[test]
    fn cancel_on_a_starting_row_asks_the_shell_to_stop_the_start() {
        let (mut w, id) = world();
        dial(&mut w, id);
        let mut state = State::default();
        let out = send(&mut state, &mut w, Msg::StartCancelled(id));
        assert_eq!(
            out,
            vec![Outcome::KubeStartCancelRequested {
                daemon: id,
                name: "Cluster".into()
            }]
        );
    }

    #[test]
    fn cancel_asks_nothing_of_a_daemon_that_is_not_starting() {
        let (mut w, id) = world();
        let mut state = State::default();
        assert!(send(&mut state, &mut w, Msg::StartCancelled(id)).is_empty());
    }

    fn second_kube(w: &mut World) -> DaemonId {
        let id = w
            .registry
            .add("Other", kubernetes("prod", "dev2"), false)
            .unwrap();
        w.links = micold_client::links::DaemonLinks::at_launch(&w.registry);
        id
    }

    #[test]
    fn two_drifting_daemons_are_asked_one_after_the_other() {
        let (mut w, a) = world();
        let b = second_kube(&mut w);
        dial(&mut w, a);
        dial(&mut w, b);
        let mut state = State::default();
        let f = vec!["Image".to_string()];
        send(&mut state, &mut w, Msg::KubeDrift(a, f.clone()));
        send(&mut state, &mut w, Msg::KubeDrift(b, f));
        assert!(state.prompt().unwrap().name.contains("Cluster"));
        send(&mut state, &mut w, Msg::RemovalCancelled);
        assert_eq!(state.drift_answers().get(&a), Some(&DriftChoice::Keep));
        assert!(state.prompt().unwrap().name.contains("Other"));
        send(&mut state, &mut w, Msg::RemovalConfirmed);
        assert_eq!(state.drift_answers().get(&b), Some(&DriftChoice::Recreate));
        assert!(state.prompt().is_none());
    }

    #[test]
    fn a_drift_prompt_behind_another_prompt_is_shown_when_that_closes() {
        let (mut w, id) = world();
        let b = second_kube(&mut w);
        dial(&mut w, b);
        with_sessions(&mut w, b, 1);
        let mut state = State::default();
        send(&mut state, &mut w, Msg::Stopped(b));
        let stop_text = state.prompt().unwrap().text.clone();
        dial(&mut w, id);
        send(&mut state, &mut w, Msg::KubeDrift(id, vec!["Image".into()]));
        assert_eq!(state.prompt().unwrap().text, stop_text, "not clobbered");
        send(&mut state, &mut w, Msg::RemovalCancelled);
        assert!(state
            .prompt()
            .unwrap()
            .text
            .starts_with("The running pod differs"));
    }

    #[test]
    fn cancel_of_a_daemon_whose_drift_prompt_is_queued_drops_it() {
        let (mut w, a) = world();
        let b = second_kube(&mut w);
        dial(&mut w, a);
        dial(&mut w, b);
        let mut state = State::default();
        let f = vec!["Image".to_string()];
        send(&mut state, &mut w, Msg::KubeDrift(a, f.clone()));
        send(&mut state, &mut w, Msg::KubeDrift(b, f));
        assert!(send(&mut state, &mut w, Msg::StartCancelled(b)).is_empty());
        send(&mut state, &mut w, Msg::RemovalCancelled);
        assert!(state.prompt().is_none());
        assert!(matches!(
            w.links.state(b),
            Some(micold_core::daemons::DaemonState::Unreachable { reason })
                if reason == "start cancelled"
        ));
    }

    #[test]
    fn cancel_while_the_drift_prompt_is_open_ends_the_start_as_cancelled() {
        let (mut w, id) = world();
        let mut state = drifted(&mut w, id, 0);
        assert!(send(&mut state, &mut w, Msg::StartCancelled(id)).is_empty());
        assert!(state.prompt().is_none());
        assert!(matches!(
            w.links.state(id),
            Some(micold_core::daemons::DaemonState::Unreachable { reason })
                if reason == "start cancelled"
        ));
    }
}

/// A save with a required field empty (feature 937, M5: FR-005, FR-006, FR-010, SC-004).
mod save_required {
    use super::daemons_fixture::World;
    use micold_client::features::daemons::{self, DraftRuntime, Msg, State};
    use micold_client::features::window::FieldId;
    use micold_core::daemons::DaemonId;
    use micold_core::required::FieldRequirement;

    fn open(w: &mut World) -> State {
        let mut state = State::default();
        daemons::update(&mut state, &mut w.model(), Msg::AddOpened);
        state
    }

    fn send(state: &mut State, w: &mut World, msg: Msg) {
        daemons::update(state, &mut w.model(), msg);
    }

    fn reads(state: &State, id: FieldId) -> FieldRequirement {
        let draft = state.draft.as_ref().expect("the form is open");
        draft.required.state_in(&draft.required_fields(), id)
    }

    #[test]
    fn an_empty_name_reads_missing_with_no_sentence_and_nothing_is_added() {
        let mut w = World::new();
        let before = w.registry.entries().len();
        let mut state = open(&mut w);
        assert_eq!(
            reads(&state, FieldId::DaemonName),
            FieldRequirement::Required
        );
        send(&mut state, &mut w, Msg::Saved);
        assert!(state.form_error.is_none(), "no \"A daemon needs a name.\"");
        assert_eq!(
            reads(&state, FieldId::DaemonName),
            FieldRequirement::Missing
        );
        assert_eq!(w.registry.entries().len(), before);
    }

    #[test]
    fn every_empty_required_field_reads_missing_after_one_save() {
        let mut w = World::new();
        let mut state = open(&mut w);
        send(&mut state, &mut w, Msg::RuntimeChosen(DraftRuntime::Ssh));
        send(&mut state, &mut w, Msg::Saved);
        assert!(state.form_error.is_none());
        assert_eq!(
            reads(&state, FieldId::DaemonName),
            FieldRequirement::Missing
        );
        assert_eq!(
            reads(&state, FieldId::DaemonSshHost),
            FieldRequirement::Missing
        );
        // Filling one clears only that one.
        send(&mut state, &mut w, Msg::NameEdited("Remote".into()));
        assert_eq!(
            reads(&state, FieldId::DaemonName),
            FieldRequirement::Required
        );
        assert_eq!(
            reads(&state, FieldId::DaemonSshHost),
            FieldRequirement::Missing
        );
    }

    #[test]
    fn a_container_daemon_marks_name_and_container_name_together() {
        let mut w = World::new();
        let mut state = open(&mut w);
        send(
            &mut state,
            &mut w,
            Msg::RuntimeChosen(DraftRuntime::Container),
        );
        send(&mut state, &mut w, Msg::Saved);
        assert_eq!(
            reads(&state, FieldId::DaemonName),
            FieldRequirement::Missing
        );
        assert_eq!(
            reads(&state, FieldId::DaemonContainerName),
            FieldRequirement::Missing
        );
    }

    #[test]
    fn the_ssh_host_stops_being_required_when_the_runtime_is_local_again() {
        let mut w = World::new();
        let mut state = open(&mut w);
        send(&mut state, &mut w, Msg::RuntimeChosen(DraftRuntime::Ssh));
        send(&mut state, &mut w, Msg::Saved);
        assert_eq!(
            reads(&state, FieldId::DaemonSshHost),
            FieldRequirement::Missing
        );
        send(&mut state, &mut w, Msg::RuntimeChosen(DraftRuntime::Host));
        assert_eq!(
            reads(&state, FieldId::DaemonSshHost),
            FieldRequirement::Optional
        );
    }

    #[test]
    fn a_port_that_is_not_a_number_keeps_its_own_error() {
        let mut w = World::new();
        let mut state = open(&mut w);
        send(&mut state, &mut w, Msg::RuntimeChosen(DraftRuntime::Ssh));
        send(&mut state, &mut w, Msg::NameEdited("Remote".into()));
        send(&mut state, &mut w, Msg::SshHostEdited("example.org".into()));
        send(&mut state, &mut w, Msg::SshPortEdited("abc".into()));
        send(&mut state, &mut w, Msg::Saved);
        assert_eq!(state.form_error.as_ref().expect("an error").field, "port");
        assert_eq!(
            reads(&state, FieldId::DaemonSshHost),
            FieldRequirement::Required
        );
    }

    #[test]
    fn an_empty_repository_path_marks_that_daemon_only() {
        let mut state = State::default();
        let mut w = World::new();
        let (a, b) = (DaemonId(1), DaemonId(2));
        state.repositories.insert(b, "/srv/app".into());
        let out = daemons::add_repository(&mut state, &w.model(), a);
        assert!(out.is_empty());
        assert!(state.repository_error.is_none());
        let (id_a, req_a) = state.repository_field(a);
        let (id_b, req_b) = state.repository_field(b);
        assert_eq!(
            state.repository_marks.state(id_a, req_a),
            FieldRequirement::Missing
        );
        assert_eq!(
            state.repository_marks.state(id_b, req_b),
            FieldRequirement::Required
        );
    }
}

/// Feature 1002: the sandbox no longer shares the user's git identity, and says so once.
mod retired_git_identity {
    use super::daemons_fixture::World;
    use micold_client::features::daemons::{self, Msg, State};
    use micold_client::features::Outcome;
    use micold_core::daemons::{DaemonRegistry, MigrationNotice};
    use micold_core::sandbox::CredentialShare;

    fn with_notice(w: &mut World) {
        let mut entries = w.registry.entries().to_vec();
        for e in entries.iter_mut().filter(|e| e.id == w.boxed) {
            e.notice = Some(MigrationNotice::RemovedGitCredentials);
        }
        w.registry = DaemonRegistry::new(entries, 10);
    }

    /// The boxed daemon's notice, and whether the row offers to dismiss it.
    fn notice_of(w: &mut World) -> (Option<String>, bool) {
        let vm = daemons::views(&State::default(), &w.model());
        let row = vm.rows.into_iter().find(|r| r.id == w.boxed).unwrap();
        (row.notice, row.notice_dismissable)
    }

    #[test]
    fn the_container_credential_rows_offer_only_git_configuration_and_ai_cli_sign_in() {
        let labels: Vec<&str> = CredentialShare::ALL.iter().map(|s| s.label()).collect();
        assert_eq!(labels, ["Git configuration", "AI CLI sign-in"]);
    }

    #[test]
    fn a_daemon_with_the_stored_notice_shows_its_message() {
        let mut w = World::new();
        assert_eq!(notice_of(&mut w), (None, false));
        with_notice(&mut w);
        assert_eq!(
            notice_of(&mut w),
            (
                Some(MigrationNotice::RemovedGitCredentials.message().to_string()),
                true
            )
        );
    }

    #[test]
    fn dismissing_the_notice_clears_it_from_the_registry_and_the_row() {
        let mut w = World::new();
        with_notice(&mut w);
        let mut state = State::default();
        let boxed = w.boxed;
        let out = daemons::update(&mut state, &mut w.model(), Msg::DismissNotice(boxed));
        assert!(matches!(
            out.as_slice(),
            [Outcome::DaemonRegistryChanged(_)]
        ));
        assert_eq!(w.registry.get(w.boxed).unwrap().notice, None);
        assert_eq!(notice_of(&mut w), (None, false));
        let again = daemons::update(&mut state, &mut w.model(), Msg::DismissNotice(boxed));
        assert!(again.is_empty());
    }
}

/// Feature 1092 R7: a system-raised prompt waits for the open form dialog (US5, edge case).
mod drift_behind_the_form {
    use super::kubernetes::{dial, send, world};
    use micold_client::features::daemons::{Msg, State};

    #[test]
    fn a_drift_prompt_raised_while_the_form_is_open_waits_for_it_to_close() {
        let (mut w, id) = world();
        let mut state = State::default();
        send(&mut state, &mut w, Msg::AddOpened);
        send(&mut state, &mut w, Msg::NameEdited("Typed".into()));
        dial(&mut w, id);
        send(&mut state, &mut w, Msg::KubeDrift(id, vec!["Image".into()]));
        assert!(state.prompt().is_none(), "no second modal over the form");
        assert_eq!(
            state.draft.as_ref().unwrap().name,
            "Typed",
            "never applied to the draft"
        );
        send(&mut state, &mut w, Msg::DraftDiscarded);
        assert!(
            state
                .prompt()
                .unwrap()
                .text
                .starts_with("The running pod differs"),
            "queued until the overlay closed"
        );
    }

    #[test]
    fn a_queued_drift_prompt_also_shows_after_a_save() {
        let (mut w, id) = world();
        let mut state = State::default();
        send(&mut state, &mut w, Msg::AddOpened);
        send(&mut state, &mut w, Msg::NameEdited("Plain".into()));
        send(&mut state, &mut w, Msg::ContainerNameEdited("plain".into()));
        send(&mut state, &mut w, Msg::PortEdited("7100".into()));
        dial(&mut w, id);
        send(&mut state, &mut w, Msg::KubeDrift(id, vec!["Image".into()]));
        assert!(state.prompt().is_none());
        send(&mut state, &mut w, Msg::Saved);
        assert!(state.draft.is_none());
        assert!(state.prompt().is_some(), "queued until the overlay closed");
    }
}
