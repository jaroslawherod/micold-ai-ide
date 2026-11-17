//! The orchestrator's own settings (#1981, FR-019): where it keeps its registry (`--data-dir`) and
//! which local socket it binds (`--socket`); `--config` names the file with the `[database]` table
//! (#1948). `--data-dir` and `--socket` each have an environment twin, and a flag wins
//! over its variable, a variable over the default. The defaults are today's: the user's data
//! directory and the orchestrator endpoint beside the daemon's. A default is only resolved when
//! neither a flag nor a variable gives the value, so a container that passes both never touches
//! `HOME` or `XDG_RUNTIME_DIR`.

use std::ffi::OsString;
use std::io;
use std::path::PathBuf;

use micold_core::endpoint::Endpoint;

/// The variable that sets the data directory when `--data-dir` is absent.
pub const DATA_DIR_ENV: &str = "MICOLD_ORCHESTRATOR_DATA_DIR";
/// The variable that sets the socket when `--socket` is absent.
pub const SOCKET_ENV: &str = "MICOLD_ORCHESTRATOR_SOCKET";
/// The variable that sets the ops listener's address when `--ops-addr` is absent.
pub const OPS_ADDR_ENV: &str = "MICOLD_ORCHESTRATOR_OPS_ADDR";

/// Where the orchestrator runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrchestratorConfig {
    /// The directory the registry lives under (`orchestrator/registry.json` inside it).
    pub data_dir: PathBuf,
    /// The socket it binds and the lock beside it.
    pub endpoint: Endpoint,
    /// Where the ops listener (`/healthz`, `/readyz`) binds; none by default.
    pub ops_addr: Option<std::net::SocketAddr>,
}

/// The values used when neither a flag nor a variable gives one.
pub trait Defaults {
    /// The data directory.
    fn data_dir(&self) -> io::Result<PathBuf>;
    /// The endpoint.
    fn endpoint(&self) -> io::Result<Endpoint>;
}

/// Today's defaults: [`micold_core::paths::data_dir`] and
/// [`micold_core::endpoint::orchestrator_endpoint`].
pub struct SystemDefaults;

impl Defaults for SystemDefaults {
    fn data_dir(&self) -> io::Result<PathBuf> {
        micold_core::paths::data_dir().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "no data directory to keep the orchestrator registry in (pass --data-dir or set {DATA_DIR_ENV})"
                ),
            )
        })
    }

    fn endpoint(&self) -> io::Result<Endpoint> {
        micold_core::endpoint::orchestrator_endpoint()
    }
}

/// The endpoint for an explicit socket path: the lock sits beside it, as in `run_with`. Unlike the
/// default, the directory is not checked for an owner-only mode: whoever names the path owns that
/// choice (in the chart it is a pod-private `emptyDir`, reachable only by the pod's containers).
pub fn endpoint_at(socket: PathBuf) -> Endpoint {
    Endpoint {
        lock_path: socket.with_extension("lock"),
        socket_path: socket,
    }
}

fn invalid(message: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

/// The orchestrator's arguments as given, before any variable or default fills a gap.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OrchestratorArgs {
    /// `--data-dir`.
    pub data_dir: Option<PathBuf>,
    /// `--socket`.
    pub socket: Option<PathBuf>,
    /// `--config`: the configuration file holding the `[database]` table (#1948).
    pub config: Option<PathBuf>,
    /// `--ops-addr`, unparsed.
    pub ops_addr: Option<OsString>,
}

impl OrchestratorArgs {
    /// Parse `args` (the arguments after `orchestrator`). `--flag value` and `--flag=value` both
    /// work; an unknown argument is an error that names it, and so is a flag with no or an empty
    /// value.
    pub fn parse(args: &[OsString]) -> io::Result<Self> {
        let mut out = Self::default();
        let mut rest = args.iter();
        while let Some(arg) = rest.next() {
            let text = arg
                .to_str()
                .ok_or_else(|| invalid(format!("orchestrator argument {arg:?} is not UTF-8")))?;
            let (name, inline) = match text.split_once('=') {
                Some((name, value)) if name.starts_with("--") => {
                    (name, Some(OsString::from(value)))
                }
                _ => (text, None),
            };
            if !matches!(name, "--data-dir" | "--socket" | "--config" | "--ops-addr") {
                return Err(invalid(format!(
                    "unknown orchestrator argument {name:?} (expected --data-dir, --socket, --config or --ops-addr)"
                )));
            }
            let value = match inline {
                Some(value) => value,
                // `--data-dir --socket=/x` is a missing value, not a directory named `--socket=/x`.
                None => rest
                    .next()
                    .filter(|next| !next.to_string_lossy().starts_with("--"))
                    .cloned()
                    .ok_or_else(|| invalid(format!("{name} needs a value")))?,
            };
            if value.is_empty() {
                return Err(invalid(format!("{name} needs a non-empty value")));
            }
            match name {
                "--data-dir" => out.data_dir = Some(PathBuf::from(value)),
                "--socket" => out.socket = Some(PathBuf::from(value)),
                "--config" => out.config = Some(PathBuf::from(value)),
                _ => out.ops_addr = Some(value),
            }
        }
        Ok(out)
    }
}

impl OrchestratorConfig {
    /// Resolve the settings from `args` (the arguments after `orchestrator`), then `env`, then
    /// `defaults`: [`OrchestratorArgs::parse`], then [`OrchestratorConfig::resolve`].
    pub fn from_args_env(
        args: &[OsString],
        env: &dyn Fn(&str) -> Option<OsString>,
        defaults: &dyn Defaults,
    ) -> io::Result<Self> {
        Self::resolve(OrchestratorArgs::parse(args)?, env, defaults)
    }

    /// Fill what `args` leave out from `env`, then `defaults`. An empty variable counts as unset.
    /// A default is resolved only when neither gives the value.
    pub fn resolve(
        args: OrchestratorArgs,
        env: &dyn Fn(&str) -> Option<OsString>,
        defaults: &dyn Defaults,
    ) -> io::Result<Self> {
        let from_env = |name: &str| {
            env(name)
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
        };
        let data_dir = match args.data_dir.or_else(|| from_env(DATA_DIR_ENV)) {
            Some(dir) => dir,
            None => defaults.data_dir()?,
        };
        let endpoint = match args.socket.or_else(|| from_env(SOCKET_ENV)) {
            Some(socket) => endpoint_at(socket),
            None => defaults.endpoint()?,
        };
        let ops_addr = match args.ops_addr {
            Some(value) => Some(socket_addr(&value, "--ops-addr")?),
            None => match env(OPS_ADDR_ENV).filter(|value| !value.is_empty()) {
                Some(value) => Some(socket_addr(&value, OPS_ADDR_ENV)?),
                None => None,
            },
        };
        Ok(Self {
            data_dir,
            endpoint,
            ops_addr,
        })
    }
}

/// `value` as an address, or an error naming `source`.
fn socket_addr(value: &OsString, source: &str) -> io::Result<std::net::SocketAddr> {
    value
        .to_str()
        .and_then(|text| text.parse().ok())
        .ok_or_else(|| {
            invalid(format!(
                "{source} {value:?} is not an address like 0.0.0.0:9090"
            ))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::collections::HashMap;

    /// Defaults that say where they came from, and count how often they were asked.
    #[derive(Default)]
    struct FakeDefaults {
        asked: Cell<u32>,
    }

    impl Defaults for FakeDefaults {
        fn data_dir(&self) -> io::Result<PathBuf> {
            self.asked.set(self.asked.get() + 1);
            Ok(PathBuf::from("/default/data"))
        }

        fn endpoint(&self) -> io::Result<Endpoint> {
            self.asked.set(self.asked.get() + 1);
            Ok(Endpoint {
                socket_path: PathBuf::from("/default/orchestrator.sock"),
                lock_path: PathBuf::from("/default/orchestrator.lock"),
            })
        }
    }

    /// Defaults that cannot be resolved, as in a container with no `HOME`.
    struct NoDefaults;

    impl Defaults for NoDefaults {
        fn data_dir(&self) -> io::Result<PathBuf> {
            Err(io::Error::new(io::ErrorKind::NotFound, "no data dir"))
        }

        fn endpoint(&self) -> io::Result<Endpoint> {
            Err(io::Error::new(io::ErrorKind::NotFound, "no runtime dir"))
        }
    }

    fn args(list: &[&str]) -> Vec<OsString> {
        list.iter().map(OsString::from).collect()
    }

    fn env_of(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
        let map: HashMap<String, OsString> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), OsString::from(v)))
            .collect();
        move |name| map.get(name).cloned()
    }

    fn resolve(list: &[&str], env: &[(&str, &str)]) -> io::Result<OrchestratorConfig> {
        OrchestratorConfig::from_args_env(&args(list), &env_of(env), &FakeDefaults::default())
    }

    /// (case, args, env, data dir, socket, lock)
    type PrecedenceCase<'a> = (
        &'a str,
        &'a [&'a str],
        &'a [(&'a str, &'a str)],
        &'a str,
        &'a str,
        &'a str,
    );

    #[test]
    fn precedence_is_flag_then_env_then_default() {
        let cases: &[PrecedenceCase] = &[
            (
                "nothing given",
                &[],
                &[],
                "/default/data",
                "/default/orchestrator.sock",
                "/default/orchestrator.lock",
            ),
            (
                "env only",
                &[],
                &[(DATA_DIR_ENV, "/env/data"), (SOCKET_ENV, "/env/o.sock")],
                "/env/data",
                "/env/o.sock",
                "/env/o.lock",
            ),
            (
                "flag over env",
                &["--data-dir", "/flag/data", "--socket", "/flag/o.sock"],
                &[(DATA_DIR_ENV, "/env/data"), (SOCKET_ENV, "/env/o.sock")],
                "/flag/data",
                "/flag/o.sock",
                "/flag/o.lock",
            ),
            (
                "flag=value form",
                &[
                    "--data-dir=/var/lib/micold",
                    "--socket=/run/micold/orchestrator.sock",
                ],
                &[],
                "/var/lib/micold",
                "/run/micold/orchestrator.sock",
                "/run/micold/orchestrator.lock",
            ),
            (
                "mixed sources",
                &["--socket=/flag/o.sock"],
                &[(DATA_DIR_ENV, "/env/data")],
                "/env/data",
                "/flag/o.sock",
                "/flag/o.lock",
            ),
            (
                "empty env counts as unset",
                &[],
                &[(DATA_DIR_ENV, ""), (SOCKET_ENV, "")],
                "/default/data",
                "/default/orchestrator.sock",
                "/default/orchestrator.lock",
            ),
            (
                "last flag wins",
                &["--data-dir=/first", "--data-dir", "/second"],
                &[],
                "/second",
                "/default/orchestrator.sock",
                "/default/orchestrator.lock",
            ),
            (
                "value with an equals sign",
                &["--data-dir=/a=b"],
                &[],
                "/a=b",
                "/default/orchestrator.sock",
                "/default/orchestrator.lock",
            ),
        ];
        for (case, list, env, data_dir, socket, lock) in cases {
            let config = resolve(list, env).unwrap_or_else(|e| panic!("{case}: {e}"));
            assert_eq!(
                config.data_dir,
                PathBuf::from(data_dir),
                "{case}: a flag beats its variable, a variable beats the default"
            );
            assert_eq!(
                config.endpoint.socket_path,
                PathBuf::from(socket),
                "{case}: the socket follows the same precedence"
            );
            assert_eq!(
                config.endpoint.lock_path,
                PathBuf::from(lock),
                "{case}: an explicit socket's lock sits beside it with the .lock extension"
            );
        }
    }

    #[test]
    fn bad_arguments_are_errors_that_name_them() {
        // (case, args, text the message must contain)
        let cases: &[(&str, &[&str], &str)] = &[
            ("unknown flag", &["--ops"], "\"--ops\""),
            ("unknown flag=value", &["--port=9090"], "\"--port\""),
            ("stray word", &["serve"], "\"serve\""),
            ("missing value", &["--data-dir"], "--data-dir needs a value"),
            (
                "a flag where the value goes",
                &["--data-dir", "--socket=/x"],
                "--data-dir needs a value",
            ),
            (
                "empty inline value",
                &["--socket="],
                "--socket needs a non-empty value",
            ),
        ];
        for (case, list, needle) in cases {
            let err = resolve(list, &[]).expect_err(case);
            assert_eq!(
                err.kind(),
                io::ErrorKind::InvalidInput,
                "{case}: a bad argument is the caller's mistake"
            );
            assert!(
                err.to_string().contains(needle),
                "{case}: the message names the argument, got {err}"
            );
        }
    }

    #[test]
    fn config_names_the_database_file_beside_the_other_flags() {
        for list in [
            &["--config", "/etc/micold/daemon.toml", "--socket=/s.sock"][..],
            &["--socket", "/s.sock", "--config=/etc/micold/daemon.toml"][..],
        ] {
            let parsed = OrchestratorArgs::parse(&args(list)).expect("valid arguments");
            assert_eq!(
                parsed.config,
                Some(PathBuf::from("/etc/micold/daemon.toml")),
                "{list:?}"
            );
            assert_eq!(parsed.socket, Some(PathBuf::from("/s.sock")), "{list:?}");
            assert_eq!(parsed.data_dir, None, "{list:?}");
        }
        let err = OrchestratorArgs::parse(&args(&["--config"])).expect_err("no value");
        assert!(err.to_string().contains("--config needs a value"), "{err}");
    }

    #[test]
    fn defaults_are_not_resolved_when_both_values_are_given() {
        let defaults = FakeDefaults::default();
        OrchestratorConfig::from_args_env(
            &args(&["--data-dir=/d"]),
            &env_of(&[(SOCKET_ENV, "/s.sock")]),
            &defaults,
        )
        .expect("flag and env give every value");
        assert_eq!(
            defaults.asked.get(),
            0,
            "a container without HOME or XDG_RUNTIME_DIR must not hit the default lookups"
        );
        let config = OrchestratorConfig::from_args_env(
            &args(&["--data-dir=/d", "--socket=/s.sock"]),
            &env_of(&[]),
            &NoDefaults,
        )
        .expect("unresolvable defaults are fine when the flags are given");
        assert_eq!(config.data_dir, PathBuf::from("/d"), "the flag is used");
    }

    #[test]
    fn an_unresolvable_default_is_an_error() {
        let err = OrchestratorConfig::from_args_env(&[], &env_of(&[]), &NoDefaults)
            .expect_err("nothing given and no default");
        assert_eq!(
            err.kind(),
            io::ErrorKind::NotFound,
            "the default's own error is passed on"
        );
    }

    /// FR-019: `--ops-addr` over its variable; none by default; a bad value names its source.
    #[test]
    fn ops_addr_is_flag_then_env_then_none() {
        let both = ["--data-dir", "/d", "--socket", "/s"];
        assert_eq!(resolve(&both, &[]).unwrap().ops_addr, None);
        assert_eq!(
            resolve(&both, &[(OPS_ADDR_ENV, "")]).unwrap().ops_addr,
            None
        );
        let env = [(OPS_ADDR_ENV, "127.0.0.1:1")];
        assert_eq!(
            resolve(&both, &env).unwrap().ops_addr,
            Some("127.0.0.1:1".parse().unwrap())
        );
        let flagged = [
            "--data-dir",
            "/d",
            "--socket",
            "/s",
            "--ops-addr=0.0.0.0:9090",
        ];
        assert_eq!(
            resolve(&flagged, &env).unwrap().ops_addr,
            Some("0.0.0.0:9090".parse().unwrap())
        );
        let bad_flag = resolve(
            &["--data-dir", "/d", "--socket", "/s", "--ops-addr", "x"],
            &[],
        )
        .unwrap_err()
        .to_string();
        assert!(bad_flag.contains("--ops-addr"), "{bad_flag}");
        let bad_env = resolve(&both, &[(OPS_ADDR_ENV, "nope")])
            .unwrap_err()
            .to_string();
        assert!(bad_env.contains(OPS_ADDR_ENV), "{bad_env}");
    }
}
