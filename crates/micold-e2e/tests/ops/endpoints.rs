//! The ops contract (specs/1981-kube-e2e/contracts/ops-http.md) on the deployed listener.

use micold_e2e::cluster::{http_get, OPS_PORT, OPS_SERVICE};
use micold_e2e::env::cluster_or_skip;

#[test]
fn an_unknown_path_answers_404_and_the_probes_still_pass() {
    let Some(cluster) = cluster_or_skip("ops::endpoints") else {
        return;
    };
    let forward = cluster
        .port_forward(OPS_SERVICE, OPS_PORT)
        .expect("kubectl port-forward");
    let (status, body) = http_get(forward.addr, "/nope").expect("GET /nope");
    assert_eq!(status, 404, "GET /nope: {body}");
    let (status, body) = http_get(forward.addr, "/readyz").expect("GET /readyz");
    assert_eq!((status, body.trim()), (200, "ready"));
}
