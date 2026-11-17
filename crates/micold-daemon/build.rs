//! Sets `micold_postgres` when `MICOLD_TEST_POSTGRES_URL` is set at build time, so the PostgreSQL
//! test cases run; without it they are reported as ignored rather than passing with no assertion.

fn main() {
    println!("cargo::rustc-check-cfg=cfg(micold_postgres)");
    println!("cargo::rerun-if-env-changed=MICOLD_TEST_POSTGRES_URL");
    if std::env::var("MICOLD_TEST_POSTGRES_URL").is_ok_and(|url| !url.trim().is_empty()) {
        println!("cargo::rustc-cfg=micold_postgres");
    }
}
