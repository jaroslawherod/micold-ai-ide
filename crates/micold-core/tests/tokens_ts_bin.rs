//! The `micold-tokens-ts` binary prints or writes exactly `tokens::ts::module()` and decides
//! nothing itself (issue #1950, FR-005; contract §6).

use std::process::{Command, Output};

fn run(args: &[&std::ffi::OsStr]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_micold-tokens-ts"))
        .args(args)
        .output()
        .expect("the binary runs")
}

#[test]
fn a_path_argument_writes_the_module_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tokens.generated.ts");
    let out = run(&[path.as_os_str()]);
    assert!(out.status.success(), "{out:?}");
    assert!(out.stdout.is_empty(), "nothing goes to stdout");
    assert!(
        std::fs::read(&path).unwrap() == micold_core::tokens::ts::module().into_bytes(),
        "the file is not `module()`"
    );
    let leftovers: Vec<_> = std::fs::read_dir(dir.path()).unwrap().collect();
    assert_eq!(leftovers.len(), 1, "no temp file is left beside the output");
}

#[test]
fn no_argument_prints_the_module_to_stdout() {
    let out = run(&[]);
    assert!(out.status.success(), "{out:?}");
    assert!(
        out.stdout == micold_core::tokens::ts::module().into_bytes(),
        "stdout is not `module()`"
    );
}

#[test]
fn an_unwritable_path_exits_1_naming_it_and_leaves_no_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("no-such-dir").join("tokens.generated.ts");
    let out = run(&[path.as_os_str()]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains(path.to_str().unwrap()),
        "stderr names the path: {stderr}"
    );
    assert!(!path.exists());
    assert!(
        !dir.path().join("no-such-dir").exists(),
        "no directory or temp file is created"
    );
}
