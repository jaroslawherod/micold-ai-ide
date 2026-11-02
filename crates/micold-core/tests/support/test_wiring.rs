//! `Cargo.toml` turns off test auto-discovery (`autotests = false`), so a file added under `tests/` that is neither
//! a `mod` of `tests/main.rs` nor a `[[test]]` target would compile nowhere and run nowhere, silently (#1631).

use std::path::Path;

/// Panics naming every `tests/*.rs` (and every `tests/<dir>/main.rs`) of the crate at `crate_dir` that no target runs.
pub fn assert_wired(crate_dir: &str) {
    let dir = Path::new(crate_dir);
    let tests = dir.join("tests");
    let main_rs = std::fs::read_to_string(tests.join("main.rs")).expect("tests/main.rs");
    let manifest = std::fs::read_to_string(dir.join("Cargo.toml")).expect("Cargo.toml");
    let mut unwired = Vec::new();
    for entry in std::fs::read_dir(&tests).expect("tests/").flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let (stem, path) = match name.strip_suffix(".rs") {
            Some("main") => continue,
            Some(stem) => (stem.to_owned(), format!("tests/{name}")),
            None if entry.path().join("main.rs").exists() => {
                (String::new(), format!("tests/{name}/main.rs"))
            }
            None => continue,
        };
        let a_module = !stem.is_empty()
            && main_rs
                .lines()
                .any(|line| line.trim() == format!("mod {stem};"));
        let a_target = manifest.contains(&format!("path = \"{path}\""));
        if !a_module && !a_target {
            unwired.push(path);
        }
    }
    assert!(
        unwired.is_empty(),
        "not run by any test target: {unwired:?}. Declare each as a `mod` in tests/main.rs, or add a [[test]] to Cargo.toml"
    );
}
