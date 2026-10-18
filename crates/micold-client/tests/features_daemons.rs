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

/// The Kubernetes form (feature 686, M4a: T024, T130, T066).
mod kubernetes {
    use super::daemons_fixture::{kubernetes, World};
    use micold_client::features::daemons::{
        self, runtime_choices, AllowedKind, ContextList, DraftRuntime, Msg, State, VOLUME_NOTICE,
    };
    use micold_client::features::Outcome;
    use micold_client::kube::live::ContextEntry;
    use micold_core::daemons::{DaemonId, DaemonRuntime};
    use micold_core::kube::credentials::KubeCredential;
    use micold_core::session::AiCli;

    fn world() -> (World, DaemonId) {
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

    fn send(state: &mut State, w: &mut World, msg: Msg) -> Vec<Outcome> {
        daemons::update(state, &mut w.model(), msg)
    }

    fn stored(w: &World, id: DaemonId) -> micold_core::kube::settings::KubeSettings {
        match &w.registry.get(id).unwrap().runtime {
            DaemonRuntime::Kubernetes(k) => k.clone(),
            other => panic!("not Kubernetes: {other:?}"),
        }
    }

    #[test]
    fn the_add_form_omits_kubernetes_in_a_build_without_the_mechanism() {
        let mut state = State::default();
        daemons::begin_add(&mut state);
        let choices = runtime_choices(state.draft.as_ref().unwrap());
        assert!(!choices.contains(&DraftRuntime::Kubernetes));
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
        assert_eq!(state.form_error.as_ref().unwrap().field, "context");
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
    fn an_ssh_key_and_registry_auth_require_a_file() {
        let (mut w, id) = world();
        let mut state = open(&mut w, id);
        send(
            &mut state,
            &mut w,
            Msg::AllowedToggled(AllowedKind::GitSshKey, true),
        );
        send(&mut state, &mut w, Msg::Saved);
        assert_eq!(state.form_error.as_ref().unwrap().field, "ssh_key_file");
        send(
            &mut state,
            &mut w,
            Msg::KubeSshKeyEdited("/home/u/.ssh/id_ed25519".into()),
        );
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
        assert!(allowed.contains(&KubeCredential::GitSshKey {
            key: "/home/u/.ssh/id_ed25519".into()
        }));
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

    fn dial(w: &mut World, id: DaemonId) {
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
}
