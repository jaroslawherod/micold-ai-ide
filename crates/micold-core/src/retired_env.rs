//! The `MICOLD_*` start-up variables the configuration file replaced (#1537, research D11).
//!
//! A release build ignores them and says which key to use instead; a build with a `test-env`
//! feature (the daemon's own tests) still honours them, since the tests drive the real binary
//! through its environment. [`resolve`] is the one rule, a pure function so a test can see the
//! release behaviour in a build that has the feature on.

use std::ffi::OsString;

/// One retired variable and the configuration key that replaces it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Retired {
    pub name: &'static str,
    pub key: &'static str,
}

/// Every variable the file replaced, with their keys. The plumbing variables
/// (`MICOLD_DAEMON_CONFIG`, `MICOLD_STATUS_RELAY`, `MICOLD_PI_ACTIVITY_LOG`) are not here.
pub const RETIRED: &[Retired] = &[
    Retired {
        name: "MICOLD_HOME",
        key: "daemon.home",
    },
    Retired {
        name: "MICOLD_IMAGE_REFERENCE",
        key: "daemon.image",
    },
    Retired {
        name: "MICOLD_LISTEN_ADDR",
        key: "listen.tcp",
    },
    Retired {
        name: "MICOLD_IDLE_STOP",
        key: "daemon.idle_stop",
    },
    Retired {
        name: "MICOLD_LOG",
        key: "daemon.log",
    },
    Retired {
        name: "MICOLD_REPOS_ROOT",
        key: "storage.clone_root",
    },
    Retired {
        name: "MICOLD_DATA_DIR",
        key: "daemon.data_dir",
    },
    Retired {
        name: "MICOLD_TOKEN_PATH",
        key: "auth.client_token_file",
    },
];

/// The six variables whose reads live in the daemon's `DaemonSettings`; the data directory and token
/// path are read by `micold-core` itself and follow with their own reads.
pub const DAEMON_SIDE: [&str; 6] = [
    "MICOLD_HOME",
    "MICOLD_IMAGE_REFERENCE",
    "MICOLD_LISTEN_ADDR",
    "MICOLD_IDLE_STOP",
    "MICOLD_LOG",
    "MICOLD_REPOS_ROOT",
];

/// The configuration key that replaces `name`, when `name` is retired.
pub fn config_key(name: &str) -> Option<&'static str> {
    RETIRED.iter().find(|r| r.name == name).map(|r| r.key)
}

/// What a read of a variable yields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    /// The value the caller may use.
    pub value: Option<OsString>,
    /// The warning to log, once, when a retired variable was set and ignored.
    pub warning: Option<String>,
}

/// The value of `name`, given its `raw` environment value. A retired variable yields nothing and a
/// warning naming its key unless `test_env_enabled`; any other name passes through.
pub fn resolve(name: &str, raw: Option<OsString>, test_env_enabled: bool) -> Resolved {
    let Some(key) = config_key(name) else {
        return Resolved {
            value: raw,
            warning: None,
        };
    };
    if test_env_enabled || raw.is_none() {
        return Resolved {
            value: raw,
            warning: None,
        };
    }
    Resolved {
        value: None,
        warning: Some(format!(
            "{name} is no longer read; it has no effect. Set `{key}` in the daemon's configuration file instead"
        )),
    }
}
