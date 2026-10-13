//! The SSH target of a remote daemon and its validation (data-model: `SshTarget`).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::daemons::RegistryError;

/// Where a remote daemon runs: a `Host` alias of the user's SSH configuration or a hostname, and
/// the optional connection settings the user entered. Every unset field comes from the user's SSH
/// configuration. Holds no secret: a key *path*, never key material (FR-003).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SshTarget {
    /// An alias or hostname, as entered; never starts with `-`.
    pub host: String,
    /// The remote user, when not taken from the SSH configuration.
    #[serde(default)]
    pub user: Option<String>,
    /// The remote port, when not taken from the SSH configuration.
    #[serde(default)]
    pub port: Option<u16>,
    /// The private key file, when not taken from the SSH configuration.
    #[serde(default)]
    pub key_file: Option<PathBuf>,
}

/// The settings of a `DaemonRuntime::Ssh`. A struct, not a bare target, so a later per-daemon
/// setting adds a field without changing the variant.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SshSettings {
    /// The host to reach (stored flat beside `kind` in `settings.json`).
    #[serde(flatten)]
    pub target: SshTarget,
}

impl SshTarget {
    /// Validate the fields as entered. The error names the field (FR-016).
    ///
    /// A host starting with `-` would be read by `ssh` as an option; the argv also passes `--`
    /// before the host, so this is the second line of defence, not the only one.
    pub fn new(
        host: &str,
        user: Option<&str>,
        port: Option<u16>,
        key_file: Option<PathBuf>,
    ) -> Result<Self, RegistryError> {
        let host = host.trim();
        if host.is_empty() || host.starts_with('-') || host.chars().any(char::is_whitespace) {
            return Err(RegistryError::InvalidField { field: "host" });
        }
        let user = match user.map(str::trim).filter(|u| !u.is_empty()) {
            Some(u)
                if u.contains('@') || u.starts_with('-') || u.chars().any(char::is_whitespace) =>
            {
                return Err(RegistryError::InvalidField { field: "user" })
            }
            other => other.map(str::to_string),
        };
        if port == Some(0) {
            return Err(RegistryError::InvalidField { field: "port" });
        }
        if let Some(key) = &key_file {
            if !key.is_file() {
                return Err(RegistryError::InvalidField { field: "key_file" });
            }
        }
        Ok(Self {
            host: host.to_string(),
            user,
            port,
            key_file,
        })
    }

    /// `user@host:port` as far as the entry says it; unset parts are omitted.
    pub fn describe(&self) -> String {
        let mut out = String::new();
        if let Some(user) = &self.user {
            out.push_str(user);
            out.push('@');
        }
        out.push_str(&self.host);
        if let Some(port) = self.port {
            out.push(':');
            out.push_str(&port.to_string());
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn invalid(field: &'static str) -> Result<SshTarget, RegistryError> {
        Err(RegistryError::InvalidField { field })
    }

    #[test]
    fn a_plain_host_is_valid_and_keeps_every_other_field_unset() {
        let t = SshTarget::new("build", None, None, None).unwrap();
        assert_eq!(
            (t.host.as_str(), t.user, t.port, t.key_file),
            ("build", None, None, None),
            "only the host is required; the rest comes from the SSH configuration"
        );
    }

    #[test]
    fn a_blank_host_is_refused_naming_the_host() {
        assert_eq!(SshTarget::new("  ", None, None, None), invalid("host"));
    }

    #[test]
    fn a_host_with_whitespace_is_refused() {
        assert_eq!(SshTarget::new("a b", None, None, None), invalid("host"));
    }

    #[test]
    fn a_host_starting_with_a_dash_is_refused_because_ssh_would_read_it_as_an_option() {
        assert_eq!(
            SshTarget::new("-oProxyCommand=x", None, None, None),
            invalid("host")
        );
    }

    #[test]
    fn a_user_containing_an_at_sign_is_refused() {
        assert_eq!(
            SshTarget::new("h", Some("me@x"), None, None),
            invalid("user")
        );
    }

    #[test]
    fn port_zero_is_refused_and_the_range_ends_are_accepted() {
        assert_eq!(SshTarget::new("h", None, Some(0), None), invalid("port"));
        assert!(SshTarget::new("h", None, Some(1), None).is_ok());
        assert!(SshTarget::new("h", None, Some(65535), None).is_ok());
    }

    #[test]
    fn a_key_file_that_does_not_exist_is_refused_naming_the_field() {
        let missing = PathBuf::from("/nonexistent/micold-489/key");
        assert_eq!(
            SshTarget::new("h", None, None, Some(missing)),
            invalid("key_file")
        );
    }

    #[test]
    fn describe_shows_the_entered_user_host_and_port() {
        let t = SshTarget::new("h", Some("me"), Some(2222), None).unwrap();
        assert_eq!(t.describe(), "me@h:2222");
    }
}
