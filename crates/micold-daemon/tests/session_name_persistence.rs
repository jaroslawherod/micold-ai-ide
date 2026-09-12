//! A session's name is recorded durably, so it is on the row after a restart (feature 029 — US1,
//! US3; FR-001, FR-003, FR-004, FR-005, FR-009, FR-012).
//!
//! # What this file is the gate for
//!
//! `Catalog::record_session_name` is the **only** durable write of a session's name, and this is
//! the only file that exercises it directly. The bug feature 029 fixes was not a persistence bug —
//! the store round-tripped titles all along (`micold-core/tests/session_name_round_trip.rs`) — it
//! was that nothing ever called a mutator. So the assertions here are about the *write*: that it
//! happens, that it is addressed by `SessionId` and reaches exactly one session, that it does not
//! happen when there is nothing to change, and that what it wrote is still there when a fresh
//! `Catalog` loads the same data directory.
//!
//! **The fresh-`Catalog`-over-the-same-directory reload is the in-process stand-in for a daemon
//! restart**, which is the condition the bug report names. It is not the same thing as restarting
//! the process, and this repository has no test that restarts the daemon — quickstart §B1 is the
//! only proof of that, by hand. What the reload does prove is the half that was broken: that the
//! name reached the disk at all.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use micold_core::project::{Availability, Project};
use micold_core::session::{AiCli, Session, SessionId, SessionLabel, SessionLocation, TerminalMode};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;

fn id(n: u128) -> SessionId {
    SessionId::from_uuid(uuid::Uuid::from_u128(n))
}

fn session(n: u128, label: SessionLabel) -> Session {
    Session::restored(
        id(n),
        SessionLocation::Default,
        label,
        TerminalMode::AiCli,
        AiCli::ClaudeCode,
    )
}

/// A catalog over `data_dir` holding `sessions` under each named project.
fn catalog_with(data_dir: &Path, projects: &[(&str, Vec<Session>)]) -> Catalog {
    let mut by_project = BTreeMap::new();
    let mut list = Vec::new();
    for (path, sessions) in projects {
        let path = PathBuf::from(path);
        list.push(Project::new(path.clone(), true, Availability::Available));
        by_project.insert(path, sessions.clone());
    }
    let workspace = Workspace {
        active: list.first().map(|p| p.path.clone()),
        projects: list,
        sessions: by_project,
        ..Default::default()
    };
    let projects_path = data_dir.join("projects.json");
    JsonFileStore::at(projects_path.clone())
        .save(&workspace)
        .unwrap();
    reload(data_dir)
}

/// A **fresh** catalog over the same data directory — what the daemon does when it starts again.
fn reload(data_dir: &Path) -> Catalog {
    Catalog::load(
        Box::new(JsonFileStore::at(data_dir.join("projects.json"))),
        Box::new(JsonFileSettingsStore::at(data_dir.join("settings.json"))),
    )
}

/// Every session's label for `project`, in id order — read from the catalog's own workspace so an
/// archived or otherwise filtered session could not hide a wrong answer.
fn labels(catalog: &Catalog, project: &str) -> Vec<(SessionId, SessionLabel)> {
    let mut out: Vec<(SessionId, SessionLabel)> = catalog
        .workspace()
        .sessions
        .get(Path::new(project))
        .map(|list| list.iter().map(|s| (s.id, s.label.clone())).collect())
        .unwrap_or_default();
    out.sort_by_key(|(id, _)| id.0);
    out
}

fn label_of(catalog: &Catalog, project: &str, session: SessionId) -> SessionLabel {
    labels(catalog, project)
        .into_iter()
        .find(|(id, _)| *id == session)
        .map(|(_, label)| label)
        .expect("session is in the catalog")
}

const P1: &str = "/repo/one";
const P2: &str = "/repo/two";

#[test]
fn a_recorded_name_is_on_disk_and_survives_a_fresh_catalog() {
    let dir = tempfile::tempdir().unwrap();
    let mut catalog = catalog_with(
        dir.path(),
        &[(P1, vec![session(0xA1, SessionLabel::Pending)])],
    );

    assert!(
        catalog
            .record_session_name(id(0xA1), "Fix the flaky login test")
            .unwrap(),
        "a name for a session that had none is a change, and reports itself as one"
    );

    let after_restart = reload(dir.path());
    assert_eq!(
        label_of(&after_restart, P1, id(0xA1)),
        SessionLabel::Named("Fix the flaky login test".into()),
        "the name is in the record a fresh daemon loads — this is the bug feature 029 fixes: \
         before it, every session this application started restored as Pending and read \
         \"New session\" until something ran it again"
    );
}

#[test]
fn three_sessions_keep_three_distinct_names_across_a_reload() {
    // SC-001/SC-006 together: not just "a name survives" but "each session's own name survives".
    // A write addressed by position rather than by `SessionId` (data-model invariant 4) passes the
    // single-session test above and fails this one.
    let dir = tempfile::tempdir().unwrap();
    let mut catalog = catalog_with(
        dir.path(),
        &[(
            P1,
            vec![
                session(0xB1, SessionLabel::Pending),
                session(0xB2, SessionLabel::Pending),
                session(0xB3, SessionLabel::Pending),
            ],
        )],
    );

    for (n, name) in [(0xB1u128, "Parser"), (0xB2, "Renderer"), (0xB3, "Docs")] {
        assert!(catalog.record_session_name(id(n), name).unwrap());
    }

    let after_restart = reload(dir.path());
    assert_eq!(
        labels(&after_restart, P1)
            .into_iter()
            .map(|(_, l)| l)
            .collect::<Vec<_>>(),
        vec![
            SessionLabel::Named("Parser".into()),
            SessionLabel::Named("Renderer".into()),
            SessionLabel::Named("Docs".into()),
        ],
        "each row keeps its own name; none reads \"New session\" and none wears another's"
    );
}

#[test]
fn recording_the_name_a_session_already_has_writes_nothing() {
    // The compare-before-write guard (contract C2, SC-007). `Catalog::persist` rewrites the file
    // holding *every one* of that project's session records, and a reconnect, a re-attach or a
    // restart all re-observe a title the catalog already holds — so writing unconditionally would
    // rewrite that file on every one of them. `remember_foreground` carries the same guard for the
    // same reason.
    let dir = tempfile::tempdir().unwrap();
    let mut catalog = catalog_with(
        dir.path(),
        &[(P1, vec![session(0xC1, SessionLabel::Pending)])],
    );
    assert!(catalog.record_session_name(id(0xC1), "Same name").unwrap());

    let written = record_mtime_probe(dir.path());
    assert!(
        !catalog.record_session_name(id(0xC1), "Same name").unwrap(),
        "the name is already recorded: nothing changed, so nothing is written and the call says so"
    );
    assert_eq!(
        record_mtime_probe(dir.path()),
        written,
        "and the file is byte-identical — the report of 'no write' is not just a return value"
    );
}

#[test]
fn an_empty_name_and_an_unknown_id_are_no_ops() {
    let dir = tempfile::tempdir().unwrap();
    let mut catalog = catalog_with(
        dir.path(),
        &[(P1, vec![session(0xD1, SessionLabel::Pending)])],
    );

    assert!(
        !catalog.record_session_name(id(0xD1), "").unwrap(),
        "an empty name is not a name (FR-004): the session keeps its placeholder rather than \
         acquiring a Named(\"\") label, which the type system cannot forbid but this can"
    );
    assert_eq!(label_of(&catalog, P1, id(0xD1)), SessionLabel::Pending);

    assert!(
        !catalog.record_session_name(id(0xDEAD), "Ghost").unwrap(),
        "an id the catalog does not know is not an error — the live registry and the catalog can \
         disagree for a tick after a session is removed"
    );
    assert_eq!(
        labels(&catalog, P1).len(),
        1,
        "and no session was invented to hold the name"
    );
}

#[test]
fn a_name_reaches_exactly_one_session_in_one_project() {
    // FR-012: a name belongs to exactly one session. The second project is here because
    // `find_session_mut` searches across all of them — a lookup that stopped at the first project
    // holding *a* session, or that matched on anything but the id, would show up here.
    let dir = tempfile::tempdir().unwrap();
    let mut catalog = catalog_with(
        dir.path(),
        &[
            (
                P1,
                vec![
                    session(0xE1, SessionLabel::Pending),
                    session(0xE2, SessionLabel::Named("Untouched".into())),
                ],
            ),
            (P2, vec![session(0xE3, SessionLabel::Pending)]),
        ],
    );

    assert!(catalog.record_session_name(id(0xE3), "Only mine").unwrap());

    let after_restart = reload(dir.path());
    assert_eq!(
        labels(&after_restart, P1),
        vec![
            (id(0xE1), SessionLabel::Pending),
            (id(0xE2), SessionLabel::Named("Untouched".into())),
        ],
        "the other project's sessions are exactly as they were — neither given the name nor \
         stripped of their own"
    );
    assert_eq!(
        label_of(&after_restart, P2, id(0xE3)),
        SessionLabel::Named("Only mine".into())
    );
}

#[test]
fn a_catalog_that_cannot_persist_still_shows_the_name_and_reports_no_failure() {
    // FR-009 / contract C5. An ephemeral catalog reaches the same branch a read-only data directory
    // would, with no filesystem permissions involved — which is why this is not a `chmod` test: it
    // runs identically on Windows (research R8).
    //
    // What is being asserted is a *posture*: a storage problem is not a session problem. The name
    // is in memory, the client sees it, and nothing about the session's lifecycle changes. The
    // supervisor's caller logs at `warn` and carries on — the convention `adopt_discovered_sessions`
    // already follows.
    let mut catalog = Catalog::ephemeral();
    catalog.adopt_discovered_sessions(Path::new(P1), vec![session(0xF1, SessionLabel::Pending)]);

    let wrote = catalog
        .record_session_name(id(0xF1), "Named without a disk")
        .expect("an ephemeral catalog's persist is a no-op, not an error");
    assert!(wrote, "the label changed, so the call reports a change");
    assert_eq!(
        label_of(&catalog, P1, id(0xF1)),
        SessionLabel::Named("Named without a disk".into()),
        "the in-memory label is updated first and stays updated — a failed write changes nothing \
         the user can see (FR-009)"
    );
}

// --- US3: the name stays true to the conversation (T017, T018) ---

#[test]
fn a_second_name_replaces_the_first_and_the_old_one_is_gone() {
    // FR-005, SC-005. Remembering a name must not mean freezing it: a conversation that is
    // re-titled has a new name, and the record follows. First-write-wins would pass every other
    // test in this file.
    let dir = tempfile::tempdir().unwrap();
    let mut catalog = catalog_with(
        dir.path(),
        &[(P1, vec![session(0x51, SessionLabel::Pending)])],
    );

    assert!(catalog.record_session_name(id(0x51), "Fix the parser").unwrap());
    assert!(
        catalog
            .record_session_name(id(0x51), "Write the release notes")
            .unwrap(),
        "a different name is a change, however recently the previous one was recorded"
    );

    let after_restart = reload(dir.path());
    assert_eq!(
        label_of(&after_restart, P1, id(0x51)),
        SessionLabel::Named("Write the release notes".into())
    );
    let on_disk = std::fs::read_dir(dir.path())
        .unwrap()
        .flatten()
        .filter_map(|e| std::fs::read_to_string(e.path()).ok())
        .collect::<String>();
    assert!(
        !on_disk.contains("Fix the parser"),
        "the superseded name is not left behind anywhere for a later read to resurrect"
    );
}

#[test]
fn a_never_named_session_stays_pending_beside_a_named_one() {
    // FR-004 + FR-012 together. "New session" must stay a truthful statement about a conversation
    // that has never been named — this feature makes names sticky, and a sticky name that leaked
    // onto the wrong row would be worse than the bug it fixes.
    let dir = tempfile::tempdir().unwrap();
    let mut catalog = catalog_with(
        dir.path(),
        &[(
            P1,
            vec![
                session(0x61, SessionLabel::Pending),
                session(0x62, SessionLabel::Pending),
            ],
        )],
    );
    assert!(catalog.record_session_name(id(0x62), "Has a name").unwrap());

    let after_restart = reload(dir.path());
    assert_eq!(
        label_of(&after_restart, P1, id(0x61)),
        SessionLabel::Pending,
        "the untouched session is still Pending after a reload — it did not inherit its \
         neighbour's name, and it was not written as Named(\"\")"
    );
    assert_eq!(
        label_of(&after_restart, P1, id(0x61)).display(),
        "New session"
    );
    assert_eq!(
        label_of(&after_restart, P1, id(0x62)),
        SessionLabel::Named("Has a name".into())
    );
}

/// The bytes of every JSON file under `data_dir`, so "nothing was written" can be asserted as
/// content rather than as a return value. Content, not mtime: a filesystem's timestamp resolution
/// is coarse enough that two writes in the same test can share one.
fn record_mtime_probe(data_dir: &Path) -> String {
    let mut queue = vec![data_dir.to_path_buf()];
    let mut out = Vec::new();
    while let Some(dir) = queue.pop() {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .collect();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                queue.push(path);
            } else if let Ok(text) = std::fs::read_to_string(&path) {
                out.push(format!("{}\n{text}", path.display()));
            }
        }
    }
    out.join("\n")
}
