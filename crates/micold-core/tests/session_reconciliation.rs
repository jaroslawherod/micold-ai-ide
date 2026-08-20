//! T065/T066 (bugfix 002/BUG-001) — session reconciliation from AI CLI provider stores
//! (FR-020b, SC-010), now per provider (feature 026, T042/T043 — FR-014, FR-015).
//!
//! # Be honest about what this file is
//!
//! It is a **mirror**. `reconcile_sessions_from_transcripts` (`src/main.rs`) was the real function
//! the client called at every project-open site, `src/main.rs` is the GUI binary and cannot be
//! linked from an integration test, so the logic was restated here from the same public seam.
//!
//! That function **no longer exists**. Feature 026 found it gone — nothing in any `src/` calls
//! `discover_transcript_session_ids`, `transcript_dir` or `is_archived` — which makes FR-014
//! net-new behaviour rather than a generalisation of something already running.
//!
//! So this file is a cheap place to pin the *rules* discovery must follow, and it is **not** the
//! gate on FR-014. That gate is `micold-daemon/tests/session_discovery.rs`, against the real entry
//! point R15 settled: the daemon's `AttachProject` arm. Nothing here should be counted as covering
//! the requirement, and the rules below are worth stating precisely because the real
//! implementation has to satisfy them too.

use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLocation, TerminalMode,
};
use micold_core::workspace::Workspace;
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use uuid::Uuid;

/// Where each provider's store lives, for one run of [`reconcile`].
type Stores = BTreeMap<AiCli, PathBuf>;

/// The rules a discovery pass must follow, restated over the public seam.
///
/// `locations` stands in for `[SessionLocation::Default] + every currently-valid worktree`.
///
/// Four properties, and three of them are feature 026's:
///
/// - it asks **every** registered provider, not one;
/// - a discovered session is assigned the provider **whose store it came from**;
/// - a session the workspace already knows is skipped **before** any archived check, so a location
///   holding hundreds of known conversations does no per-conversation filesystem work (R15);
/// - and a known session's provider is never re-derived from disk, whichever store its id turns up
///   in (data-model invariant 3).
fn reconcile(
    workspace: &mut Workspace,
    repo: &Path,
    stores: &Stores,
    locations: &[SessionLocation],
) {
    let mut seen: HashSet<Uuid> = workspace
        .sessions
        .get(repo)
        .map(|list| list.iter().map(|s| s.id.0).collect())
        .unwrap_or_default();

    let mut reconstructed = Vec::new();
    for which in AiCli::ALL {
        // Each provider's config directory is resolved independently: one being unavailable must
        // not suppress the other's contribution.
        let Some(config_dir) = stores.get(&which) else {
            continue;
        };
        let provider = which.provider();
        for location in locations {
            let cwd = location.cwd(repo);
            for session_id in provider.recorded_session_ids(config_dir, &cwd) {
                // Subtract what we already know *first*. `is_archived` is a filesystem stat per
                // id, and running it over conversations the application already has a record of is
                // exactly the per-conversation cost FR-014's proportionality rule forbids.
                if !seen.insert(session_id) {
                    continue;
                }
                if provider.is_archived(config_dir, &cwd, session_id) {
                    continue;
                }
                let label = match provider.read_title(config_dir, &cwd, session_id) {
                    Some(title) => SessionLabel::Named(title),
                    None => SessionLabel::Pending,
                };
                reconstructed.push(Session::restored(
                    // The CLI's own conversation uuid *is* the session id, which is what makes a
                    // reopen idempotent rather than a duplicate.
                    SessionId::from_uuid(session_id),
                    location.clone(),
                    label,
                    TerminalMode::AiCli,
                    which,
                ));
            }
        }
    }
    if !reconstructed.is_empty() {
        workspace
            .sessions
            .entry(repo.to_path_buf())
            .or_default()
            .extend(reconstructed);
    }
}

/// A store map naming only Claude Code's directory — the shape every pre-026 test here assumed.
fn claude_only(dir: &Path) -> Stores {
    Stores::from([(AiCli::ClaudeCode, dir.to_path_buf())])
}

/// Write a fake `claude` transcript file directly, bypassing any real `claude` process — mirrors
/// how `tests/session_title_sync.rs` and `tests/ai_cli_provider.rs` fabricate transcripts.
fn write_transcript(config_dir: &Path, cwd: &Path, session_id: Uuid, title: Option<&str>) {
    let encoded: String = cwd
        .to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let dir = config_dir.join("projects").join(encoded);
    std::fs::create_dir_all(&dir).unwrap();
    let line = match title {
        Some(title) => format!(r#"{{"type":"ai-title","aiTitle":"{title}"}}"#),
        // No title record — still "has a conversation" (session_has_conversation only checks
        // the transcript's existence, not its contents).
        None => r#"{"type":"user","message":"hi"}"#.to_string(),
    };
    std::fs::write(dir.join(format!("{session_id}.jsonl")), line).unwrap();
}

#[test]
fn orphan_transcripts_are_reconstructed_with_correct_location_and_title() {
    let config = tempdir().unwrap();
    let repo = PathBuf::from("/repo");
    let root_id = Uuid::new_v4();
    let worktree_id = Uuid::new_v4();
    let worktree_location = SessionLocation::Worktree("feat-x".to_string());

    write_transcript(config.path(), &repo, root_id, Some("Root session"));
    write_transcript(
        config.path(),
        &worktree_location.cwd(&repo),
        worktree_id,
        None,
    );

    let mut ws = Workspace::empty();
    let locations = vec![SessionLocation::Default, worktree_location.clone()];
    reconcile(&mut ws, &repo, &claude_only(config.path()), &locations);

    let sessions = ws.sessions.get(&repo).expect("sessions reconstructed");
    assert_eq!(sessions.len(), 2);

    let root = sessions
        .iter()
        .find(|s| s.id.0 == root_id)
        .expect("root session found");
    assert_eq!(root.location, SessionLocation::Default);
    assert_eq!(root.label, SessionLabel::Named("Root session".to_string()));

    let worktree = sessions
        .iter()
        .find(|s| s.id.0 == worktree_id)
        .expect("worktree session found");
    assert_eq!(worktree.location, worktree_location);
    assert_eq!(worktree.label, SessionLabel::Pending, "no title record yet");
}

#[test]
fn a_transcript_matching_an_existing_record_is_not_duplicated() {
    let config = tempdir().unwrap();
    let repo = PathBuf::from("/repo");
    let known_id = Uuid::new_v4();
    write_transcript(config.path(), &repo, known_id, Some("Known"));

    let mut ws = Workspace::empty();
    ws.sessions.insert(
        repo.clone(),
        vec![Session::restored(
            SessionId::from_uuid(known_id),
            SessionLocation::Default,
            SessionLabel::Named("Known".to_string()),
            TerminalMode::AiCli,
            AiCli::ClaudeCode,
        )],
    );

    reconcile(
        &mut ws,
        &repo,
        &claude_only(config.path()),
        &[SessionLocation::Default],
    );

    assert_eq!(
        ws.sessions.get(&repo).unwrap().len(),
        1,
        "an already-persisted session must not be duplicated"
    );
}

#[test]
fn no_transcripts_leaves_workspace_sessions_untouched() {
    let config = tempdir().unwrap();
    let repo = PathBuf::from("/repo");
    let mut ws = Workspace::empty();

    reconcile(
        &mut ws,
        &repo,
        &claude_only(config.path()),
        &[SessionLocation::Default],
    );

    assert!(!ws.sessions.contains_key(&repo));
}

// --- Bugfix BUG-003: the marker survives total loss of the app's own store (T069) ---

#[test]
fn a_marked_archived_transcript_is_never_reconstructed_even_with_an_empty_store() {
    let config = tempdir().unwrap();
    let repo = PathBuf::from("/repo");
    let closed_id = Uuid::new_v4();

    write_transcript(config.path(), &repo, closed_id, Some("Closed session"));
    AiCli::ClaudeCode
        .provider()
        .mark_archived(config.path(), &repo, closed_id)
        .unwrap();

    // Simulates total loss of the app's own store: an entirely empty `Workspace`, with no
    // knowledge whatsoever that this session was ever closed. The marker alone must suppress it.
    let mut ws = Workspace::empty();
    reconcile(
        &mut ws,
        &repo,
        &claude_only(config.path()),
        &[SessionLocation::Default],
    );

    assert!(
        !ws.sessions.contains_key(&repo),
        "a session with a durable archived marker must never be reconstructed, \
         regardless of what the app's own store remembers"
    );
}

// ---------------------------------------------------------------------------------------
// Feature 026 (T042/T043) — the rules, per provider
// ---------------------------------------------------------------------------------------

/// Materialise a Copilot conversation: the per-cwd index Copilot keeps for its own picker, plus
/// the session directory whose `events.jsonl` is what "a conversation was recorded" means.
fn write_copilot_conversation(config_dir: &Path, cwd: &Path, ids: &[Uuid]) {
    let hashed = micold_core::protocol::hashing::sha256_hex(cwd.to_string_lossy().as_bytes());
    let index_dir = config_dir.join("sidebar-sessions-state");
    std::fs::create_dir_all(&index_dir).unwrap();
    let listed = ids
        .iter()
        .map(|id| format!("    \"{id}\""))
        .collect::<Vec<_>>()
        .join(",\n");
    std::fs::write(
        index_dir.join(format!("{hashed}.json")),
        format!(
            "{{\n  \"schemaVersion\": 1,\n  \"cwd\": {:?},\n  \"sessionIds\": [\n{listed}\n  ]\n}}\n",
            cwd.to_string_lossy()
        ),
    )
    .unwrap();
    for id in ids {
        let dir = config_dir.join("session-state").join(id.to_string());
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("events.jsonl"), "{}\n").unwrap();
    }
}

fn both_stores(claude: &Path, copilot: &Path) -> Stores {
    Stores::from([
        (AiCli::ClaudeCode, claude.to_path_buf()),
        (AiCli::Copilot, copilot.to_path_buf()),
    ])
}

#[test]
fn a_conversation_started_outside_the_application_is_listed_as_its_own_cli() {
    // FR-014. Two conversations in the same worktree, one per CLI, neither of which this
    // application has any record of. Both are surfaced, and each is assigned the provider whose
    // store it was found in — never a default.
    let claude_home = tempdir().unwrap();
    let copilot_home = tempdir().unwrap();
    let repo = PathBuf::from("/repo");
    let location = SessionLocation::Worktree("feat-x".to_string());
    let cwd = location.cwd(&repo);

    let claude_id = Uuid::from_u128(0xC1);
    let copilot_id = Uuid::from_u128(0xC0);
    write_transcript(claude_home.path(), &cwd, claude_id, Some("From claude"));
    write_copilot_conversation(copilot_home.path(), &cwd, &[copilot_id]);

    let mut ws = Workspace::empty();
    reconcile(
        &mut ws,
        &repo,
        &both_stores(claude_home.path(), copilot_home.path()),
        std::slice::from_ref(&location),
    );

    let sessions = ws.sessions.get(&repo).expect("sessions reconstructed");
    assert_eq!(sessions.len(), 2);
    let provider_of = |id: Uuid| {
        sessions
            .iter()
            .find(|s| s.id.0 == id)
            .map(|s| s.provider)
            .expect("session present")
    };
    assert_eq!(provider_of(claude_id), AiCli::ClaudeCode);
    assert_eq!(
        provider_of(copilot_id),
        AiCli::Copilot,
        "a pass that asked only the first provider would have found one of these two and reported \
         nothing about the other"
    );
}

#[test]
fn a_copilot_session_with_our_marker_is_never_reconstructed() {
    // FR-015, held for the second provider. Its marker lives inside the session directory rather
    // than beside a transcript, so this is not the same code path as Claude Code's — and the
    // sentinel is ours, in Copilot's storage, so it survives the loss of our own store.
    let copilot_home = tempdir().unwrap();
    let repo = PathBuf::from("/repo");
    let cwd = SessionLocation::Default.cwd(&repo);
    let closed = Uuid::from_u128(0xDEAD);
    write_copilot_conversation(copilot_home.path(), &cwd, &[closed]);
    AiCli::Copilot
        .provider()
        .mark_archived(copilot_home.path(), &cwd, closed)
        .unwrap();

    let mut ws = Workspace::empty();
    reconcile(
        &mut ws,
        &repo,
        &Stores::from([(AiCli::Copilot, copilot_home.path().to_path_buf())]),
        &[SessionLocation::Default],
    );

    assert!(
        !ws.sessions.contains_key(&repo),
        "a closed session stays closed, with an entirely empty application store"
    );
}

#[test]
fn a_long_history_is_neither_capped_nor_aged_out() {
    // FR-014 as amended 2026-08-16, and the half of the "long CLI history" edge case that lives on
    // the discovery side (SC-009 covers the rendering side). ~250 recorded conversations is a real
    // number — it is what the development machine holds — not an outlier.
    let copilot_home = tempdir().unwrap();
    let repo = PathBuf::from("/repo");
    let cwd = SessionLocation::Default.cwd(&repo);
    let ids: Vec<Uuid> = (1..=250).map(Uuid::from_u128).collect();
    write_copilot_conversation(copilot_home.path(), &cwd, &ids);

    let mut ws = Workspace::empty();
    reconcile(
        &mut ws,
        &repo,
        &Stores::from([(AiCli::Copilot, copilot_home.path().to_path_buf())]),
        &[SessionLocation::Default],
    );

    assert_eq!(
        ws.sessions.get(&repo).map(Vec::len),
        Some(250),
        "every recorded conversation is surfaced — nothing is dropped by count or by age"
    );
}

#[test]
fn a_second_open_adds_nothing() {
    // Idempotence, and the reason for it: a discovered session's `SessionId` **is** the CLI's own
    // conversation uuid, so the second pass finds it already known and skips it. Discovery runs on
    // every project open (Clarifications 2026-08-18), so this is the ordinary case, not an edge.
    let copilot_home = tempdir().unwrap();
    let repo = PathBuf::from("/repo");
    let cwd = SessionLocation::Default.cwd(&repo);
    let ids: Vec<Uuid> = (1..=3).map(Uuid::from_u128).collect();
    write_copilot_conversation(copilot_home.path(), &cwd, &ids);
    let stores = Stores::from([(AiCli::Copilot, copilot_home.path().to_path_buf())]);

    let mut ws = Workspace::empty();
    reconcile(&mut ws, &repo, &stores, &[SessionLocation::Default]);
    let after_first: Vec<SessionId> = ws.sessions[&repo].iter().map(|s| s.id).collect();

    reconcile(&mut ws, &repo, &stores, &[SessionLocation::Default]);
    let after_second: Vec<SessionId> = ws.sessions[&repo].iter().map(|s| s.id).collect();

    assert_eq!(
        after_first, after_second,
        "a reopen is a no-op, not a duplicate"
    );
}

#[test]
fn a_colliding_id_resolves_by_the_persisted_provider_and_never_by_disk() {
    // The spec's colliding-id edge case, and data-model invariant 3. The same uuid exists in
    // *both* stores — improbable, but the two id spaces are independent, so nothing prevents it —
    // and the application already has a record saying which CLI that session runs.
    //
    // The rule is that the persisted value wins and is never re-derived. The failure it guards is
    // specific: a discovery pass that "corrected" a known session's provider from whichever store
    // it happened to scan last would silently switch a live session's CLI, and the user's next
    // resume would start the wrong tool in their worktree.
    let claude_home = tempdir().unwrap();
    let copilot_home = tempdir().unwrap();
    let repo = PathBuf::from("/repo");
    let cwd = SessionLocation::Default.cwd(&repo);
    let shared = Uuid::from_u128(0xC011DE);

    write_transcript(claude_home.path(), &cwd, shared, Some("Claude's"));
    write_copilot_conversation(copilot_home.path(), &cwd, &[shared]);

    let mut ws = Workspace::empty();
    ws.sessions.insert(
        repo.clone(),
        vec![Session::restored(
            SessionId::from_uuid(shared),
            SessionLocation::Default,
            SessionLabel::Named("The one we know about".to_string()),
            TerminalMode::AiCli,
            AiCli::Copilot,
        )],
    );

    reconcile(
        &mut ws,
        &repo,
        &both_stores(claude_home.path(), copilot_home.path()),
        &[SessionLocation::Default],
    );

    let sessions = &ws.sessions[&repo];
    assert_eq!(sessions.len(), 1, "one id is one session, in both stores");
    assert_eq!(
        sessions[0].provider,
        AiCli::Copilot,
        "the persisted provider survived a pass that saw the same id in the other CLI's store"
    );
    assert_eq!(
        sessions[0].label,
        SessionLabel::Named("The one we know about".to_string()),
        "and nothing else about the known session was re-derived either"
    );
}
