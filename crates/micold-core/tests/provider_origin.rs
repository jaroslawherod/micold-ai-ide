//! Feature 551, US3 (T026): `AiCliProvider::reported_origin` — a provider names the session that
//! started another only from its own native record (FR-001), and never from a name, path or time
//! (FR-002). Every fixture is synthetic, built from the provider's schema or source (see the
//! README beside each).

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use micold_core::provider::{
    AiCliProvider, ClaudeProvider, CodexProvider, ConversationRef, CopilotProvider,
    FakeAiCliProvider, OpenCodeProvider, PiProvider,
};
use uuid::Uuid;

const PARENT_CONVERSATION: &str = "0190aaaa-0000-7000-8000-000000000001";
const CHILD_CONVERSATION: &str = "0190aaaa-0000-7000-8000-000000000002";

fn parent() -> Uuid {
    Uuid::from_u128(0xA1)
}

fn child() -> Uuid {
    Uuid::from_u128(0xA2)
}

fn fixture(provider: &str, name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(provider)
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn conversation(id: &str, cwd: &Path) -> ConversationRef {
    ConversationRef {
        id: id.to_string(),
        cwd: cwd.to_path_buf(),
        created: SystemTime::now(),
    }
}

#[test]
fn a_provider_without_a_reader_reports_nothing_even_when_names_and_paths_suggest_one() {
    // A directory and a transcript that read like a parent link by name, path and time only.
    let base = tempfile::tempdir().unwrap();
    let cwd = PathBuf::from(format!("/work/child-of-{}", parent()));
    let dir = base.path().join("projects").join("-work-child-of-parent");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(format!("{}.jsonl", child())),
        format!("{{\"title\":\"child of {}\"}}\n", parent()),
    )
    .unwrap();
    let providers: [&dyn AiCliProvider; 4] = [
        &ClaudeProvider,
        &CopilotProvider,
        &PiProvider,
        &FakeAiCliProvider::default(),
    ];
    for provider in providers {
        assert_eq!(
            provider.reported_origin(base.path(), &cwd, child()),
            None,
            "{:?}",
            provider.id()
        );
    }
}

#[test]
fn codex_reports_the_session_bound_to_the_spawning_thread() {
    for name in [
        "synthetic-rollout-child-thread-spawn.jsonl",
        "synthetic-rollout-child-parent-field.jsonl",
    ] {
        let codex = CodexHome::new(&fixture("codex", name));
        assert_eq!(
            CodexProvider.reported_origin(codex.base.path(), &codex.cwd, child()),
            Some(parent()),
            "{name}"
        );
    }
}

#[test]
fn codex_without_the_field_or_without_a_known_parent_reports_nothing() {
    let root = CodexHome::new(&fixture("codex", "synthetic-rollout-root.jsonl"));
    assert_eq!(
        CodexProvider.reported_origin(root.base.path(), &root.cwd, child()),
        None,
        "the same rollout without the field; its first turn names a parent and a path, which count for nothing"
    );

    let lone = CodexHome::new(&fixture(
        "codex",
        "synthetic-rollout-child-thread-spawn.jsonl",
    ));
    std::fs::remove_file(
        lone.base
            .path()
            .join("micold-bindings")
            .join(parent().to_string()),
    )
    .unwrap();
    assert_eq!(
        CodexProvider.reported_origin(lone.base.path(), &lone.cwd, child()),
        None,
        "a spawning thread no session is bound to is not guessed at"
    );
    assert_eq!(
        CodexProvider.reported_origin(lone.base.path(), &lone.cwd, Uuid::from_u128(0xFF)),
        None,
        "an unbound session has no rollout to read"
    );
}

/// A Codex home holding the child's rollout (`fixture`), the child bound to it and the parent
/// bound to the conversation the rollout names.
struct CodexHome {
    base: tempfile::TempDir,
    cwd: PathBuf,
}

impl CodexHome {
    fn new(rollout: &str) -> Self {
        let base = tempfile::tempdir().unwrap();
        let cwd = PathBuf::from("/work/tree");
        let day = base.path().join("sessions/2026/10/13");
        std::fs::create_dir_all(&day).unwrap();
        std::fs::write(
            day.join(format!(
                "rollout-2026-10-13T10-00-00-{CHILD_CONVERSATION}.jsonl"
            )),
            rollout,
        )
        .unwrap();
        CodexProvider
            .bind(
                base.path(),
                child(),
                &conversation(CHILD_CONVERSATION, &cwd),
            )
            .unwrap();
        CodexProvider
            .bind(
                base.path(),
                parent(),
                &conversation(PARENT_CONVERSATION, &cwd),
            )
            .unwrap();
        Self { base, cwd }
    }
}

#[cfg(unix)]
mod opencode {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    /// The tests change `PATH`; one at a time.
    static ENV: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// `PATH` holds a stand-in `opencode` whose `export <id>` prints `export.json`.
    struct Cli {
        base: tempfile::TempDir,
        _bin: tempfile::TempDir,
        cwd: PathBuf,
        saved: Option<std::ffi::OsString>,
        _guard: std::sync::MutexGuard<'static, ()>,
    }

    impl Cli {
        fn new(export: &str) -> Self {
            let guard = ENV.lock().unwrap_or_else(|p| p.into_inner());
            let bin = tempfile::tempdir().unwrap();
            std::fs::write(bin.path().join("export.json"), export).unwrap();
            let program = bin.path().join("opencode");
            std::fs::write(
                &program,
                format!(
                    "#!/bin/sh\n[ \"$1\" = export ] && [ \"$2\" = ses_child ] && cat '{}/export.json' && exit 0\nexit 9\n",
                    bin.path().display()
                ),
            )
            .unwrap();
            std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
            let saved = std::env::var_os("PATH");
            std::env::set_var(
                "PATH",
                std::env::join_paths([bin.path(), Path::new("/usr/bin"), Path::new("/bin")])
                    .unwrap(),
            );
            let base = tempfile::tempdir().unwrap();
            let cwd = std::fs::canonicalize(bin.path()).unwrap();
            OpenCodeProvider
                .bind(base.path(), child(), &conversation("ses_child", &cwd))
                .unwrap();
            OpenCodeProvider
                .bind(base.path(), parent(), &conversation("ses_parent", &cwd))
                .unwrap();
            Self {
                base,
                _bin: bin,
                cwd,
                saved,
                _guard: guard,
            }
        }
    }

    impl Drop for Cli {
        fn drop(&mut self) {
            match self.saved.take() {
                Some(path) => std::env::set_var("PATH", path),
                None => std::env::remove_var("PATH"),
            }
        }
    }

    #[test]
    fn opencode_reports_the_session_bound_to_the_parent_id() {
        let cli = Cli::new(&fixture("opencode", "synthetic-export-child.json"));
        assert_eq!(
            OpenCodeProvider.reported_origin(cli.base.path(), &cli.cwd, child()),
            Some(parent())
        );
    }

    #[test]
    fn opencode_without_the_field_or_a_known_parent_reports_nothing() {
        let cli = Cli::new(&fixture("opencode", "synthetic-export-root.json"));
        assert_eq!(
            OpenCodeProvider.reported_origin(cli.base.path(), &cli.cwd, child()),
            None,
            "the same export without the field; its title names a parent, which counts for nothing"
        );

        drop(cli);
        let cli = Cli::new(&fixture("opencode", "synthetic-export-child.json"));
        std::fs::remove_file(
            cli.base
                .path()
                .join("micold-bindings")
                .join(parent().to_string()),
        )
        .unwrap();
        assert_eq!(
            OpenCodeProvider.reported_origin(cli.base.path(), &cli.cwd, child()),
            None
        );
        assert_eq!(
            OpenCodeProvider.reported_origin(cli.base.path(), &cli.cwd, Uuid::from_u128(0xFF)),
            None,
            "an unbound session has nothing to ask about"
        );
    }
}
