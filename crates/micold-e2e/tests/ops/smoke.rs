//! Smoke: after `helm install --wait` the pod is Ready, so `/healthz` and `/readyz` both answer 200.

use micold_e2e::cluster::{http_get, OPS_PORT, OPS_SERVICE};
use micold_e2e::env::cluster_or_skip;

#[test]
fn healthz_and_readyz_answer_ok_through_the_service() {
    let Some(cluster) = cluster_or_skip("ops::smoke") else {
        return;
    };
    let forward = cluster
        .port_forward(OPS_SERVICE, OPS_PORT)
        .expect("kubectl port-forward");
    for path in ["/healthz", "/readyz"] {
        let (status, body) =
            http_get(forward.addr, path).unwrap_or_else(|e| panic!("GET {path}: {e}"));
        assert_eq!(status, 200, "GET {path}: {body}");
    }
}
