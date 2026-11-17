//! `micold-daemon e2e-link idle | register --name N [--hold] --socket P | list --socket P`: the
//! test daemon of the kind e2e suites (#1981, FR-011, plan D1).
//!
//! The chart's `testDaemon` sidecar runs this binary beside the orchestrator and shares its
//! `/run/micold` socket volume. `idle` only keeps the container alive; the suites then
//! `kubectl exec` the other verbs, which dial the orchestrator's link socket directly and speak
//! the link protocol as the daemon's own orchestrator link does ([`crate::orchestrator_link`]).
//! Test support only: no session, no singleton, no data or runtime directory, nothing written.

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use interprocess::local_socket::tokio::{prelude::*, Stream};
use interprocess::local_socket::{GenericFilePath, ToFsName};
use micold_core::daemon_config::Kind;
use micold_core::orchestrator::{LinkAnnouncement, MAX_PREAMBLE_FRAME};
use micold_core::protocol::routing::WireDaemonId;
use micold_core::protocol::version::PROTOCOL_VERSION;
use tokio::io::AsyncReadExt;
use tokio_util::codec::{Framed, LengthDelimitedCodec};

use crate::orchestrator_link::dial::{attempt, steady, Handshake};

/// One verb of the subcommand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verb {
    /// Keep the container alive until it is stopped.
    Idle,
    /// Link as daemon `name`; with `hold`, stay linked until stdin closes.
    Register {
        name: String,
        hold: bool,
        socket: PathBuf,
    },
    /// Print the orchestrator's listing as JSON.
    List { socket: PathBuf },
}

const USAGE: &str =
    "usage: e2e-link idle | register --name <name> [--hold] --socket <path> | list --socket <path>";

/// Reads the arguments after `e2e-link`.
pub fn parse(args: &[OsString]) -> Result<Verb, String> {
    let mut args = args.iter().map(|a| a.to_string_lossy().into_owned());
    let verb = args.next().ok_or(USAGE)?;
    let (mut name, mut hold, mut socket) = (None, false, None);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--hold" if verb == "register" => hold = true,
            "--name" if verb == "register" => {
                name = Some(args.next().ok_or("--name needs a value")?)
            }
            "--socket" if verb != "idle" => {
                socket = Some(PathBuf::from(args.next().ok_or("--socket needs a value")?))
            }
            other => return Err(format!("unexpected argument '{other}' for {verb}; {USAGE}")),
        }
    }
    let socket = || {
        socket
            .clone()
            .ok_or(format!("{verb} needs --socket; {USAGE}"))
    };
    match verb.as_str() {
        "idle" => Ok(Verb::Idle),
        "register" => Ok(Verb::Register {
            name: name.ok_or(format!("register needs --name; {USAGE}"))?,
            hold,
            socket: socket()?,
        }),
        "list" => Ok(Verb::List { socket: socket()? }),
        other => Err(format!("unknown verb '{other}'; {USAGE}")),
    }
}

/// The subcommand: returns the process exit status. Prints the listing for `list`.
pub fn main(args: &[OsString]) -> i32 {
    let verb = match parse(args) {
        Ok(verb) => verb,
        Err(message) => {
            eprintln!("micold-daemon: e2e-link: {message}");
            return 2;
        }
    };
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(e) => {
            eprintln!("micold-daemon: e2e-link: {e}");
            return 1;
        }
    };
    let result = runtime.block_on(async {
        match verb {
            Verb::Idle => idle().await,
            Verb::Register { name, hold, socket } => {
                register(&socket, &name, hold, tokio::io::stdin()).await
            }
            Verb::List { socket } => {
                println!("{}", list(&socket).await?);
                Ok(())
            }
        }
    });
    match result {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("micold-daemon: e2e-link: {e}");
            1
        }
    }
}

/// Waits for SIGTERM (or ctrl-c). A container's first process has no default signal action, so
/// without this a pod deletion would wait out its whole grace period.
async fn idle() -> io::Result<()> {
    #[cfg(unix)]
    {
        let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        tokio::select! {
            _ = term.recv() => Ok(()),
            r = tokio::signal::ctrl_c() => r,
        }
    }
    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c().await
    }
}

async fn connect(socket: &Path) -> io::Result<Stream> {
    let name = socket.as_os_str().to_fs_name::<GenericFilePath>()?;
    Stream::connect(name).await
}

fn announcement(name: &str) -> io::Result<LinkAnnouncement> {
    let id = WireDaemonId::new(name.to_ascii_lowercase())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, format!("--name: {e}")))?;
    Ok(LinkAnnouncement {
        format: micold_core::orchestrator::announce::ANNOUNCEMENT_FORMAT,
        id,
        kind: Kind::Kubernetes,
        name: name.to_owned(),
        protocol_version: PROTOCOL_VERSION,
        capabilities: Default::default(),
        tenant: None,
    })
}

/// Links as daemon `name` (id: its lower case). Without `hold` the link ends as soon as
/// `Welcome` is read, so the orchestrator lists the daemon offline; with it, the link stays up,
/// answering pings, until `stdin` ends.
pub async fn register(
    socket: &Path,
    name: &str,
    hold: bool,
    mut stdin: impl tokio::io::AsyncRead + Unpin,
) -> io::Result<()> {
    let announcement = announcement(name)?;
    let framed = match attempt(connect(socket).await?, &announcement).await? {
        Handshake::Linked(framed) => framed,
        Handshake::Refused { reason, message } => {
            return Err(io::Error::other(format!(
                "the orchestrator refused the link ({reason:?}): {message}"
            )))
        }
        Handshake::HelloRefused(reason) => {
            return Err(io::Error::other(format!("Hello refused: {reason}")))
        }
    };
    if !hold {
        return Ok(());
    }
    let closed = async {
        let mut sink = [0u8; 64];
        while stdin.read(&mut sink).await? > 0 {}
        Ok::<(), io::Error>(())
    };
    tokio::select! {
        r = steady(framed, None) => r,
        r = closed => r,
    }
}

/// The orchestrator's listing frame: `{ "daemons": [{ "id", "name", "health", .. }], .. }`.
pub async fn list(socket: &Path) -> io::Result<String> {
    let codec = LengthDelimitedCodec::builder()
        .length_field_type::<u32>()
        .little_endian()
        .max_frame_length(MAX_PREAMBLE_FRAME)
        .new_codec();
    let mut framed = Framed::new(connect(socket).await?, codec);
    framed
        .send(Bytes::from_static(br#"{"query":"list"}"#))
        .await?;
    let frame = framed
        .next()
        .await
        .ok_or_else(|| io::Error::from(io::ErrorKind::UnexpectedEof))??;
    String::from_utf8(frame.to_vec()).map_err(io::Error::other)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator::{run_with, SystemClock};
    use micold_core::orchestrator::JsonRegistryStore;
    use std::sync::Arc;
    use std::time::Duration;

    fn args(list: &[&str]) -> Vec<OsString> {
        list.iter().map(OsString::from).collect()
    }

    #[test]
    fn parse_reads_each_verb() {
        assert_eq!(parse(&args(&["idle"])), Ok(Verb::Idle));
        assert_eq!(
            parse(&args(&["register", "--name", "A", "--socket", "/s"])),
            Ok(Verb::Register {
                name: "A".into(),
                hold: false,
                socket: "/s".into()
            })
        );
        assert_eq!(
            parse(&args(&[
                "register", "--hold", "--socket", "/s", "--name", "b"
            ])),
            Ok(Verb::Register {
                name: "b".into(),
                hold: true,
                socket: "/s".into()
            })
        );
        assert_eq!(
            parse(&args(&["list", "--socket", "/s"])),
            Ok(Verb::List {
                socket: "/s".into()
            })
        );
    }

    #[test]
    fn parse_names_what_is_wrong() {
        for (bad, needle) in [
            (vec![], "usage"),
            (vec!["nope"], "unknown verb 'nope'"),
            (vec!["register", "--socket", "/s"], "--name"),
            (vec!["register", "--name", "a"], "--socket"),
            (vec!["list"], "--socket"),
            (vec!["list", "--hold", "--socket", "/s"], "--hold"),
            (vec!["idle", "--socket", "/s"], "--socket"),
        ] {
            let err = parse(&args(&bad)).unwrap_err();
            assert!(err.contains(needle), "{bad:?}: {err}");
        }
    }

    async fn orchestrator(dir: &Path) -> PathBuf {
        let socket = dir.join("orchestrator.sock");
        let store = Arc::new(JsonRegistryStore::in_data_dir(dir));
        tokio::spawn(run_with(socket.clone(), Arc::new(SystemClock), store));
        for _ in 0..200 {
            if connect(&socket).await.is_ok() {
                return socket;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!("the orchestrator never listened");
    }

    async fn daemons(socket: &Path) -> Vec<serde_json::Value> {
        let listing: serde_json::Value =
            serde_json::from_str(&list(socket).await.unwrap()).unwrap();
        listing["daemons"].as_array().cloned().unwrap_or_default()
    }

    /// A dropped link lists offline once the liveness deadline (9 s) and a check interval pass.
    async fn health_of(socket: &Path, id: &str, want: &str) {
        for _ in 0..750 {
            if daemons(socket)
                .await
                .iter()
                .any(|d| d["id"] == id && d["health"] == want)
            {
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!("{id} never listed {want}: {:?}", daemons(socket).await);
    }

    #[tokio::test]
    async fn list_of_a_fresh_orchestrator_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let socket = orchestrator(dir.path()).await;
        assert!(daemons(&socket).await.is_empty());
    }

    #[tokio::test]
    async fn register_without_hold_lists_the_daemon_offline() {
        let dir = tempfile::tempdir().unwrap();
        let socket = orchestrator(dir.path()).await;
        register(&socket, "A", false, tokio::io::empty())
            .await
            .unwrap();
        health_of(&socket, "a", "offline").await;
        let listed = daemons(&socket).await;
        assert_eq!(listed[0]["name"], "A", "{listed:?}");
    }

    #[tokio::test]
    async fn register_with_hold_stays_online_until_stdin_closes() {
        let dir = tempfile::tempdir().unwrap();
        let socket = orchestrator(dir.path()).await;
        let (stdin_tx, stdin_rx) = tokio::io::duplex(8);
        let held = tokio::spawn({
            let socket = socket.clone();
            async move { register(&socket, "b", true, stdin_rx).await }
        });
        health_of(&socket, "b", "online").await;
        assert!(!held.is_finished());
        drop(stdin_tx);
        held.await.unwrap().unwrap();
        health_of(&socket, "b", "offline").await;
    }

    #[tokio::test]
    async fn two_held_daemons_are_online_together() {
        let dir = tempfile::tempdir().unwrap();
        let socket = orchestrator(dir.path()).await;
        let (_keep_a, a_in) = tokio::io::duplex(8);
        let (_keep_b, b_in) = tokio::io::duplex(8);
        for (name, stdin) in [("a", a_in), ("b", b_in)] {
            let socket = socket.clone();
            tokio::spawn(async move { register(&socket, name, true, stdin).await });
        }
        health_of(&socket, "a", "online").await;
        health_of(&socket, "b", "online").await;
    }

    #[tokio::test]
    async fn register_rejects_a_name_that_is_no_daemon_id() {
        let dir = tempfile::tempdir().unwrap();
        let err = register(
            &dir.path().join("none.sock"),
            "bad name",
            false,
            tokio::io::empty(),
        )
        .await
        .unwrap_err();
        assert!(err.to_string().contains("--name"), "{err}");
    }

    #[tokio::test]
    async fn list_without_an_orchestrator_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        assert!(list(&dir.path().join("none.sock")).await.is_err());
    }
}
