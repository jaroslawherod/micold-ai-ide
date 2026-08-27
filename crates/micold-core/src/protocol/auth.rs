//! The shared-secret handshake token (feature 027, research R1).
//!
//! # Why this exists at all
//!
//! Until now the transport authenticated by **filesystem permission**: the daemon's Unix socket
//! lives in a `0700` directory, so reaching it already proves you are the user (`endpoint.rs`,
//! FR-030). A containerised daemon cannot use that, because a bind-mounted Unix socket does not
//! survive Docker Desktop's file sharing on macOS or Windows — so the sandbox transport is loopback
//! TCP, and loopback TCP has no such property: any local process can connect to `127.0.0.1:<port>`.
//!
//! Moving to TCP without a token would be a security **regression shipped inside a security
//! feature**. Hence this module, and hence the protocol version moving 5 → 6.
//!
//! # How the secret reaches the container
//!
//! Through the filesystem, never the command line. The client writes it `0600` in the per-user
//! state directory and the runtime bind-mounts that file read-only; the daemon reads it at startup.
//! The same permission that protected the socket now protects the secret, and because it is not an
//! argument it does not appear in `docker inspect` output or in the host's process list.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use uuid::Uuid;

/// Where the daemon reads its token from inside the container. Matches the `Containerfile`'s
/// `MICOLD_TOKEN_PATH`, and is the mount point the runtime attaches the host file to.
pub const CONTAINER_TOKEN_PATH: &str = "/run/micold/token";

/// The environment variable naming the token file, so the daemon does not hard-code the path and a
/// host-process daemon can be handed one for testing.
pub const TOKEN_PATH_ENV: &str = "MICOLD_TOKEN_PATH";

/// A handshake token: 32 bytes of CSPRNG output, hex-encoded.
///
/// The bytes come from two v4 UUIDs rather than a new dependency. `uuid`'s v4 constructor is backed
/// by `getrandom`, which is the platform CSPRNG on all three targets — so this is the OS's random
/// source, reached through a crate already in the tree, and 244 bits of it. Adding `rand` for the
/// same bytes would have been a dependency the constitution asks us not to take.
#[derive(Clone, PartialEq, Eq)]
pub struct Token(String);

impl Token {
    /// Generate a fresh token. One per sandbox lifetime.
    pub fn generate() -> Self {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let mut bytes = [0u8; 32];
        bytes[..16].copy_from_slice(a.as_bytes());
        bytes[16..].copy_from_slice(b.as_bytes());
        Token(hex(&bytes))
    }

    /// The token as it travels on the wire.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Read a token from a file, trimming the trailing newline a text editor may add.
    pub fn read_from(path: &Path) -> io::Result<Self> {
        let raw = fs::read_to_string(path)?;
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("token file {} is empty", path.display()),
            ));
        }
        Ok(Token(trimmed.to_string()))
    }

    /// Write this token to `path` with owner-only permissions, creating parents as needed.
    ///
    /// The mode is set **before** the content is written on Unix, so there is no window in which a
    /// world-readable file holds the secret.
    pub fn write_to(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        write_owner_only(path, &self.0)
    }

    /// Whether `candidate` is this token, compared in constant time.
    ///
    /// The comparison must not return early on the first differing byte: a caller that can measure
    /// the difference can recover the token one byte at a time. `==` on `String` is permitted to
    /// short-circuit, so it is not used here.
    pub fn verify(&self, candidate: &str) -> bool {
        let expected = self.0.as_bytes();
        let actual = candidate.as_bytes();
        // Fold the length difference into the result rather than returning on it, so a wrong-length
        // guess is not distinguishable from a wrong-value one by timing.
        let mut diff = (expected.len() ^ actual.len()) as u8;
        for i in 0..expected.len().max(actual.len()) {
            let e = expected.get(i).copied().unwrap_or(0);
            let a = actual.get(i).copied().unwrap_or(0);
            diff |= e ^ a;
        }
        diff == 0
    }
}

/// Deliberately opaque. A token that can be printed is a token that ends up in a log, and this one
/// travels through the client, the runtime and the daemon — three places with logging.
impl std::fmt::Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Token(<redacted>)")
    }
}

/// The conventional host-side location of the token file, beside the endpoint it guards.
pub fn host_token_path(state_dir: &Path) -> PathBuf {
    state_dir.join("sandbox.token")
}

#[cfg(unix)]
fn write_owner_only(path: &Path, contents: &str) -> io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;

    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(contents.as_bytes())?;
    file.sync_all()
}

#[cfg(not(unix))]
fn write_owner_only(path: &Path, contents: &str) -> io::Result<()> {
    // Windows has no mode bits. The file inherits the per-user state directory's ACL, which is the
    // same protection the named pipe already relies on.
    fs::write(path, contents)
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_generated_token_is_sixty_four_hex_characters() {
        let t = Token::generate();
        assert_eq!(t.as_str().len(), 64);
        assert!(t.as_str().chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn two_tokens_differ() {
        // Not a randomness test — a wiring test. A constant token would pass every other assertion
        // in this file and defeat the entire mechanism.
        assert_ne!(Token::generate().as_str(), Token::generate().as_str());
    }

    #[test]
    fn a_token_verifies_itself_and_nothing_else() {
        let t = Token::generate();
        assert!(t.verify(t.as_str()));
        assert!(!t.verify(""));
        assert!(!t.verify("0"));
        assert!(
            !t.verify(&format!("{}0", t.as_str())),
            "a longer guess must fail"
        );

        // One byte wrong, same length: the case a short-circuiting comparison leaks.
        let mut wrong: Vec<char> = t.as_str().chars().collect();
        wrong[0] = if wrong[0] == 'a' { 'b' } else { 'a' };
        assert!(!t.verify(&wrong.into_iter().collect::<String>()));
    }

    #[test]
    fn the_debug_form_never_shows_the_token() {
        // This token passes through the client, the runtime and the daemon — three places with
        // logging, any of which could format it by accident.
        let t = Token::generate();
        let shown = format!("{t:?}");
        assert!(
            !shown.contains(t.as_str()),
            "Debug leaked the token: {shown}"
        );
        assert!(shown.contains("redacted"));
    }

    #[test]
    fn a_written_token_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = host_token_path(dir.path());
        let t = Token::generate();
        t.write_to(&path).unwrap();
        assert_eq!(Token::read_from(&path).unwrap(), t);
    }

    #[cfg(unix)]
    #[test]
    fn a_written_token_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = host_token_path(dir.path());
        Token::generate().write_to(&path).unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "token file mode was {mode:#o}");
    }

    #[test]
    fn a_trailing_newline_is_not_part_of_the_token() {
        // The file is mounted into a container and may be inspected, copied, or re-created by hand.
        // A newline that silently became part of the secret would fail authentication with no
        // visible cause.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("token");
        fs::write(&path, "abc123\n").unwrap();
        assert_eq!(Token::read_from(&path).unwrap().as_str(), "abc123");
    }

    #[test]
    fn an_empty_token_file_is_an_error_not_an_empty_token() {
        // An empty token that verified against an empty guess would authenticate everyone.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("token");
        fs::write(&path, "   \n").unwrap();
        assert!(Token::read_from(&path).is_err());
    }
}
