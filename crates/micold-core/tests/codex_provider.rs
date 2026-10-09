//! `CodexProvider` — the Codex profile of the AI CLI seam, fresh start only (feature 488, M1, T003).
//!
//! Every derivation is pure, so the provider is testable without the CLI installed.

use micold_core::provider::{
    ActivitySource, AiCliProvider, CodexProvider, FolderTrust, InputReadiness, ToolServerSupport,
};
use micold_core::session::AiCli;
use micold_core::terminal::LaunchMode;
use std::path::Path;
use uuid::Uuid;

fn id() -> Uuid {
    Uuid::parse_str("44444444-5555-4555-8555-666666666666").unwrap()
}

#[test]
fn identity_is_codex() {
    assert_eq!(CodexProvider.command(), "codex");
    assert_eq!(CodexProvider.display_name(), "Codex");
    assert_eq!(CodexProvider.id(), AiCli::Codex);
    assert_eq!(AiCli::Codex.tool_name(), "codex");
    assert_eq!(AiCli::Codex.provider().command(), "codex");
}

#[test]
fn a_fresh_start_passes_no_arguments() {
    assert!(CodexProvider
        .launch_args(id(), LaunchMode::Fresh)
        .is_empty());
}

#[test]
fn availability_follows_the_path_it_is_given() {
    let dir = tempfile::tempdir().unwrap();
    let path = std::env::join_paths([dir.path()]).unwrap();
    assert!(!CodexProvider.is_available(&path));
    std::fs::write(dir.path().join("codex"), "").unwrap();
    assert!(CodexProvider.is_available(&path));
    assert!(!CodexProvider.is_available(std::ffi::OsStr::new("")));
}

#[test]
fn an_empty_store_contributes_nothing() {
    let cwd = Path::new("/work");
    let base = Path::new("/nowhere");
    assert!(CodexProvider.recorded_session_ids(base, cwd).is_empty());
    assert!(!CodexProvider.has_recorded_conversation(base, cwd, id()));
    assert_eq!(CodexProvider.read_title(base, cwd, id()), None);
    assert_eq!(CodexProvider.read_label(base, cwd, id()), None);
    assert_eq!(CodexProvider.name_in_terminal_title("Codex", cwd), None);
}

#[test]
fn the_seam_answers_for_activity_tools_readiness_and_trust() {
    assert_eq!(
        CodexProvider.activity_source(Path::new("/b"), Path::new("/w"), id()),
        ActivitySource::None
    );
    assert!(matches!(
        CodexProvider.tool_server_support(),
        ToolServerSupport::Unsupported { .. }
    ));
    assert_eq!(
        CodexProvider.input_readiness(),
        InputReadiness::OutputSettled
    );
    assert_eq!(CodexProvider.folder_trust(), FolderTrust::CodexProjects);
    assert_eq!(CodexProvider.launch_env(), Vec::<(String, String)>::new());
}

#[test]
fn closing_a_session_leaves_a_marker_outside_the_cli_store() {
    let base = tempfile::tempdir().unwrap();
    let cwd = Path::new("/work");
    assert!(!CodexProvider.is_archived(base.path(), cwd, id()));
    CodexProvider.mark_archived(base.path(), cwd, id()).unwrap();
    assert!(CodexProvider.is_archived(base.path(), cwd, id()));
    assert!(!CodexProvider.is_archived(base.path(), cwd, Uuid::from_u128(7)));
}

// --- feature 488, M3 (T018): resume and naming from Codex's own rollout store ---

mod resume {
    use super::*;
    use micold_core::provider::{sole_candidate, ConversationIdentity, ConversationRef};
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime};

    const A: &str = "11111111-aaaa-4aaa-8aaa-000000000001";
    const B: &str = "22222222-bbbb-4bbb-8bbb-000000000002";

    fn cwd(dir: &Path) -> PathBuf {
        dir.join("work")
    }

    /// `sessions/2026/10/09/rollout-<ts>-<id>.jsonl` with a `session_meta` line and `body` lines.
    fn rollout(base: &Path, day: &str, id: &str, recorded_cwd: &Path, body: &[&str]) -> PathBuf {
        let dir = base.join("sessions").join("2026").join("10").join(day);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("rollout-2026-10-{day}T10-00-00-{id}.jsonl"));
        let meta = serde_json::json!({
            "type": "session_meta",
            "payload": {"id": id, "cwd": recorded_cwd.to_string_lossy()},
        });
        let mut text = format!("{meta}\n");
        for line in body {
            text.push_str(line);
            text.push('\n');
        }
        std::fs::write(&path, text).unwrap();
        path
    }

    fn typed(text: &str) -> String {
        serde_json::json!({
            "type": "event_msg",
            "payload": {"type": "user_message", "message": text},
        })
        .to_string()
    }

    fn since() -> SystemTime {
        SystemTime::now() - Duration::from_secs(5)
    }

    fn bind(base: &Path, session: Uuid, conversation: &str, cwd: &Path) -> std::io::Result<()> {
        CodexProvider.bind(
            base,
            session,
            &ConversationRef {
                id: conversation.to_string(),
                cwd: cwd.to_path_buf(),
                created: SystemTime::now(),
            },
        )
    }

    #[test]
    fn codex_mints_its_own_ids() {
        assert_eq!(CodexProvider.identity(), ConversationIdentity::Minted);
    }

    #[test]
    fn a_new_rollout_in_this_directory_is_a_candidate() {
        let base = tempfile::tempdir().unwrap();
        let work = cwd(base.path());
        rollout(base.path(), "09", A, &work, &[]);
        let found = CodexProvider.new_conversations(base.path(), &work, since());
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, A);
        assert_eq!(found[0].cwd, work);
    }

    #[test]
    fn a_rollout_for_another_directory_is_not() {
        let base = tempfile::tempdir().unwrap();
        rollout(base.path(), "09", A, &base.path().join("elsewhere"), &[]);
        assert!(CodexProvider
            .new_conversations(base.path(), &cwd(base.path()), since())
            .is_empty());
    }

    #[test]
    fn a_rollout_older_than_the_spawn_is_not() {
        let base = tempfile::tempdir().unwrap();
        let work = cwd(base.path());
        let path = rollout(base.path(), "09", A, &work, &[]);
        let old = SystemTime::now() - Duration::from_secs(3600);
        std::fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(old)
            .unwrap();
        assert!(CodexProvider
            .new_conversations(base.path(), &work, since())
            .is_empty());
    }

    #[test]
    fn a_conversation_another_session_holds_is_not_offered_again() {
        let base = tempfile::tempdir().unwrap();
        let work = cwd(base.path());
        rollout(base.path(), "09", A, &work, &[]);
        rollout(base.path(), "09", B, &work, &[]);
        bind(base.path(), Uuid::from_u128(1), A, &work).unwrap();
        let found = CodexProvider.new_conversations(base.path(), &work, since());
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, B);
    }

    #[test]
    fn only_exactly_one_candidate_is_ever_bound() {
        let base = tempfile::tempdir().unwrap();
        let work = cwd(base.path());
        let found = |base: &Path| CodexProvider.new_conversations(base, &work, since());
        assert!(
            sole_candidate(&found(base.path())).is_none(),
            "0 candidates"
        );
        rollout(base.path(), "09", A, &work, &[]);
        assert_eq!(sole_candidate(&found(base.path())).unwrap().id, A);
        rollout(base.path(), "09", B, &work, &[]);
        assert!(
            sole_candidate(&found(base.path())).is_none(),
            "2 candidates"
        );
    }

    #[test]
    fn a_bound_session_resumes_its_conversation_and_an_unbound_one_starts_fresh() {
        let base = tempfile::tempdir().unwrap();
        let work = cwd(base.path());
        rollout(base.path(), "09", A, &work, &[]);
        let session = Uuid::from_u128(1);
        let resume = |s| CodexProvider.launch_args_in(Some(base.path()), s, LaunchMode::Resume);
        assert!(resume(session).is_empty(), "unbound starts fresh");
        bind(base.path(), session, A, &work).unwrap();
        assert_eq!(resume(session), vec!["resume".to_string(), A.to_string()]);
        assert!(
            resume(Uuid::from_u128(2)).is_empty(),
            "never another session's"
        );
        assert!(CodexProvider
            .launch_args_in(Some(base.path()), session, LaunchMode::Fresh)
            .is_empty());
        assert!(CodexProvider
            .launch_args_in(None, session, LaunchMode::Resume)
            .is_empty());
    }

    #[test]
    fn a_binding_to_a_rollout_that_is_gone_starts_fresh() {
        let base = tempfile::tempdir().unwrap();
        let session = Uuid::from_u128(1);
        bind(base.path(), session, A, &cwd(base.path())).unwrap();
        assert!(CodexProvider
            .launch_args_in(Some(base.path()), session, LaunchMode::Resume)
            .is_empty());
        assert!(!CodexProvider.has_recorded_conversation(base.path(), Path::new("/w"), session));
    }

    #[test]
    fn a_binding_is_written_once_and_never_reassigned() {
        let base = tempfile::tempdir().unwrap();
        let work = cwd(base.path());
        let session = Uuid::from_u128(1);
        rollout(base.path(), "09", A, &work, &[]);
        bind(base.path(), session, A, &work).unwrap();
        bind(base.path(), session, A, &work).unwrap();
        assert!(bind(base.path(), session, B, &work).is_err(), "other id");
        assert!(
            bind(base.path(), Uuid::from_u128(2), A, &work).is_err(),
            "another session's conversation"
        );
        assert!(bind(base.path(), Uuid::from_u128(3), "--last", &work).is_err());
    }

    #[test]
    fn a_binding_whose_conversation_is_gone_can_be_replaced() {
        let base = tempfile::tempdir().unwrap();
        let work = cwd(base.path());
        let session = Uuid::from_u128(1);
        bind(base.path(), session, A, &work).unwrap();
        rollout(base.path(), "09", B, &work, &[]);
        bind(base.path(), session, B, &work).unwrap();
        assert_eq!(
            CodexProvider.launch_args_in(Some(base.path()), session, LaunchMode::Resume),
            vec!["resume".to_string(), B.to_string()]
        );
    }

    #[test]
    fn has_recorded_conversation_follows_the_binding_and_the_rollout() {
        let base = tempfile::tempdir().unwrap();
        let work = cwd(base.path());
        let session = Uuid::from_u128(1);
        rollout(base.path(), "09", A, &work, &[]);
        assert!(!CodexProvider.has_recorded_conversation(base.path(), &work, session));
        bind(base.path(), session, A, &work).unwrap();
        assert!(CodexProvider.has_recorded_conversation(base.path(), &work, session));
    }

    #[test]
    fn the_session_is_named_from_its_first_typed_turn() {
        let base = tempfile::tempdir().unwrap();
        let work = cwd(base.path());
        let session = Uuid::from_u128(1);
        let context = serde_json::json!({
            "type": "response_item",
            "payload": {"type": "message", "role": "user",
                "content": [{"type": "input_text", "text": "<environment_context>x</environment_context>"}]},
        })
        .to_string();
        rollout(
            base.path(),
            "09",
            A,
            &work,
            &[&context, &typed("fix the\n flaky   test"), &typed("second")],
        );
        bind(base.path(), session, A, &work).unwrap();
        assert_eq!(
            CodexProvider
                .read_title(base.path(), &work, session)
                .as_deref(),
            Some("fix the flaky test")
        );
        assert_eq!(CodexProvider.read_label(base.path(), &work, session), None);
    }

    #[test]
    fn a_response_item_turn_names_the_session_when_there_is_no_event() {
        let base = tempfile::tempdir().unwrap();
        let work = cwd(base.path());
        let session = Uuid::from_u128(1);
        let turn = serde_json::json!({
            "type": "response_item",
            "payload": {"type": "message", "role": "user",
                "content": [{"type": "input_text", "text": "add a test"}]},
        })
        .to_string();
        rollout(base.path(), "09", A, &work, &[&turn]);
        bind(base.path(), session, A, &work).unwrap();
        assert_eq!(
            CodexProvider
                .read_title(base.path(), &work, session)
                .as_deref(),
            Some("add a test")
        );
    }

    #[test]
    fn only_a_bounded_prefix_is_read_for_the_name() {
        let base = tempfile::tempdir().unwrap();
        let work = cwd(base.path());
        let session = Uuid::from_u128(1);
        let filler = serde_json::json!({"type": "event_msg", "payload": {"type": "token_count", "pad": "x".repeat(1000)}}).to_string();
        let mut body: Vec<String> = (0..100).map(|_| filler.clone()).collect();
        body.push(typed("too late"));
        let body: Vec<&str> = body.iter().map(String::as_str).collect();
        rollout(base.path(), "09", A, &work, &body);
        bind(base.path(), session, A, &work).unwrap();
        assert_eq!(CodexProvider.read_title(base.path(), &work, session), None);
    }

    #[test]
    fn a_missing_or_unreadable_store_yields_nothing() {
        let missing = Path::new("/nowhere/at/all");
        let work = Path::new("/w");
        assert!(CodexProvider
            .new_conversations(missing, work, since())
            .is_empty());
        let base = tempfile::tempdir().unwrap();
        // A file where the sessions directory should be, and a binding that is not an id.
        std::fs::write(base.path().join("sessions"), "x").unwrap();
        std::fs::create_dir_all(base.path().join("micold-bindings")).unwrap();
        std::fs::write(
            base.path().join("micold-bindings").join(id().to_string()),
            "../x y",
        )
        .unwrap();
        assert!(CodexProvider
            .new_conversations(base.path(), work, since())
            .is_empty());
        assert_eq!(CodexProvider.read_title(base.path(), work, id()), None);
        assert!(CodexProvider
            .launch_args_in(Some(base.path()), id(), LaunchMode::Resume)
            .is_empty());
    }

    #[test]
    fn closing_marks_the_session_archived_outside_the_cli_store() {
        let base = tempfile::tempdir().unwrap();
        CodexProvider
            .mark_archived(base.path(), Path::new("/w"), id())
            .unwrap();
        assert!(CodexProvider.is_archived(base.path(), Path::new("/w"), id()));
        assert!(!base.path().join("sessions").exists());
    }
}
