//! T065b (#1532, research D14's "share this computer's sign-in" relay): the pure logic for
//! deriving a daemon-safe copy of an AI CLI's own sign-in file, and the math for when to ask the
//! host's own CLI to refresh it.
//!
//! Deliberately narrow (D9): this module derives the copy, says when an access token expires and
//! when a refresh is due, and writes the derived copy to a destination path — never the source.
//! It does **not** watch the source file, does not know which daemon or path a derived copy
//! belongs to, and does not deliver anything over a connection: no client→daemon secret-push
//! message exists yet (M3's T100), and the client has no structured read of a target daemon's
//! `[[ai.cli]]` config (M4's T066/T067). Wiring this into a live `notify` watcher and an actual
//! delivery channel is named follow-up, not this milestone's.

use std::io;
use std::path::Path;
use std::time::{Duration, SystemTime};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use micold_core::session::AiCli;

/// Why [`derive_relay_copy`] could not produce a copy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelayError {
    /// `source` did not parse as JSON.
    NotJson,
    /// `cli` has no sign-in file to relay (Copilot, Pi — research D14).
    Unsupported(AiCli),
    /// The JSON lacks the object the refresh token lives in, so it cannot be stripped safely.
    UnexpectedShape,
}

impl std::fmt::Display for RelayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RelayError::NotJson => write!(f, "sign-in file is not valid JSON"),
            RelayError::UnexpectedShape => {
                write!(f, "sign-in file does not have the shape the relay strips")
            }
            RelayError::Unsupported(cli) => {
                write!(f, "{cli:?} has no sign-in file to relay")
            }
        }
    }
}

impl std::error::Error for RelayError {}

/// Derive the copy of `cli`'s own sign-in file (`source`) that is safe to hand the daemon: the
/// refresh token research D14 names is dropped, so the daemon-side CLI can read the access token
/// but can never refresh it (never spending or rotating the user's own refresh token).
///
/// - Claude Code: drops `claudeAiOauth.refreshToken`.
/// - Codex: drops `tokens.refresh_token`, and sets `last_refresh` to `now` (ISO-8601, UTC) so
///   Codex does not attempt its own age-based refresh against a file it cannot write back.
/// - OpenCode: for every top-level provider object, drops that provider's `refresh` field.
///
/// `now` is a parameter so a test can pick it, rather than racing the real clock.
pub fn derive_relay_copy(
    cli: AiCli,
    source: &[u8],
    now: SystemTime,
) -> Result<Vec<u8>, RelayError> {
    let mut value: serde_json::Value =
        serde_json::from_slice(source).map_err(|_| RelayError::NotJson)?;
    match cli {
        AiCli::ClaudeCode => {
            let oauth = value
                .get_mut("claudeAiOauth")
                .and_then(|v| v.as_object_mut())
                .ok_or(RelayError::UnexpectedShape)?;
            oauth.remove("refreshToken");
        }
        AiCli::Codex => {
            let tokens = value
                .get_mut("tokens")
                .and_then(|v| v.as_object_mut())
                .ok_or(RelayError::UnexpectedShape)?;
            tokens.remove("refresh_token");
            if let Some(object) = value.as_object_mut() {
                object.insert(
                    "last_refresh".to_string(),
                    serde_json::Value::String(iso8601_utc(now)),
                );
            }
        }
        AiCli::OpenCode => {
            let object = value.as_object_mut().ok_or(RelayError::UnexpectedShape)?;
            for provider in object.values_mut() {
                if let Some(provider) = provider.as_object_mut() {
                    provider.remove("refresh");
                }
            }
        }
        AiCli::Copilot | AiCli::Pi => return Err(RelayError::Unsupported(cli)),
    }
    serde_json::to_vec(&value).map_err(|_| RelayError::NotJson)
}

/// `now`, as an ISO-8601 UTC timestamp with second precision (`YYYY-MM-DDTHH:MM:SSZ`) — the shape
/// Codex's own `auth.json` uses for `last_refresh`. No timezone database needed: unix time is
/// always UTC.
fn iso8601_utc(now: SystemTime) -> String {
    let secs = now
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Civil-from-days (Howard Hinnant's algorithm), proleptic Gregorian, no external dependency.
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let (hour, minute, second) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let z = days as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { y + 1 } else { y };
    format!(
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z",
        day = day,
    )
}

/// When `cli`'s access token in `derived` (a value [`derive_relay_copy`] already produced) runs
/// out, or `None` when `cli` carries no expiry hint the daemon can read (OpenCode — research
/// D14).
///
/// - Claude Code: `claudeAiOauth.expiresAt`, milliseconds since the epoch.
/// - Codex: the `exp` claim of `tokens.access_token`'s JWT payload (the middle, base64url
///   segment) — decoded for its expiry hint only, never verified: this reads a token the daemon
///   already trusts enough to run, not an untrusted one.
pub fn access_token_expiry(cli: AiCli, derived: &[u8]) -> Option<SystemTime> {
    let value: serde_json::Value = serde_json::from_slice(derived).ok()?;
    match cli {
        AiCli::ClaudeCode => {
            let ms = value.get("claudeAiOauth")?.get("expiresAt")?.as_u64()?;
            SystemTime::UNIX_EPOCH.checked_add(Duration::from_millis(ms))
        }
        AiCli::Codex => {
            let token = value.get("tokens")?.get("access_token")?.as_str()?;
            let payload = token.split('.').nth(1)?;
            let bytes = URL_SAFE_NO_PAD.decode(payload).ok()?;
            let claims: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
            let exp = claims.get("exp")?.as_u64()?;
            SystemTime::UNIX_EPOCH.checked_add(Duration::from_secs(exp))
        }
        AiCli::OpenCode | AiCli::Copilot | AiCli::Pi => None,
    }
}

/// When the application should ask the host's own CLI to refresh, so the daemon-side copy is
/// re-delivered before the access token expires: 75 % of the way from `delivered_at` to
/// `expires_at` (research D14; Claude's 8 h access token → about 6 h). `None` when `expires_at`
/// is not after `delivered_at` (nothing to schedule — [`refresh_urgency`] decides what to do with
/// an already-expired or non-expiring token).
pub fn schedule_refresh_at(delivered_at: SystemTime, expires_at: SystemTime) -> Option<SystemTime> {
    let life = expires_at.duration_since(delivered_at).ok()?;
    if life.is_zero() {
        return None;
    }
    Some(delivered_at + Duration::from_secs_f64(life.as_secs_f64() * 0.75))
}

/// How urgently a refresh is needed, right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshUrgency {
    /// Not yet: wait until this time and try then.
    ScheduledAt(SystemTime),
    /// Under an hour remains (or it already expired): ask now, do not wait for the schedule.
    WarnNow,
}

/// Decide between the two, from `now`, the access token's `expires_at`, and
/// [`schedule_refresh_at`]'s `scheduled_at` (`None` when there was nothing to schedule — treated
/// as due now).
pub fn refresh_urgency(
    now: SystemTime,
    expires_at: SystemTime,
    scheduled_at: Option<SystemTime>,
) -> RefreshUrgency {
    const WARN_WINDOW: Duration = Duration::from_secs(3600);
    let remaining = expires_at.duration_since(now).unwrap_or(Duration::ZERO);
    if remaining <= WARN_WINDOW {
        return RefreshUrgency::WarnNow;
    }
    match scheduled_at {
        Some(at) => RefreshUrgency::ScheduledAt(at),
        None => RefreshUrgency::WarnNow,
    }
}

/// Write `source`'s derived copy (`derive_relay_copy`) to `dest`, only when it differs from what
/// is already there, and never touch `source` itself (the relay reads the user's own CLI's file,
/// it does not get to decide when that CLI considers itself refreshed). `Ok(true)` when `dest`
/// was written, `Ok(false)` when it already held the derived bytes.
pub fn apply_relay(source: &Path, dest: &Path, cli: AiCli, now: SystemTime) -> io::Result<bool> {
    let bytes = std::fs::read(source)?;
    let derived = derive_relay_copy(cli, &bytes, now)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
    if same_file(source, dest) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "the relay destination is the sign-in source itself",
        ));
    }
    if let Ok(existing) = std::fs::read(dest) {
        if existing == derived {
            return Ok(false);
        }
    }
    let (Some(dir), Some(name)) = (dest.parent(), dest.file_name().and_then(|n| n.to_str())) else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "the relay destination has no directory or file name",
        ));
    };
    // Owner-only, written to a temporary file and renamed over `dest`: the copy holds a live access
    // token, is never readable under a wider mode, and a reader sees the old file or the new one.
    // A rename also replaces a symlink or hard link at `dest` rather than writing through it.
    micold_core::owner_only::write(dir, name, &derived)?;
    Ok(true)
}

/// Whether `a` and `b` name the same file once resolved (a symlink or the same path); `b` need not
/// exist yet, so its directory is resolved and its name kept.
fn same_file(a: &Path, b: &Path) -> bool {
    let Ok(a) = std::fs::canonicalize(a) else {
        return false;
    };
    let resolved = std::fs::canonicalize(b).or_else(|_| {
        let dir = b.parent().filter(|d| !d.as_os_str().is_empty());
        let dir = std::fs::canonicalize(dir.unwrap_or(Path::new(".")))?;
        Ok::<_, io::Error>(dir.join(b.file_name().unwrap_or_default()))
    });
    resolved.is_ok_and(|b| a == b)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLAUDE_SIGN_IN: &str = r#"{"claudeAiOauth":{"accessToken":"acc","refreshToken":"ref","expiresAt":1700000000000,"refreshTokenExpiresAt":1700600000000}}"#;

    fn codex_sign_in() -> String {
        // A minimal unsigned JWT: header.payload.signature, payload `{"exp":1700000500}`.
        let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"none"}"#);
        let payload = URL_SAFE_NO_PAD.encode(br#"{"exp":1700000500}"#);
        format!(r#"{{"tokens":{{"access_token":"{header}.{payload}.sig","refresh_token":"ref"}}}}"#,)
    }

    const OPENCODE_SIGN_IN: &str = r#"{"anthropic":{"type":"oauth","refresh":"ref","access":"acc"},"openai":{"type":"api","key":"sk-x"}}"#;

    #[test]
    fn claude_codes_derived_copy_has_no_refresh_token() {
        let now = SystemTime::UNIX_EPOCH;
        let derived = derive_relay_copy(AiCli::ClaudeCode, CLAUDE_SIGN_IN.as_bytes(), now).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&derived).unwrap();
        assert!(value["claudeAiOauth"].get("refreshToken").is_none());
        assert_eq!(value["claudeAiOauth"]["accessToken"], "acc");
    }

    #[test]
    fn codexs_derived_copy_has_no_refresh_token_and_a_fresh_last_refresh() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let source = codex_sign_in();
        let derived = derive_relay_copy(AiCli::Codex, source.as_bytes(), now).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&derived).unwrap();
        assert!(value["tokens"].get("refresh_token").is_none());
        assert_eq!(value["last_refresh"], "2023-11-14T22:13:20Z");
    }

    #[test]
    fn opencodes_derived_copy_drops_every_providers_refresh_field() {
        let now = SystemTime::UNIX_EPOCH;
        let derived = derive_relay_copy(AiCli::OpenCode, OPENCODE_SIGN_IN.as_bytes(), now).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&derived).unwrap();
        assert!(value["anthropic"].get("refresh").is_none());
        assert_eq!(value["anthropic"]["access"], "acc");
        assert_eq!(value["openai"]["key"], "sk-x");
    }

    #[test]
    fn copilot_and_pi_have_no_sign_in_file_to_relay() {
        let now = SystemTime::UNIX_EPOCH;
        assert_eq!(
            derive_relay_copy(AiCli::Copilot, b"{}", now),
            Err(RelayError::Unsupported(AiCli::Copilot))
        );
        assert_eq!(
            derive_relay_copy(AiCli::Pi, b"{}", now),
            Err(RelayError::Unsupported(AiCli::Pi))
        );
    }

    #[test]
    fn not_json_is_reported_rather_than_panicking() {
        let now = SystemTime::UNIX_EPOCH;
        assert_eq!(
            derive_relay_copy(AiCli::ClaudeCode, b"not json", now),
            Err(RelayError::NotJson)
        );
    }

    #[test]
    fn claude_codes_access_token_expiry_reads_the_milliseconds_field() {
        let now = SystemTime::UNIX_EPOCH;
        let derived = derive_relay_copy(AiCli::ClaudeCode, CLAUDE_SIGN_IN.as_bytes(), now).unwrap();
        assert_eq!(
            access_token_expiry(AiCli::ClaudeCode, &derived),
            Some(SystemTime::UNIX_EPOCH + Duration::from_millis(1_700_000_000_000))
        );
    }

    #[test]
    fn codexs_access_token_expiry_decodes_the_jwts_exp_claim() {
        let now = SystemTime::UNIX_EPOCH;
        let derived = derive_relay_copy(AiCli::Codex, codex_sign_in().as_bytes(), now).unwrap();
        assert_eq!(
            access_token_expiry(AiCli::Codex, &derived),
            Some(SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_500))
        );
    }

    #[test]
    fn opencode_has_no_expiry_hint_the_daemon_can_read() {
        let now = SystemTime::UNIX_EPOCH;
        let derived = derive_relay_copy(AiCli::OpenCode, OPENCODE_SIGN_IN.as_bytes(), now).unwrap();
        assert_eq!(access_token_expiry(AiCli::OpenCode, &derived), None);
    }

    #[test]
    fn schedule_refresh_at_is_three_quarters_of_the_way_to_expiry() {
        let delivered = SystemTime::UNIX_EPOCH;
        let expires = delivered + Duration::from_secs(8 * 3600); // Claude Code's 8h access token.
        assert_eq!(
            schedule_refresh_at(delivered, expires),
            Some(delivered + Duration::from_secs(6 * 3600))
        );
    }

    #[test]
    fn schedule_refresh_at_is_none_when_expiry_is_not_after_delivery() {
        let t = SystemTime::UNIX_EPOCH + Duration::from_secs(100);
        assert_eq!(schedule_refresh_at(t, t), None);
        assert_eq!(schedule_refresh_at(t, t - Duration::from_secs(1)), None);
    }

    #[test]
    fn refresh_urgency_warns_now_under_an_hour_out() {
        let expires = SystemTime::UNIX_EPOCH + Duration::from_secs(3600);
        let now = SystemTime::UNIX_EPOCH;
        assert_eq!(
            refresh_urgency(now, expires, Some(now + Duration::from_secs(1800))),
            RefreshUrgency::WarnNow
        );
    }

    #[test]
    fn refresh_urgency_waits_for_the_schedule_otherwise() {
        let expires = SystemTime::UNIX_EPOCH + Duration::from_secs(8 * 3600);
        let now = SystemTime::UNIX_EPOCH;
        let scheduled = now + Duration::from_secs(6 * 3600);
        assert_eq!(
            refresh_urgency(now, expires, Some(scheduled)),
            RefreshUrgency::ScheduledAt(scheduled)
        );
    }

    #[test]
    fn apply_relay_writes_the_derived_copy_and_never_touches_the_source() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("source.json");
        let dest = tmp.path().join("relayed").join("dest.json");
        std::fs::write(&source, CLAUDE_SIGN_IN).unwrap();
        let before_bytes = std::fs::read(&source).unwrap();
        let before_mtime = std::fs::metadata(&source).unwrap().modified().unwrap();

        let now = SystemTime::UNIX_EPOCH;
        assert!(apply_relay(&source, &dest, AiCli::ClaudeCode, now).unwrap());
        let written: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&dest).unwrap()).unwrap();
        assert!(written["claudeAiOauth"].get("refreshToken").is_none());

        assert_eq!(std::fs::read(&source).unwrap(), before_bytes);
        assert_eq!(
            std::fs::metadata(&source).unwrap().modified().unwrap(),
            before_mtime
        );

        // A second call with unchanged source content writes nothing new.
        assert!(!apply_relay(&source, &dest, AiCli::ClaudeCode, now).unwrap());
    }

    #[test]
    fn an_unexpected_shape_is_refused_rather_than_relayed_with_its_refresh_token() {
        let now = SystemTime::UNIX_EPOCH;
        assert_eq!(
            derive_relay_copy(AiCli::ClaudeCode, br#"{"refreshToken":"r"}"#, now),
            Err(RelayError::UnexpectedShape)
        );
        assert_eq!(
            derive_relay_copy(AiCli::Codex, b"[]", now),
            Err(RelayError::UnexpectedShape)
        );
        assert_eq!(
            derive_relay_copy(AiCli::OpenCode, b"[]", now),
            Err(RelayError::UnexpectedShape)
        );
    }

    #[test]
    fn a_huge_expiry_is_none_rather_than_a_panic() {
        let claude = format!(r#"{{"claudeAiOauth":{{"expiresAt":{}}}}}"#, u64::MAX);
        // Milliseconds of u64::MAX still fit a SystemTime; the call must simply not panic.
        let _ = access_token_expiry(AiCli::ClaudeCode, claude.as_bytes());
        let payload = URL_SAFE_NO_PAD.encode(format!(r#"{{"exp":{}}}"#, u64::MAX));
        let codex = format!(r#"{{"tokens":{{"access_token":"h.{payload}.s"}}}}"#);
        assert_eq!(access_token_expiry(AiCli::Codex, codex.as_bytes()), None);
    }

    #[test]
    fn apply_relay_refuses_a_destination_that_is_the_source() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("source.json");
        std::fs::write(&source, CLAUDE_SIGN_IN).unwrap();
        let link = tmp.path().join("link.json");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&source, &link).unwrap();
        #[cfg(not(unix))]
        let link = source.clone();
        let now = SystemTime::UNIX_EPOCH;
        for dest in [&source, &link] {
            let err = apply_relay(&source, dest, AiCli::ClaudeCode, now).unwrap_err();
            assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        }
        assert_eq!(std::fs::read_to_string(&source).unwrap(), CLAUDE_SIGN_IN);
    }

    #[cfg(unix)]
    #[test]
    fn the_relayed_copy_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("source.json");
        let dest = tmp.path().join("relayed").join("dest.json");
        std::fs::write(&source, CLAUDE_SIGN_IN).unwrap();
        apply_relay(&source, &dest, AiCli::ClaudeCode, SystemTime::UNIX_EPOCH).unwrap();
        let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&dest), 0o600);
        assert_eq!(mode(dest.parent().unwrap()), 0o700);
    }
}
