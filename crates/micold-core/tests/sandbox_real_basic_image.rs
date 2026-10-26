//! FR-002 against the built image: the basic image carries an SSH server and what a session needs,
//! and neither the session daemon nor any AI CLI (feature 806).
//!
//! The client copies its own daemon build in over SSH and installs the AI CLIs the user selected,
//! so a daemon or CLI baked into the image would be one the client did not choose, of a version it
//! did not build. `node`/`npm` and `flock` are what those installs need; glibc 2.41 is the floor the
//! glibc daemon builds link against (trixie).
//!
//! And what a session builds with before any setup (#1439, FR-001, FR-003, FR-004, FR-017): the
//! pinned `mise`, a C/C++ toolchain with `pkg-config`, `python3`, and `xz`/`unzip` for installers,
//! as root and as the Kubernetes pod's uid 10001 on a read-only root file system.
//!
//! The image is `MICOLD_BASIC_IMAGE_TAG`, default `micold-basic:dev` (`docker build -f
//! packaging/basic/Containerfile -t micold-basic:dev packaging/basic`). Behind
//! `sandbox-real-runtime` for the reason every real-runtime target here is: the default suite must
//! need nothing installed on any platform (Principle VI).

#![cfg(feature = "sandbox-real-runtime")]

use std::process::Command;

use micold_core::sandbox::basic_image::DEV_BASIC_IMAGE;

/// What must not be in the image: the daemon and every AI CLI the application offers.
/// `gemini` is not offered today; it is listed because the spec names it.
const ABSENT: [&str; 7] = [
    "micold-daemon",
    "claude",
    "copilot",
    "codex",
    "gemini",
    "opencode",
    "pi",
];

/// What must be: the SSH server, the runtime the npm-distributed CLIs install with, the tool the
/// daemon runs for worktrees, the lock the install step serialises on, `git-lfs`, which the
/// shared host gitconfig's `filter.lfs` block requires (#1288), and what a session builds with
/// before any setup (#1439): `mise`, the C/C++ toolchain, `pkg-config`, `python3`, `xz`, `unzip`.
const PRESENT: [&str; 14] = [
    "sshd",
    "node",
    "npm",
    "git",
    "flock",
    "git-lfs",
    "mise",
    "cc",
    "c++",
    "make",
    "pkg-config",
    "python3",
    "xz",
    "unzip",
];

/// The `pkg-config` modules of the X11/Wayland development libraries this repository's gate links
/// (#1439, FR-007): `libxkbcommon-dev`, `libwayland-dev`, `libx11-dev`, `libxcursor-dev`,
/// `libxrandr-dev`, `libxi-dev`.
const DEV_PKG_CONFIG: [&str; 6] = [
    "xkbcommon",
    "wayland-client",
    "x11",
    "xcursor",
    "xrandr",
    "xi",
];

/// The oldest glibc the image may carry: trixie's, which the daemon's glibc builds need.
const MIN_GLIBC: (u32, u32) = (2, 41);

fn image() -> String {
    std::env::var("MICOLD_BASIC_IMAGE_TAG").unwrap_or_else(|_| DEV_BASIC_IMAGE.to_string())
}

/// Run `script` with `sh -c` inside a throwaway container of the image, past its entrypoint.
fn in_image(script: &str) -> std::process::Output {
    in_image_with(&[], script)
}

/// [`in_image`] with `docker run` options (`--user`, `--read-only`, ...) before the image.
fn in_image_with(options: &[&str], script: &str) -> std::process::Output {
    let tag = image();
    let mut args = vec!["run", "--rm", "--entrypoint", "/bin/sh"];
    args.extend_from_slice(options);
    args.extend_from_slice(&[&tag, "-c", script]);
    let out = Command::new("docker")
        .args(&args)
        .output()
        .expect("docker runs");
    assert!(
        out.status.success() || !out.stdout.is_empty(),
        "docker run {tag} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out
}

/// The commands of `names` that `command -v` finds in the image, one per line.
fn found(names: &[&str]) -> Vec<String> {
    let script = names
        .iter()
        .map(|n| format!("command -v {n} >/dev/null 2>&1 && echo {n}"))
        .collect::<Vec<_>>()
        .join("; ");
    let out = in_image(&format!("{script}; true"));
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_string)
        .collect()
}

/// The mise version `packaging/basic/mise.sha256` pins: the text between `mise-v` and `-linux-` of
/// its first line, so a bump needs no change here.
fn pinned_mise_version() -> String {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../packaging/basic/mise.sha256"
    );
    let pins = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let first = pins.lines().next().unwrap_or_default();
    first
        .split_once("  mise-v")
        .and_then(|(_, name)| name.split_once("-linux-"))
        .map(|(version, _)| version.to_string())
        .unwrap_or_else(|| panic!("no mise version in {path}: {first:?}"))
}

/// Builds and runs a C and a C++ program in `$WORK` and runs `python3`, after printing
/// `mise --version`. `cc` writes its temporary files to `TMPDIR`, which the read-only case sets.
const TOOLCHAIN_SCRIPT: &str = r#"set -e
cd "$WORK"
mise --version
printf '#include <stdio.h>\nint main(void) { puts("c ok"); return 0; }\n' > t.c
cc -o t t.c
./t
printf '#include <iostream>\nint main() { std::cout << "c++ ok" << std::endl; }\n' > t.cpp
c++ -o tpp t.cpp
./tpp
python3 -c 'print(1)'
"#;

/// [`TOOLCHAIN_SCRIPT`] in the image with `options`: what it printed, after asserting it succeeded.
fn toolchain_output(options: &[&str]) -> String {
    let out = in_image_with(options, TOOLCHAIN_SCRIPT);
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        out.status.success(),
        "the toolchain script failed in {} with {options:?}: {stdout}{}",
        image(),
        String::from_utf8_lossy(&out.stderr)
    );
    stdout
}

/// `(major, minor)` from the first line of `ldd --version`, which ends in the version.
fn glibc_version(ldd: &str) -> Option<(u32, u32)> {
    let first = ldd.lines().next()?;
    let version = first.split_whitespace().last()?;
    let (major, minor) = version.split_once('.')?;
    Some((major.parse().ok()?, minor.parse().ok()?))
}

#[test]
fn sandbox_real_basic_image_has_no_daemon_or_cli() {
    let present = found(&ABSENT);
    assert!(
        present.is_empty(),
        "{} carries what the client installs itself: {present:?}",
        image()
    );

    let present = found(&PRESENT);
    let missing: Vec<_> = PRESENT
        .iter()
        .filter(|n| !present.iter().any(|p| p == *n))
        .collect();
    assert!(missing.is_empty(), "{} lacks {missing:?}", image());

    let ldd = in_image("ldd --version 2>&1");
    let ldd = String::from_utf8_lossy(&ldd.stdout);
    let version = glibc_version(&ldd).unwrap_or_else(|| panic!("no glibc version in: {ldd}"));
    assert!(version >= MIN_GLIBC, "glibc {version:?} < {MIN_GLIBC:?}");
}

#[test]
fn sandbox_real_basic_image_builds_before_any_setup() {
    let version = pinned_mise_version();
    let cases: [(&str, &[&str]); 2] = [
        ("as root", &["-e", "WORK=/tmp"]),
        // The Kubernetes pod (feature 686): uid 10001, a read-only root file system, and the
        // daemon's volume as the only writable home (FR-006). Docker mounts a tmpfs `noexec`
        // by default; the pod's volume is not, so the tmpfs asks for `exec`.
        (
            "as uid 10001 with --read-only",
            &[
                "--user",
                "10001:0",
                "--read-only",
                "--tmpfs",
                "/data/home:mode=1777,exec",
                "-e",
                "HOME=/data/home",
                "-e",
                "TMPDIR=/data/home",
                "-e",
                "WORK=/data/home",
            ],
        ),
    ];
    for (name, options) in cases {
        let stdout = toolchain_output(options);
        let lines: Vec<&str> = stdout.lines().map(str::trim).collect();
        assert_eq!(
            lines.first().and_then(|l| l.split_whitespace().next()),
            Some(version.as_str()),
            "{name}: `mise --version` does not print the pinned {version}: {stdout}"
        );
        for want in ["c ok", "c++ ok", "1"] {
            assert!(
                lines.contains(&want),
                "{name}: no {want:?} line in: {stdout}"
            );
        }
    }
}

#[test]
fn sandbox_real_basic_image_pkg_config_finds_the_gate_libraries() {
    let script = format!(
        "set -e; for m in {}; do pkg-config --exists \"$m\" || {{ echo \"missing $m\"; exit 1; }}; done; pkg-config --libs x11",
        DEV_PKG_CONFIG.join(" ")
    );
    let cases: [(&str, &[&str]); 2] = [
        ("as root", &[]),
        (
            "as uid 10001 with --read-only",
            &["--user", "10001:0", "--read-only"],
        ),
    ];
    for (name, options) in cases {
        let out = in_image_with(options, &script);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            out.status.success(),
            "{name}: pkg-config does not find {DEV_PKG_CONFIG:?} in {}: {stdout}{}",
            image(),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            stdout.split_whitespace().any(|flag| flag == "-lX11"),
            "{name}: `pkg-config --libs x11` does not print -lX11: {stdout}"
        );
    }
}
