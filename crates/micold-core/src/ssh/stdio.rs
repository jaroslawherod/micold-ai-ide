//! The byte stream of an `ssh` child: its stdout is read, its stdin written (`Transport::Ssh`).
//!
//! Boxed halves rather than a concrete child, so a test can stand in for the remote end with an
//! in-memory duplex and the real process is one construction ([`SshStdio::from_child`]).

use std::io;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

type Reader = Box<dyn AsyncRead + Send + Unpin>;
type Writer = Box<dyn AsyncWrite + Send + Unpin>;

/// How much of the child's stderr is kept: the tail names the cause of a failure.
const STDERR_KEEP: usize = 4096;

/// The two halves of an `ssh` child's stdio, plus what keeps the child alive and its stderr tail.
pub struct SshStdio {
    reader: Reader,
    writer: Writer,
    /// Held so dropping the connection kills the child (`kill_on_drop`): at most one per daemon.
    _child: Option<tokio::process::Child>,
    stderr_tail: Arc<Mutex<String>>,
}

impl std::fmt::Debug for SshStdio {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SshStdio").finish_non_exhaustive()
    }
}

/// A handle on the child's stderr tail that outlives the stream it was taken from.
#[derive(Debug, Clone, Default)]
pub struct StderrTail(Arc<Mutex<String>>);

impl StderrTail {
    /// What the child wrote to stderr, most recent last, bounded.
    pub fn text(&self) -> String {
        self.0.lock().map(|g| g.clone()).unwrap_or_default()
    }
}

impl SshStdio {
    /// Wrap arbitrary halves (a stand-in remote end in tests).
    pub fn from_parts(
        reader: impl AsyncRead + Send + Unpin + 'static,
        writer: impl AsyncWrite + Send + Unpin + 'static,
    ) -> Self {
        Self {
            reader: Box::new(reader),
            writer: Box::new(writer),
            _child: None,
            stderr_tail: Arc::default(),
        }
    }

    /// Wrap a spawned child whose stdin, stdout and stderr are piped. Returns an error naming
    /// which pipe is missing, so a misconfigured spawn is loud.
    pub fn from_child(mut child: tokio::process::Child) -> io::Result<Self> {
        let missing = |what: &str| io::Error::other(format!("ssh child has no {what} pipe"));
        let stdin = child.stdin.take().ok_or_else(|| missing("stdin"))?;
        let stdout = child.stdout.take().ok_or_else(|| missing("stdout"))?;
        let tail: Arc<Mutex<String>> = Arc::default();
        if let Some(mut stderr) = child.stderr.take() {
            let sink = tail.clone();
            tokio::spawn(async move {
                use tokio::io::AsyncReadExt;
                let mut buf = [0u8; 512];
                while let Ok(n) = stderr.read(&mut buf).await {
                    if n == 0 {
                        break;
                    }
                    if let Ok(mut text) = sink.lock() {
                        text.push_str(&String::from_utf8_lossy(&buf[..n]));
                        let len = text.len();
                        if len > STDERR_KEEP {
                            let cut = (len - STDERR_KEEP..len)
                                .find(|i| text.is_char_boundary(*i))
                                .unwrap_or(len);
                            text.drain(..cut);
                        }
                    }
                }
            });
        }
        Ok(Self {
            reader: Box::new(stdout),
            writer: Box::new(stdin),
            _child: Some(child),
            stderr_tail: tail,
        })
    }

    /// A handle on the stderr tail, usable after the stream is consumed by the handshake.
    pub fn stderr_tail(&self) -> StderrTail {
        StderrTail(self.stderr_tail.clone())
    }
}

impl AsyncRead for SshStdio {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.reader).poll_read(cx, buf)
    }
}

impl AsyncWrite for SshStdio {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.writer).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.writer).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.writer).poll_shutdown(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn bytes_written_reach_the_far_end_and_bytes_read_come_from_it() {
        let (near, mut far) = tokio::io::duplex(64);
        let (r, w) = tokio::io::split(near);
        let mut stdio = SshStdio::from_parts(r, w);
        stdio.write_all(b"ping").await.unwrap();
        let mut got = [0u8; 4];
        far.read_exact(&mut got).await.unwrap();
        assert_eq!(&got, b"ping");
        far.write_all(b"pong").await.unwrap();
        let mut back = [0u8; 4];
        stdio.read_exact(&mut back).await.unwrap();
        assert_eq!(&back, b"pong");
    }
}
