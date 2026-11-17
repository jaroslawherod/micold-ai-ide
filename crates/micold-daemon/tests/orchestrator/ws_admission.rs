//! #1732 FR-004 hardening: an upgrade that never completes is dropped after the handshake timeout,
//! connections past the socket limit are dropped at accept, and an `Origin` that is not text is
//! refused like a foreign one.

use std::sync::Arc;
use std::time::Duration;

use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;

use crate::support::within;
use crate::ws_support::{fast, refused_status, serve, Scripted, Ws, ALLOWED};
use micold_daemon::orchestrator::ws::ApiOptions;

/// Read until the server closes the raw connection.
async fn dropped(mut stream: TcpStream) {
    let mut buf = [0u8; 256];
    loop {
        match within("the server drops the connection", stream.read(&mut buf)).await {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
    }
}

#[tokio::test]
async fn a_connection_that_sends_nothing_is_dropped_after_the_handshake_timeout() {
    let options = ApiOptions {
        handshake: Duration::from_millis(200),
        ..fast()
    };
    let addr = serve(Arc::new(Scripted::default()), options).await;
    dropped(TcpStream::connect(addr).await.unwrap()).await;
}

#[tokio::test]
async fn a_connection_past_the_socket_limit_is_dropped() {
    let options = ApiOptions {
        max_sockets: 1,
        ..fast()
    };
    let addr = serve(Arc::new(Scripted::default()), options).await;
    let mut first = Ws::connect(addr, Some(ALLOWED)).await.unwrap();
    assert!(Ws::connect(addr, Some(ALLOWED)).await.is_err());
    assert_eq!(
        first.ask(serde_json::json!({"id":1,"type":"ping"})).await["type"],
        "pong"
    );
}

#[tokio::test]
async fn an_origin_that_is_not_text_is_refused() {
    let addr = serve(Arc::new(Scripted::default()), fast()).await;
    let mut request = format!("ws://{addr}/api/ws").into_client_request().unwrap();
    request.headers_mut().insert(
        "origin",
        HeaderValue::from_bytes(b"http://\xff.example").unwrap(),
    );
    let stream = TcpStream::connect(addr).await.unwrap();
    let result = tokio_tungstenite::client_async(request, stream)
        .await
        .map(|(socket, _)| Ws(socket));
    assert_eq!(refused_status(result), 403);
}
