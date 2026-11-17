//! Write the client API's JSON Schema (#1732, FR-009).
//!
//! Deliberately trivial, like `micold-tokens-ts`: it writes what `client_api::schema::document()`
//! returns and decides nothing. With a path it writes that file atomically (a temp file beside it,
//! then a rename); with no argument it prints to stdout. A failed write exits 1 and names the path.
//! `scripts/api-types.sh` runs it and then generates the TypeScript types from the file.

use std::ffi::OsString;
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let document = micold_core::client_api::schema::document();
    let Some(path) = std::env::args_os().nth(1) else {
        print!("{document}");
        return ExitCode::SUCCESS;
    };
    match write_atomically(Path::new(&path), &document) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!(
                "micold-api-schema: cannot write {}: {e}",
                path.to_string_lossy()
            );
            ExitCode::FAILURE
        }
    }
}

fn write_atomically(path: &Path, contents: &str) -> std::io::Result<()> {
    let mut temp: OsString = path.as_os_str().to_owned();
    temp.push(".tmp");
    let temp = Path::new(&temp);
    let result = std::fs::write(temp, contents).and_then(|()| std::fs::rename(temp, path));
    if result.is_err() {
        let _ = std::fs::remove_file(temp);
    }
    result
}
