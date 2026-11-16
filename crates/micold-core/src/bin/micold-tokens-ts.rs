//! Write the design tokens as a TypeScript module (issue #1950, FR-005).
//!
//! Deliberately trivial: it writes what `tokens::ts::module()` returns and decides nothing. Every
//! property the module has to hold is asserted by unit tests over that function, which is only worth
//! anything if this binary cannot introduce a line of its own. With a path it writes that file
//! atomically (a temp file beside it, then a rename, so a reader never sees half a module); with no
//! argument it prints to stdout. A failed write exits 1, names the path, and leaves no file behind.
//!
//! The client's copy is regenerated with
//!
//!     cargo run -p micold-core --bin micold-tokens-ts -- apps/client/src/theme/tokens.generated.ts

use std::ffi::OsString;
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let module = micold_core::tokens::ts::module();
    let Some(path) = std::env::args_os().nth(1) else {
        print!("{module}");
        return ExitCode::SUCCESS;
    };
    match write_atomically(Path::new(&path), &module) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!(
                "micold-tokens-ts: cannot write {}: {e}",
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
