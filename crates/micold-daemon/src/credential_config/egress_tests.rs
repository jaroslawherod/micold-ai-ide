//! `UreqTransport` honours the daemon's proxy and `NO_PROXY` (#1536, I16-2).

use std::sync::atomic::Ordering;

use micold_core::scoped_credential::SecretToken;

use super::{Transport, UreqTransport};
use crate::session_extras::egress_tests::listener;
use crate::session_extras::Egress;

fn status_through(no_proxy: Option<&str>) -> (u16, usize, usize) {
    let (proxy, proxied) = listener();
    let (origin, direct) = listener();
    let transport = UreqTransport {
        egress: Egress {
            https_proxy: Some(format!("http://{proxy}")),
            no_proxy: no_proxy.map(str::to_string),
            ..Default::default()
        },
    };
    // Through the stub proxy the call fails (it does not tunnel); only who was reached counts.
    let reply = transport.get(&format!("http://{origin}/x"), &SecretToken::new("t"));
    (
        reply.map_or(0, |r| r.status),
        proxied.load(Ordering::SeqCst),
        direct.load(Ordering::SeqCst),
    )
}

#[test]
fn the_transport_goes_through_the_proxy_unless_the_host_is_in_no_proxy() {
    let (_, proxied, direct) = status_through(None);
    assert_eq!((proxied, direct), (1, 0));
    assert_eq!(status_through(Some("127.0.0.1")), (200, 0, 1));
}
