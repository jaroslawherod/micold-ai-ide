//! The two services the pod reaches on the host side of the docker network `kind` (feature 686,
//! research K12): a package source serving a stand-in `claude` as the pinned npm package, and a git
//! remote served by `git daemon`.
#![cfg(feature = "kube-real-runtime")]

use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// The port the test image's npm config names (`packaging/kube/test.Containerfile`).
pub const REGISTRY_PORT: u16 = 4873;
/// The git daemon's port.
pub const GIT_PORT: u16 = 9418;

/// The stand-in `claude`: announces itself, then ticks, and echoes what is typed.
pub const STAND_IN: &str = "#!/bin/bash\necho STAND-IN-AI-READY\ni=0\nwhile :; do i=$((i+1)); echo TICK-$i; if read -t 1 line; then echo ECHO-$line; fi; done\n";

/// The package source: a minimal npm registry for one package.
pub struct Registry {
    available: Arc<AtomicBool>,
    tarball_gets: Arc<AtomicUsize>,
}

impl Registry {
    /// Build the stand-in package and serve it on every interface (the pod reaches it through the
    /// docker gateway).
    pub fn start(dir: &Path) -> Result<Self, String> {
        let table = micold_core::provision::ai_cli::AiCliTable::pinned();
        let row = table
            .row(micold_core::session::AiCli::ClaudeCode)
            .ok_or("the pinned table has no Claude Code row")?;
        let package = row.package.clone();
        let version = match &row.source {
            micold_core::provision::ai_cli::Source::Registry { version } => version.clone(),
            _ => return Err("the pinned Claude Code row is not a registry package".into()),
        };
        let tarball = build_package(dir, &package, &version)?;
        let available = Arc::new(AtomicBool::new(true));
        let tarball_gets = Arc::new(AtomicUsize::new(0));
        let (flag, count) = (available.clone(), tarball_gets.clone());
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .expect("registry runtime");
            rt.block_on(async move {
                let listener = match TcpListener::bind(("0.0.0.0", REGISTRY_PORT)).await {
                    Ok(l) => {
                        let _ = tx.send(Ok(()));
                        l
                    }
                    Err(e) => {
                        let _ = tx.send(Err(format!(
                            "bind the package source on {REGISTRY_PORT}: {e}"
                        )));
                        return;
                    }
                };
                loop {
                    let Ok((stream, _)) = listener.accept().await else {
                        continue;
                    };
                    let (flag, count) = (flag.clone(), count.clone());
                    let (package, version, tarball) =
                        (package.clone(), version.clone(), tarball.clone());
                    tokio::spawn(async move {
                        serve(stream, &flag, &count, &package, &version, &tarball).await;
                    });
                }
            });
        });
        rx.recv().map_err(|e| e.to_string())??;
        Ok(Self {
            available,
            tarball_gets,
        })
    }

    /// Make the source answer (`true`) or drop every connection (`false`): an unreachable source.
    pub fn set_available(&self, up: bool) {
        self.available.store(up, Ordering::SeqCst);
    }

    /// How many tarballs have been downloaded (one per install of the CLI).
    pub fn downloads(&self) -> usize {
        self.tarball_gets.load(Ordering::SeqCst)
    }
}

fn build_package(dir: &Path, package: &str, version: &str) -> Result<Vec<u8>, String> {
    let src = dir.join("pkg-src");
    let pkg = src.join("package");
    std::fs::create_dir_all(pkg.join("bin")).map_err(|e| e.to_string())?;
    std::fs::write(
        pkg.join("package.json"),
        format!(r#"{{"name":"{package}","version":"{version}","bin":{{"claude":"bin/claude"}}}}"#),
    )
    .map_err(|e| e.to_string())?;
    let bin = pkg.join("bin/claude");
    std::fs::write(&bin, STAND_IN).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| e.to_string())?;
    }
    let out = dir.join("claude.tgz");
    let status = Command::new("tar")
        .arg("czf")
        .arg(&out)
        .arg("-C")
        .arg(&src)
        .arg("package")
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("tar failed to pack the stand-in package".into());
    }
    std::fs::read(&out).map_err(|e| e.to_string())
}

async fn serve(
    mut stream: tokio::net::TcpStream,
    available: &AtomicBool,
    downloads: &AtomicUsize,
    package: &str,
    version: &str,
    tarball: &[u8],
) {
    if !available.load(Ordering::SeqCst) {
        return; // drops the connection
    }
    let mut buf = vec![0u8; 8192];
    let Ok(n) = stream.read(&mut buf).await else {
        return;
    };
    let request = String::from_utf8_lossy(&buf[..n]).to_string();
    let path = request
        .split_whitespace()
        .nth(1)
        .unwrap_or("/")
        .to_ascii_lowercase()
        .replace("%2f", "/");
    let (status, content_type, body): (&str, &str, Vec<u8>) = if path
        == format!("/{}", package.to_ascii_lowercase())
    {
        let doc = format!(
            r#"{{"name":"{package}","dist-tags":{{"latest":"{version}"}},"versions":{{"{version}":{{"name":"{package}","version":"{version}","bin":{{"claude":"bin/claude"}},"dist":{{"tarball":"http://{}:{REGISTRY_PORT}/tarball/claude.tgz"}}}}}}}}"#,
            super::HOST_IP
        );
        ("200 OK", "application/json", doc.into_bytes())
    } else if path == "/tarball/claude.tgz" {
        downloads.fetch_add(1, Ordering::SeqCst);
        ("200 OK", "application/octet-stream", tarball.to_vec())
    } else {
        (
            "404 Not Found",
            "application/json",
            b"{\"error\":\"not found\"}".to_vec(),
        )
    };
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(head.as_bytes()).await;
    let _ = stream.write_all(&body).await;
    let _ = stream.shutdown().await;
}

/// A bare repository with one commit, served by `git daemon` on every interface.
pub struct GitRemote {
    child: Option<Child>,
}

impl GitRemote {
    pub fn start(dir: &Path) -> Result<Self, String> {
        // A fixed directory: the daemon outlives this test process (each test binary is one), and
        // the next binary reuses it while it listens.
        let base = std::env::temp_dir().join("micold-kube-test-git");
        let seed = dir.join("seed");
        if std::net::TcpStream::connect(("127.0.0.1", GIT_PORT)).is_ok()
            && base.join("repo.git").exists()
        {
            return Ok(Self { child: None });
        }
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&seed).map_err(|e| e.to_string())?;
        let git = |cwd: &Path, args: &[&str]| -> Result<(), String> {
            let status = Command::new("git")
                .args([
                    "-c",
                    "user.name=t",
                    "-c",
                    "user.email=t@t",
                    "-c",
                    "init.defaultBranch=main",
                ])
                .args(args)
                .current_dir(cwd)
                .status()
                .map_err(|e| e.to_string())?;
            status
                .success()
                .then_some(())
                .ok_or_else(|| format!("git {args:?} failed"))
        };
        git(&seed, &["init", "-q"])?;
        std::fs::write(seed.join("seed.txt"), "kube real\n").map_err(|e| e.to_string())?;
        git(&seed, &["add", "."])?;
        git(&seed, &["commit", "-q", "-m", "init"])?;
        let bare = base.join("repo.git");
        std::fs::create_dir_all(&base).map_err(|e| e.to_string())?;
        git(
            &base,
            &[
                "clone",
                "-q",
                "--bare",
                seed.to_str().unwrap(),
                bare.to_str().unwrap(),
            ],
        )?;
        let child = Command::new("git")
            .args(["daemon", "--reuseaddr", "--export-all", "--listen=0.0.0.0"])
            .arg(format!("--port={GIT_PORT}"))
            .arg(format!("--base-path={}", base.display()))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("git daemon: {e}"))?;
        std::thread::sleep(std::time::Duration::from_millis(500));
        Ok(Self { child: Some(child) })
    }

    /// The remote's URL as the pod reaches it.
    pub fn url(&self) -> String {
        format!("git://{}:{GIT_PORT}/repo.git", super::HOST_IP)
    }
}

impl Drop for GitRemote {
    fn drop(&mut self) {
        // Left running for the next test binary; `mise run test-kube` and the CI job stop it.
        let _ = self.child.take();
    }
}
