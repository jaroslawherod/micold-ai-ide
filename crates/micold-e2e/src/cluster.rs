//! The `kubectl` calls the suites make: `exec` into a container, and `port-forward` to a Service.
//! Every call names the harness's context and namespace, so a suite never acts on the user's
//! current context.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use crate::listing::{parse_listing, Daemon};

/// The Service in front of the orchestrator's ops port (`templates/service.yaml`, release `micold`).
pub const OPS_SERVICE: &str = "svc/micold-orchestrator";
/// The ops port in the chart's values (`ops.port`).
pub const OPS_PORT: u16 = 9090;

/// The orchestrator pod's labels (`templates/deployment.yaml`, release `micold`).
pub const ORCHESTRATOR_SELECTOR: &str =
    "app.kubernetes.io/instance=micold,app.kubernetes.io/component=orchestrator";
/// The orchestrator's container, and the test daemon beside it (`testDaemon.enabled`).
pub const ORCHESTRATOR_CONTAINER: &str = "orchestrator";
pub const TEST_DAEMON_CONTAINER: &str = "test-daemon";
/// The orchestrator's link socket, in the volume both containers share.
pub const LINK_SOCKET: &str = "/run/micold/orchestrator.sock";
const MICOLD_DAEMON: &str = "/micold-daemon";

/// The cluster a suite runs against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cluster {
    pub namespace: String,
    pub context: String,
}

impl Cluster {
    /// `kubectl --context <ctx> -n <ns> <args>`, not yet run.
    pub fn kubectl<I, S>(&self, args: I) -> Command
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        let mut cmd = Command::new("kubectl");
        cmd.arg("--context")
            .arg(&self.context)
            .arg("-n")
            .arg(&self.namespace)
            .args(args);
        cmd
    }

    /// Runs `kubectl exec <target> -c <container> -- <command...>` and returns its output.
    pub fn exec(&self, target: &str, container: &str, command: &[&str]) -> std::io::Result<Output> {
        let mut args = vec!["exec", target, "-c", container, "--"];
        args.extend_from_slice(command);
        self.kubectl(args).output()
    }

    /// The name of the orchestrator pod that is not being deleted; an error when there is none.
    pub fn orchestrator_pod(&self) -> std::io::Result<String> {
        let out = self
            .kubectl([
                "get",
                "pods",
                "-l",
                ORCHESTRATOR_SELECTOR,
                "-o",
                POD_STATE_TEMPLATE,
            ])
            .output()?;
        pod_named(&String::from_utf8_lossy(&out.stdout), false)
            .ok_or_else(|| other(format!("no orchestrator pod: {}", stderr_of(&out))))
    }

    /// Waits until an orchestrator pod that is not being deleted is Ready.
    pub fn wait_ready(&self, within: Duration) -> std::io::Result<String> {
        let started = Instant::now();
        loop {
            let out = self
                .kubectl([
                    "get",
                    "pods",
                    "-l",
                    ORCHESTRATOR_SELECTOR,
                    "-o",
                    POD_STATE_TEMPLATE,
                ])
                .output()?;
            if let Some(pod) = pod_named(&String::from_utf8_lossy(&out.stdout), true) {
                return Ok(pod);
            }
            if started.elapsed() > within {
                return Err(other(format!(
                    "no Ready orchestrator pod within {within:?}"
                )));
            }
            std::thread::sleep(Duration::from_secs(2));
        }
    }

    /// Whether some orchestrator pod (not being deleted) is Ready right now.
    pub fn is_ready(&self) -> std::io::Result<bool> {
        let out = self
            .kubectl([
                "get",
                "pods",
                "-l",
                ORCHESTRATOR_SELECTOR,
                "-o",
                POD_STATE_TEMPLATE,
            ])
            .output()?;
        Ok(pod_named(&String::from_utf8_lossy(&out.stdout), true).is_some())
    }

    /// Deletes the orchestrator pod and waits until it is gone; the Deployment starts another.
    pub fn delete_orchestrator_pod(&self) -> std::io::Result<()> {
        let pod = self.orchestrator_pod()?;
        let out = self
            .kubectl(["delete", "pod", &pod, "--wait=true", "--timeout=120s"])
            .output()?;
        if out.status.success() {
            Ok(())
        } else {
            Err(other(format!(
                "kubectl delete pod {pod}: {}",
                stderr_of(&out)
            )))
        }
    }

    /// A value of the orchestrator pod, by `kubectl` JSONPath (`{.spec.containers[0].image}`).
    pub fn pod_value(&self, jsonpath: &str) -> std::io::Result<String> {
        let pod = self.orchestrator_pod()?;
        let out = self
            .kubectl(["get", "pod", &pod, "-o", &format!("jsonpath={jsonpath}")])
            .output()?;
        if out.status.success() {
            Ok(String::from_utf8_lossy(&out.stdout).into_owned())
        } else {
            Err(other(format!("kubectl get pod {pod}: {}", stderr_of(&out))))
        }
    }

    /// The log of `container` in the orchestrator pod; `previous` reads the one before its last
    /// restart.
    pub fn logs(&self, container: &str, previous: bool) -> std::io::Result<String> {
        let pod = self.orchestrator_pod()?;
        let mut args = vec!["logs", &pod, "-c", container];
        if previous {
            args.push("--previous");
        }
        let out = self.kubectl(args).output()?;
        Ok(format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ))
    }

    /// `e2e-link <verb args>` in the test daemon, against the orchestrator's socket.
    fn link_command(&self, pod: &str, interactive: bool, verb: &[&str]) -> Command {
        let mut args = vec!["exec"];
        if interactive {
            args.push("-i");
        }
        args.extend([
            pod,
            "-c",
            TEST_DAEMON_CONTAINER,
            "--",
            MICOLD_DAEMON,
            "e2e-link",
        ]);
        args.extend_from_slice(verb);
        args.extend(["--socket", LINK_SOCKET]);
        self.kubectl(args)
    }

    /// The daemons the orchestrator lists, read through the test daemon.
    pub fn list_daemons(&self) -> std::io::Result<Vec<Daemon>> {
        let pod = self.orchestrator_pod()?;
        let out = self.link_command(&pod, false, &["list"]).output()?;
        if !out.status.success() {
            return Err(other(format!("e2e-link list: {}", stderr_of(&out))));
        }
        parse_listing(&String::from_utf8_lossy(&out.stdout)).map_err(other)
    }

    /// Links a daemon named `name` and lets go at once, so the orchestrator lists it offline.
    pub fn register(&self, name: &str) -> std::io::Result<()> {
        let pod = self.orchestrator_pod()?;
        let out = self
            .link_command(&pod, false, &["register", "--name", name])
            .output()?;
        if out.status.success() {
            Ok(())
        } else {
            Err(other(format!(
                "e2e-link register {name}: {}",
                stderr_of(&out)
            )))
        }
    }

    /// Links a daemon named `name` and keeps it linked until the returned guard is dropped.
    pub fn hold(&self, name: &str) -> std::io::Result<HeldDaemon> {
        let pod = self.orchestrator_pod()?;
        let child = self
            .link_command(&pod, true, &["register", "--name", name, "--hold"])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()?;
        Ok(HeldDaemon { child })
    }

    /// Polls the listing until daemon `id` shows `health` (`online` or `offline`).
    pub fn wait_for_daemon(
        &self,
        id: &str,
        health: &str,
        within: Duration,
    ) -> std::io::Result<Daemon> {
        let started = Instant::now();
        loop {
            let daemons = self.list_daemons()?;
            if let Some(d) = daemons.iter().find(|d| d.id == id && d.health == health) {
                return Ok(d.clone());
            }
            if started.elapsed() > within {
                return Err(other(format!(
                    "daemon {id} never listed {health} within {within:?}: {daemons:?}"
                )));
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    }

    /// Starts `kubectl port-forward <target> :<port>` and waits for the local address it prints.
    pub fn port_forward(&self, target: &str, port: u16) -> std::io::Result<PortForward> {
        let mut child = self
            .kubectl(["port-forward", target, &format!(":{port}")])
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;
        let stdout = child.stdout.take().expect("piped stdout");
        let mut lines = BufReader::new(stdout).lines();
        let addr = loop {
            let Some(line) = lines.next() else {
                let _ = child.kill();
                let status = child.wait()?;
                return Err(std::io::Error::other(format!(
                    "kubectl port-forward {target} exited ({status}) before forwarding"
                )));
            };
            if let Some(addr) = forwarded_addr(&line?) {
                break addr;
            }
        };
        // Keep draining stdout so kubectl never blocks on a full pipe ("Handling connection ...").
        std::thread::spawn(move || lines.for_each(drop));
        Ok(PortForward { child, addr })
    }
}

/// Prints, for each pod: `<name> <deletionTimestamp> <Ready status>`, one line per pod.
const POD_STATE_TEMPLATE: &str = r#"jsonpath={range .items[*]}{.metadata.name}{" "}{.metadata.deletionTimestamp}{"|"}{.status.conditions[?(@.type=="Ready")].status}{"\n"}{end}"#;

fn other(message: impl Into<String>) -> std::io::Error {
    std::io::Error::other(message.into())
}

fn stderr_of(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).trim().to_string()
}

/// The first pod of [`POD_STATE_TEMPLATE`]'s output that is not being deleted, and with
/// `ready_only` is Ready.
pub fn pod_named(output: &str, ready_only: bool) -> Option<String> {
    output.lines().find_map(|line| {
        let (head, ready) = line.split_once('|')?;
        let mut parts = head.split_whitespace();
        let name = parts.next()?;
        let deleting = parts.next().is_some();
        (!deleting && (!ready_only || ready.trim() == "True")).then(|| name.to_string())
    })
}

/// A daemon the test daemon keeps linked; killed on drop (its stdin closes, so it lets go).
pub struct HeldDaemon {
    child: Child,
}

impl Drop for HeldDaemon {
    fn drop(&mut self) {
        drop(self.child.stdin.take());
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The local address in a `kubectl port-forward` line such as
/// `Forwarding from 127.0.0.1:41235 -> 9090`; `None` for any other line (the `[::1]` one too).
pub fn forwarded_addr(line: &str) -> Option<SocketAddr> {
    let rest = line.trim().strip_prefix("Forwarding from ")?;
    let (local, _) = rest.split_once(" -> ")?;
    let addr: SocketAddr = local.parse().ok()?;
    addr.is_ipv4().then_some(addr)
}

/// A running `kubectl port-forward`; killed on drop.
pub struct PortForward {
    child: Child,
    pub addr: SocketAddr,
}

impl Drop for PortForward {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// `GET <path>` over plain HTTP/1.1 to `addr`; the status code and the body.
pub fn http_get(addr: SocketAddr, path: &str) -> std::io::Result<(u16, String)> {
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(10))?;
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
    )?;
    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    parse_response(&response)
        .ok_or_else(|| std::io::Error::other(format!("not an HTTP response: {response:?}")))
}

/// The status code and body of a raw HTTP/1.1 response.
pub fn parse_response(response: &str) -> Option<(u16, String)> {
    let status = response.split_whitespace().nth(1)?.parse().ok()?;
    let body = response
        .split_once("\r\n\r\n")
        .map(|(_, b)| b)
        .unwrap_or("");
    Some((status, body.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forwarded_addr_reads_the_ipv4_line_only() {
        assert_eq!(
            forwarded_addr("Forwarding from 127.0.0.1:41235 -> 9090"),
            Some("127.0.0.1:41235".parse().unwrap())
        );
        assert_eq!(forwarded_addr("Forwarding from [::1]:41235 -> 9090"), None);
        assert_eq!(forwarded_addr("Handling connection for 41235"), None);
    }

    #[test]
    fn parse_response_reads_status_and_body() {
        assert_eq!(
            parse_response(
                "HTTP/1.1 503 Service Unavailable\r\nContent-Length: 9\r\n\r\nnot ready"
            ),
            Some((503, "not ready".into()))
        );
        assert_eq!(parse_response("garbage"), None);
    }

    #[test]
    fn pod_named_skips_pods_being_deleted_and_unready_ones() {
        let out = "old 2026-01-01T00:00:00Z|True\nnew |False\n";
        assert_eq!(pod_named(out, false), Some("new".into()));
        assert_eq!(pod_named(out, true), None);
        assert_eq!(
            pod_named("old 2026-01-01T00:00:00Z|True\nnew |True\n", true),
            Some("new".into())
        );
        assert_eq!(pod_named("", false), None);
    }

    #[test]
    fn kubectl_names_context_and_namespace_first() {
        let c = Cluster {
            namespace: "ns".into(),
            context: "ctx".into(),
        };
        let cmd = c.kubectl(["get", "pods"]);
        let args: Vec<_> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(args, ["--context", "ctx", "-n", "ns", "get", "pods"]);
    }
}
