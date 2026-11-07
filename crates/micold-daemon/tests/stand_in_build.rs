//! The shared stand-in build compiles once and reuses the result (#1761).

#[path = "support/stand_in_build.rs"]
mod stand_in_build;

use std::sync::atomic::Ordering;

#[test]
fn building_twice_yields_one_path_and_one_rustc_run() {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("a clock after 1970")
        .as_nanos();
    let source = format!(
        "fn main() {{ println!(\"{}-{nanos}\"); }}\n",
        std::process::id()
    );
    let first = stand_in_build::stand_in_dir(&source, "claude");
    let second = stand_in_build::stand_in_dir(&source, "claude");
    assert_eq!(first, second);
    assert!(first.join("claude").exists());
    assert_eq!(stand_in_build::RUSTC_RUNS.load(Ordering::SeqCst), 1);
    let _ = std::fs::remove_dir_all(&first);
}
