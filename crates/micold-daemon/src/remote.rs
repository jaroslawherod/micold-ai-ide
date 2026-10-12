//! The remote helper: `micold-daemon remote bridge | prune` (feature 489, contracts/ssh-transport.md).
//!
//! The client runs these over `ssh` on the host this daemon lives on. `bridge` is the whole
//! transport: it connects the `ssh` child's stdin and stdout to the daemon's own local endpoint,
//! starting the daemon first when none is listening. It never speaks the protocol and never writes
//! anything to stdout but relayed bytes. Like `status-line`, it runs before the runtime, the log and
//! the singleton of a daemon.

use std::ffi::OsString;
use std::io;
use std::path::Path;
use std::time::{Duration, Instant};

use micold_core::connect::{dial, SPAWN_TIMEOUT};
use micold_core::endpoint::Endpoint;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};

/// How long a spawned daemon is given before another is spawned, as `connect_or_spawn`.
const RESPAWN_AFTER: Duration = Duration::from_secs(1);

/// Entry point for `remote <subcommand> ...`: the process exit code.
pub fn main(args: &[OsString]) -> i32 {
    let sub = args.first().and_then(|a| a.to_str()).unwrap_or("");
    match sub {
        "bridge" => match run_bridge() {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("micold-daemon remote bridge: {e}");
                1
            }
        },
        "prune" => {
            let keep = args
                .get(1)
                .filter(|a| a.to_str() == Some("--keep"))
                .and_then(|_| args.get(2))
                .and_then(|n| n.to_str())
                .and_then(|n| n.parse::<usize>().ok());
            let Some(keep) = keep else {
                eprintln!("usage: micold-daemon remote prune --keep <n>");
                return 2;
            };
            match prune_self(keep) {
                Ok(()) => 0,
                Err(e) => {
                    eprintln!("micold-daemon remote prune: {e}");
                    1
                }
            }
        }
        _ => {
            eprintln!("usage: micold-daemon remote bridge | prune --keep <n>");
            2
        }
    }
}

fn run_bridge() -> io::Result<()> {
    // The daemon this starts must bind the pinned endpoint the bridge resolved, which ignores
    // `XDG_RUNTIME_DIR`; removing it here, while single-threaded, makes the daemon's own `resolve`
    // pick the same one (research R2).
    std::env::remove_var("XDG_RUNTIME_DIR");
    let endpoint = micold_core::endpoint::resolve_pinned()?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(bridge(
        &endpoint,
        tokio::io::stdin(),
        tokio::io::stdout(),
        || micold_core::spawn::spawn_detached_daemon().map(|_| ()),
        SPAWN_TIMEOUT,
    ))
}

/// Relay `input` to the daemon at `endpoint` and its answers to `output`, byte for byte, until
/// either side closes. `start` spawns a detached daemon; it is called when nothing accepts, again
/// after [`RESPAWN_AFTER`], until `timeout`.
pub async fn bridge<I, O>(
    endpoint: &Endpoint,
    mut input: I,
    mut output: O,
    mut start: impl FnMut() -> io::Result<()>,
    timeout: Duration,
) -> io::Result<()>
where
    I: AsyncRead + Unpin,
    O: AsyncWrite + Unpin,
{
    let deadline = Instant::now() + timeout;
    let mut backoff = Duration::from_millis(10);
    let mut started: Option<Instant> = None;
    let stream = loop {
        if let Some(stream) = dial(endpoint).await? {
            break stream;
        }
        if started.is_none_or(|at| at.elapsed() >= RESPAWN_AFTER) {
            start()?;
            started = Some(Instant::now());
        }
        if Instant::now() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!(
                    "the daemon did not start accepting on {} within {timeout:?}",
                    endpoint.socket_path.display()
                ),
            ));
        }
        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(Duration::from_millis(250));
    };
    let (mut from_daemon, mut to_daemon) = tokio::io::split(stream);
    tokio::select! {
        _ = tokio::io::copy(&mut input, &mut to_daemon) => {}
        _ = tokio::io::copy(&mut from_daemon, &mut output) => {}
    }
    output.flush().await?;
    Ok(())
}

fn prune_self(keep: usize) -> io::Result<()> {
    let exe = std::env::current_exe()?;
    let dir = exe
        .parent()
        .ok_or_else(|| io::Error::other("the helper has no directory"))?;
    let root = dir
        .parent()
        .ok_or_else(|| io::Error::other("the helper's directory has no parent"))?;
    prune(root, dir.file_name().unwrap_or_default(), keep)
}

/// Remove the build directories of `root` except `current` and the `keep - 1` most recently
/// modified others. A directory that cannot be removed is left, not an error: pruning is housekeeping.
pub fn prune(root: &Path, current: &std::ffi::OsStr, keep: usize) -> io::Result<()> {
    let mut others: Vec<(std::time::SystemTime, std::path::PathBuf)> = Vec::new();
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        if entry.file_name() == current || !entry.file_type()?.is_dir() {
            continue;
        }
        let modified = entry.metadata()?.modified().unwrap_or(std::time::UNIX_EPOCH);
        others.push((modified, entry.path()));
    }
    others.sort_by(|a, b| b.0.cmp(&a.0));
    for (_, path) in others.into_iter().skip(keep.saturating_sub(1)) {
        let _ = std::fs::remove_dir_all(path);
    }
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::UnixListener;

    fn endpoint(dir: &Path) -> Endpoint {
        Endpoint {
            socket_path: dir.join("d.sock"),
            lock_path: dir.join("d.lock"),
        }
    }

    /// An echo "daemon": answers whatever it reads with the same bytes.
    fn echo_daemon(listener: UnixListener) {
        tokio::spawn(async move {
            while let Ok((mut s, _)) = listener.accept().await {
                tokio::spawn(async move {
                    let mut buf = [0u8; 64];
                    while let Ok(n) = s.read(&mut buf).await {
                        if n == 0 || s.write_all(&buf[..n]).await.is_err() {
                            break;
                        }
                    }
                });
            }
        });
    }

    async fn relay_once(ep: &Endpoint, mut start: impl FnMut() -> io::Result<()>) -> Vec<u8> {
        let (mut client, near) = tokio::io::duplex(64);
        let (out_w, mut out_r) = tokio::io::duplex(64);
        let relay = bridge(ep, near, out_w, &mut start, Duration::from_secs(5));
        let user = async move {
            client.write_all(b"hello").await.unwrap();
            let mut got = [0u8; 5];
            out_r.read_exact(&mut got).await.unwrap();
            drop(client);
            got.to_vec()
        };
        let (done, got) = tokio::join!(relay, user);
        done.unwrap();
        got
    }

    #[tokio::test]
    async fn bytes_reach_a_running_daemon_and_come_back_without_starting_another() {
        let dir = tempfile::tempdir().unwrap();
        let ep = endpoint(dir.path());
        echo_daemon(UnixListener::bind(&ep.socket_path).unwrap());
        let starts = AtomicUsize::new(0);
        let got = relay_once(&ep, || {
            starts.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
        .await;
        assert_eq!(got, b"hello");
        assert_eq!(starts.load(Ordering::SeqCst), 0, "a running daemon is reused");
    }

    #[tokio::test]
    async fn a_daemon_is_started_when_none_listens() {
        let dir = tempfile::tempdir().unwrap();
        let ep = endpoint(dir.path());
        let starts = AtomicUsize::new(0);
        let path = ep.socket_path.clone();
        let got = relay_once(&ep, || {
            starts.fetch_add(1, Ordering::SeqCst);
            // Stand in for the spawned daemon: it binds the endpoint.
            echo_daemon(UnixListener::bind(&path)?);
            Ok(())
        })
        .await;
        assert_eq!(got, b"hello");
        assert_eq!(starts.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn no_daemon_ever_accepting_times_out_naming_the_endpoint() {
        let dir = tempfile::tempdir().unwrap();
        let ep = endpoint(dir.path());
        let (_keep, input) = tokio::io::duplex(8);
        let (output, _out) = tokio::io::duplex(8);
        let err = bridge(&ep, input, output, || Ok(()), Duration::from_millis(100))
            .await
            .unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::TimedOut);
        assert!(err.to_string().contains("d.sock"));
    }

    #[test]
    fn prune_keeps_the_current_build_and_the_newest_others() {
        let dir = tempfile::tempdir().unwrap();
        let now = std::time::SystemTime::now();
        for (name, age) in [("cur", 0u64), ("a", 10), ("b", 20), ("c", 30)] {
            let p = dir.path().join(name);
            std::fs::create_dir(&p).unwrap();
            let f = std::fs::File::open(&p).unwrap();
            f.set_modified(now - Duration::from_secs(age * 1000)).unwrap();
        }
        prune(dir.path(), std::ffi::OsStr::new("cur"), 2).unwrap();
        let mut left: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        left.sort();
        assert_eq!(left, ["a", "cur"], "current plus the newest other");
    }
}
