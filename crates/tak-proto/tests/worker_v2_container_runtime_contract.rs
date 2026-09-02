use serde_json::json;
use tak_core::v2::TaskRuntime;
use tak_proto::worker_v2::{
    DispatchAttemptRequest, decode_dispatch_request, encode_dispatch_request, payload_digest,
};

use crate::worker_v2_attempt_support::{payload, request};

#[test]
fn worker_dispatch_preserves_container_mounts_and_environment_but_rejects_command() {
    let configured = configured_runtime_request();
    let decoded = decode_dispatch_request(&encode_dispatch_request(&configured).unwrap()).unwrap();
    let runtime = serde_json::to_value(&decoded.payload.tasks[0].runtime).unwrap();
    assert_eq!(runtime["env"]["ORDER"], "runtime");
    assert_eq!(runtime["mounts"][0]["source"], "cache/input");
    assert_eq!(runtime["mounts"][0]["target"], "/var/cache/input");
    assert_eq!(runtime["mounts"][0]["read_only"], true);

    let mut removed_command = serde_json::to_value(configured).unwrap();
    removed_command["payload"]["tasks"][0]["runtime"]["command"] =
        json!(["sh", "-c", "echo obsolete"]);
    let error = decode_dispatch_request(&serde_json::to_vec(&removed_command).unwrap())
        .unwrap_err()
        .to_string();
    assert!(error.contains("command"), "{error}");
}

fn configured_runtime_request() -> DispatchAttemptRequest {
    let mut request = request(payload());
    request.payload.tasks[0].runtime = Some(
        serde_json::from_value::<TaskRuntime>(json!({
            "kind": "container",
            "source": {"kind": "image", "image": "alpine:3.20"},
            "mounts": [{
                "source": "cache/input",
                "target": "/var/cache/input",
                "read_only": true,
            }],
            "env": {"ORDER": "runtime"},
        }))
        .unwrap(),
    );
    request.payload_digest = payload_digest(&request.payload).unwrap();
    request
}
