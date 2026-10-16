//! A fault-injecting TCP proxy between `ssh` and the stand-in `sshd` (feature 489, T043).
//!
//! `cut()` severs every open connection and refuses new ones, which is what a network drop looks
//! like to the client: `ssh` sees the connection die while the `sshd` side (and so the remote
//! daemon and its sessions) keeps running. `restore()` lets new connections through again.

use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Default)]
struct State {
    cut: AtomicBool,
    stop: AtomicBool,
    open: Mutex<Vec<TcpStream>>,
}

/// A running proxy; it stops accepting when dropped.
pub struct Proxy {
    /// The loopback port clients dial.
    pub port: u16,
    state: Arc<State>,
}

/// Copy `from` to `to` until either side closes, then close both.
fn pipe(mut from: TcpStream, mut to: TcpStream) {
    let mut buf = [0u8; 16 * 1024];
    while let Ok(n) = from.read(&mut buf) {
        if n == 0 || to.write_all(&buf[..n]).is_err() {
            break;
        }
    }
    let _ = from.shutdown(Shutdown::Both);
    let _ = to.shutdown(Shutdown::Both);
}

impl Proxy {
    /// Listen on a free loopback port and forward to `upstream` (dialled per connection, so the
    /// upstream may start later).
    pub fn start(upstream: u16) -> Result<Self, String> {
        let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        let state = Arc::new(State::default());
        let shared = state.clone();
        std::thread::spawn(move || {
            while !shared.stop.load(Ordering::SeqCst) {
                let Ok((client, _)) = listener.accept() else {
                    std::thread::sleep(Duration::from_millis(20));
                    continue;
                };
                let _ = client.set_nonblocking(false);
                if shared.cut.load(Ordering::SeqCst) {
                    continue; // dropped: the client sees a closed connection
                }
                let Ok(server) = TcpStream::connect(("127.0.0.1", upstream)) else {
                    continue;
                };
                let (Ok(c2), Ok(s2)) = (client.try_clone(), server.try_clone()) else {
                    continue;
                };
                let mut open = shared.open.lock().unwrap();
                open.push(client.try_clone().expect("clone"));
                open.push(server.try_clone().expect("clone"));
                drop(open);
                std::thread::spawn(move || pipe(client, server));
                std::thread::spawn(move || pipe(s2, c2));
            }
        });
        Ok(Self { port, state })
    }

    /// Sever every open connection and refuse new ones.
    pub fn cut(&self) {
        self.state.cut.store(true, Ordering::SeqCst);
        for s in self.state.open.lock().unwrap().drain(..) {
            let _ = s.shutdown(Shutdown::Both);
        }
    }

    /// Let new connections through again.
    pub fn restore(&self) {
        self.state.cut.store(false, Ordering::SeqCst);
    }
}

impl Drop for Proxy {
    fn drop(&mut self) {
        self.state.stop.store(true, Ordering::SeqCst);
        self.cut();
    }
}
