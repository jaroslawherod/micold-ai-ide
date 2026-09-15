//! Embeds the project icon in the Windows exe (feature 030, FR-003, research R11).

fn main() {
    let rc = "../../packaging/windows/app-icon.rc";
    println!("cargo:rerun-if-changed={rc}");
    println!("cargo:rerun-if-changed=../../assets/icon/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    match embed_resource::compile(rc, embed_resource::NONE) {
        // A cross-check from a host with no resource compiler still type-checks. The packaging
        // smoke fails an exe that shipped without its icon.
        result @ embed_resource::CompilationResult::NotAttempted(_) => {
            println!("cargo:warning={result}; the exe has no icon")
        }
        result => result.manifest_required().unwrap(),
    }
}
