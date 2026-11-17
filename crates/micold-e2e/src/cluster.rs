//! The `kubectl` calls the suites make: `exec` into a container, and `port-forward` to a Service.
//! Every call names the harness's context and namespace, so a suite never acts on the user's
//! current context.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::process::{Child, Command, Output, Stdio};
use std::time::Duration;

/// The Service in front of the orchestrator's ops port (`templates/service.yaml`, release `micold`).
pub const OPS_SERVICE: &str = "svc/micold-orchestrator";
/// The ops port in the chart's values (`ops.port`).
pub const OPS_PORT: u16 = 9090;

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
