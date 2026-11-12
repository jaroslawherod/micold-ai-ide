//! #1724 T019: `ClientMsg::LinkPong` answers the orchestrator's probe; a daemon that hosts sessions
//! has no use for it and ignores it, so the connection stays up and answers the next real message.

use futures_util::{SinkExt, StreamExt};
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{ClientMsg, DaemonMsg};

use crate::support::conn::{connect, service};

#[tokio::test]
async fn a_stray_link_pong_is_ignored_and_the_connection_stays_up() {
    let store = tempfile::tempdir().unwrap();
    let state = service(store.path());
    let mut client = connect(&state).await;

    client
        .send(Frame::Control(ClientMsg::LinkPong { nonce: 41 }))
        .await
        .unwrap();
    client
        .send(Frame::Control(ClientMsg::Ping { nonce: 42 }))
        .await
        .unwrap();

    let next = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            match client.next().await.expect("the connection stays open") {
                Ok(Frame::Control(DaemonMsg::Pong { nonce })) => return nonce,
                Ok(Frame::Control(DaemonMsg::Refused { reason })) => {
                    panic!("the stray LinkPong was refused: {reason:?}")
                }
                _ => {}
            }
        }
    })
    .await
    .expect("the Ping after the LinkPong is answered");
    assert_eq!(next, 42);
}
