//! #1439, Story 5 (quickstart A10): on a container daemon on the basic image with **no setup
//! command**, a session installs the build tools it needs itself, as the non-root session user, into
//! the sandbox home, and they stay.
//!
//! One test, in the order a user meets it:
//!
//! 1. in a session terminal: mise installs Rust (minimal profile), uv, Go and Python into the home;
//!    `npm install -g` of a local package, `cargo install --path` of the A7 fixture crate,
//!    `uv tool install` of a pinned package, `go install` of a local module and `pip install
//!    --user` of a pinned package under the mise Python; nothing is written outside the home and
//!    the project (FR-014, FR-017), and `apt-get install` fails (AS5.6);
//! 2. a new session in a second worktree finds every binary under the home, `npm`, `cargo` and `go`
//!    installs in `~/.local/bin` (FR-015, contract sandbox-install-env I2), and each runs;
//! 3. the same after a container restart, after the container is replaced by one of a second tag
//!    of the same image (a `DifferentImage` replacement), and after a last replacement onto the
//!    no-outbound network posture, where every tool runs offline, so nothing was downloaded again
//!    (FR-016, SC-006, AS5.3, AS5.4).
//!
//! Behind `sandbox-real-runtime` and needs the network (mise, npm, uv, pip and go download).
//!
//! ```text
//! cargo test --release -p micold-daemon --features sandbox-real-runtime --test sandbox_real_self_install -- --nocapture
//! ```

#![cfg(all(feature = "sandbox-real-runtime", unix))]

mod sandbox_real_support;

use std::path::Path;
use std::time::Duration;

use micold_core::session::{SessionId, SessionLocation, TerminalMode};
use sandbox_real_support::{
    cli, git, image, input_serial, open_session, repo, runtime_cmd, seed_sessions,
    start_no_outbound_sandbox, start_sandbox, start_sandbox_from, wait_for_accept, written_outside,
    SandboxSpec, Terminal,
};

const CONTAINER: &str = "micold-self-install-probe";
const NETWORK: &str = "micold-self-install-probe-net";
const PORT: u16 = 17792;

/// A second name for the basic image: a container of it is a replacement for another image tag.
const SECOND_TAG: &str = "localhost/micold-self-install-probe:second";

/// Pinned for the reason the other probes pin it: an inherited `SHELL` is the developer's.
const SESSION_SHELL: &str = "/bin/sh";

/// One install on a CI runner takes well under this; the bound only stops a hang.
const INSTALL_BOUND: Duration = Duration::from_secs(15 * 60);

/// The second worktree, where a new session looks for what the first one installed.
const SECOND: &str = "second";

/// What a session runs to install its tools, in order: each `(what, command)` runs in the project.
const INSTALLS: &[(&str, &str)] = &[
    // `mise use` has no profile flag, so the minimal profile goes in the global config as a tool
    // option, as mise's Rust page gives it. The entry is added to whatever the config already
    // holds (under its `[tools]` table, or a new one), never written over it, and left alone when
    // the config already names rust.
    (
        "rust through mise",
        r#"f="$HOME/.config/mise/config.toml" && mkdir -p "${f%/*}" && touch "$f" && rust='rust = { version = "1.90.0", profile = "minimal" }' && { grep -q '^rust *=' "$f" || if grep -q '^\[tools\]' "$f"; then sed -i "/^\[tools\]/a $rust" "$f"; else printf '\n[tools]\n%s\n' "$rust" >> "$f"; fi; } && mise install rust"#,
    ),
    ("uv through mise", "mise use -g uv@0.8.0"),
    ("go through mise", "mise use -g go@1.24.0"),
    ("python through mise", "mise use -g python@3.12.8"),
    (
        "npm install -g",
        "cd npm-probe && npm pack --silent && npm install -g ./self-install-probe-1.0.0.tgz",
    ),
    ("cargo install", "cargo install --path crate"),
    ("uv tool install", "uv tool install pycodestyle==2.12.1"),
    ("go install", "cd go-probe && go install ."),
    (
        "pip install --user",
        "python -m pip install --user pyflakes==3.2.0",
    ),
];

/// A tool a session installed: its command, how to run it, what that prints, and whether it lands
/// in `~/.local/bin` itself (the rest are found through mise's shims, also in the home).
struct Tool {
    command: &'static str,
    run: &'static str,
    prints: &'static str,
    local_bin: bool,
}

const TOOLS: &[Tool] = &[
    Tool {
        command: "self-install-probe",
        run: "self-install-probe",
        prints: "npm-probe-ok",
        local_bin: true,
    },
    Tool {
        command: "mise-c-crate",
        run: "mise-c-crate",
        prints: "answer 42",
        local_bin: true,
    },
    Tool {
        command: "go-probe",
        run: "go-probe",
        prints: "go-probe-ok",
        local_bin: true,
    },
    Tool {
        command: "pyflakes",
        run: "pyflakes --version",
        prints: "3.2.0",
        local_bin: true,
    },
    Tool {
        command: "pycodestyle",
        run: "pycodestyle --version",
        prints: "2.12.1",
        local_bin: true,
    },
    Tool {
        command: "uv",
        run: "uv --version",
        prints: "uv 0.8.0",
        local_bin: false,
    },
    Tool {
        command: "cargo",
        run: "cargo --version",
        prints: "cargo 1.90.0",
        local_bin: false,
    },
    Tool {
        command: "go",
        run: "go version",
        prints: "go1.24.0",
        local_bin: false,
    },
    Tool {
        command: "python",
        run: "python --version",
        prints: "Python 3.12.8",
        local_bin: false,
    },
];

/// The project at `project`, made from `sources`: the A7 fixture crate without its `mise.toml` (so
/// the global config picks the toolchain, in every worktree), a local npm package and a Go module
/// with no dependencies, committed, and a second worktree of it.
fn project_files(sources: &Path, project: &Path) {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mise-c-crate");
    std::fs::create_dir_all(sources.join("crate")).unwrap();
    sandbox_real_support::copy_tree(&fixture, &sources.join("crate"));
    std::fs::remove_file(sources.join("crate/mise.toml")).unwrap();
    let write = |path: &str, body: &str| {
        let path = sources.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    };
    write(
        "npm-probe/package.json",
        r#"{"name":"self-install-probe","version":"1.0.0","bin":{"self-install-probe":"cli.js"}}"#,
    );
    write(
        "npm-probe/cli.js",
        "#!/usr/bin/env node\nconsole.log('npm-probe-ok');\n",
    );
    write(
        "go-probe/go.mod",
        "module example.invalid/go-probe\n\ngo 1.24\n",
    );
    write(
        "go-probe/main.go",
        "package main\n\nimport \"fmt\"\n\nfunc main() { fmt.Println(\"go-probe-ok\") }\n",
    );
    repo(project, Some(sources));
    let worktree = format!(".claude/worktrees/{SECOND}");
    git(project, &["worktree", "add", "-q", "-b", SECOND, &worktree]);
}

/// Run `cmd` in `dir`; fail with the tail of its output unless it succeeds. The success word is
/// typed split, so the shell's echo of the command cannot match it.
async fn install(term: &mut Terminal<'_>, dir: &Path, what: &str, cmd: &str) {
    let out = term
        .run_within(
            &format!(
                "(cd '{}' && {cmd}) >/tmp/self-install.log 2>&1 && echo IN\"STALL-OK\" \
                 || {{ tail -n 20 /tmp/self-install.log; echo INSTALL-FAILED; }}",
                dir.display()
            ),
            INSTALL_BOUND,
        )
        .await;
    assert!(
        out.lines().any(|l| l.trim() == "INSTALL-OK"),
        "{what} as the session user: {out}"
    );
}

/// Every tool of [`TOOLS`] is found under `sandbox_home` and runs, `when`.
async fn assert_tools(term: &mut Terminal<'_>, sandbox_home: &str, when: &str) {
    for tool in TOOLS {
        let found = term.run(&format!("command -v {}", tool.command)).await;
        let found = found.trim();
        if tool.local_bin {
            assert_eq!(
                found,
                format!("{sandbox_home}/.local/bin/{}", tool.command),
                "{} is not in the home's .local/bin {when}",
                tool.command
            );
        } else {
            assert!(
                found.starts_with(&format!("{sandbox_home}/")),
                "{} is not found under the sandbox home {sandbox_home} {when}: {found}",
                tool.command
            );
        }
        let ran = term.run_within(tool.run, Duration::from_secs(60)).await;
        assert!(
            ran.contains(tool.prints),
            "`{}` {when} printed no {:?}: {ran}",
            tool.run,
            tool.prints
        );
    }
}

/// Connect, start the worktree session and check every tool, `when`.
async fn check_in_second_worktree(
    project: &Path,
    session: SessionId,
    log: &Path,
    sandbox_home: &str,
    when: &str,
) {
    let (mut conn, catalog) = wait_for_accept(PORT).await;
    let serial = input_serial(&catalog, session);
    let screen = open_session(&mut conn, project, session, log).await;
    let mut term = Terminal::new(&mut conn, session, screen, CONTAINER, log, serial);
    let at = term.run("pwd").await;
    assert!(
        at.contains(&format!(".claude/worktrees/{SECOND}")),
        "the session is not in the second worktree: {at}"
    );
    assert_tools(&mut term, sandbox_home, when).await;
}

#[tokio::test]
async fn sandbox_real_self_install_a_session_installs_its_tools_and_they_stay() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let projects = dir.path().join("projects");
    let project = projects.join("self-install");
    project_files(&dir.path().join("sources"), &project);
    let sessions = seed_sessions(
        &data,
        &[(&project, true)],
        &[
            (SessionLocation::Default, "self-install-probe"),
            (
                SessionLocation::Worktree(SECOND.into()),
                "self-install-second",
            ),
        ],
        TerminalMode::Regular,
    );
    let (first, second) = (sessions[0], sessions[1]);
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());

    // SAFETY: set before any spawn in this single-test binary.
    std::env::set_var("SHELL", SESSION_SHELL);

    let spec = SandboxSpec {
        container: CONTAINER,
        network: NETWORK,
        port: PORT,
        data_home: &data,
        project: &projects,
        home: &home,
        survive_logout: false,
        extra: &[],
    };
    let sandbox = start_sandbox(&spec);
    let log = data.join("micold-ai-ide").join("micold-daemon.log");

    // 1. The installs, in the project's own session, with no setup command.
    let sandbox_home = {
        let (mut conn, catalog) = wait_for_accept(PORT).await;
        let serial = input_serial(&catalog, first);
        let screen = open_session(&mut conn, &project, first, &log).await;
        let mut term = Terminal::new(&mut conn, first, screen, CONTAINER, &log, serial);

        let uid = term.run("id -u").await;
        assert_ne!(uid.trim(), "0", "the session user is root");
        let sandbox_home = term
            .run("printf '%s\\n' \"$HOME\"")
            .await
            .trim()
            .to_string();
        let marker = "$HOME/.self-install-probe-marker";
        term.run(&format!("touch \"{marker}\"")).await;

        for (what, cmd) in INSTALLS {
            install(&mut term, &project, what, cmd).await;
        }

        let outside = written_outside(&mut term, &sandbox_home, &projects, marker).await;
        assert!(
            outside.is_empty(),
            "the session's installs wrote outside the sandbox home and the projects:\n{outside:#?}"
        );
        // AS5.6: system packages are out of a session's reach.
        let apt = term
            .run_within(
                "apt-get install -y libpq-dev >/tmp/apt.log 2>&1; echo APT\"-EXIT=$?\"",
                Duration::from_secs(120),
            )
            .await;
        assert!(
            apt.lines().any(|l| l
                .trim()
                .strip_prefix("APT-EXIT=")
                .and_then(|code| code.trim().parse::<u32>().ok())
                .is_some_and(|code| code != 0)),
            "apt-get install succeeded as the session user: {apt}"
        );
        sandbox_home
    };

    // 2. A new session in a second worktree.
    check_in_second_worktree(
        &project,
        second,
        &log,
        &sandbox_home,
        "in a new session in a second worktree",
    )
    .await;

    // 3. A restart, then two replacements: another tag of the image, then the no-outbound posture.
    cli(&["restart", "-t", "1", CONTAINER]);
    check_in_second_worktree(&project, second, &log, &sandbox_home, "after a restart").await;

    cli(&["tag", &image(), SECOND_TAG]);
    // Dropping a `Sandbox` removes its container: drop the old one before its replacement exists.
    drop(sandbox);
    let sandbox = start_sandbox_from(&spec, SECOND_TAG, &[]);
    check_in_second_worktree(
        &project,
        second,
        &log,
        &sandbox_home,
        "after a replacement for another image tag",
    )
    .await;

    drop(sandbox);
    let _ = runtime_cmd(sandbox_real_support::dialect().program)
        .args(["rmi", SECOND_TAG])
        .output();
    let _sandbox = start_no_outbound_sandbox(&spec);
    check_in_second_worktree(
        &project,
        second,
        &log,
        &sandbox_home,
        "after a replacement onto the no-outbound posture (offline)",
    )
    .await;
}
