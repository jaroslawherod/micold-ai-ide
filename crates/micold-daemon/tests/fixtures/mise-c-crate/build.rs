//! Compiles `hello.c` with the image's `cc` and archives it with `ar`, then links the archive:
//! what the `cc` crate does, without a crates.io dependency.

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn run(cmd: &mut Command) {
    let status = cmd.status().unwrap_or_else(|e| panic!("{cmd:?}: {e}"));
    assert!(status.success(), "{cmd:?} failed: {status}");
}

fn main() {
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let object = out.join("hello.o");
    run(Command::new("cc")
        .args(["-c", "-fPIC", "hello.c", "-o"])
        .arg(&object));
    run(Command::new("ar")
        .arg("crs")
        .arg(out.join("libhello.a"))
        .arg(&object));
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=hello");
    println!("cargo:rerun-if-changed=hello.c");
}
