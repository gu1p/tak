fn assert_json_response(response: &str, expected: serde_json::Value) {
    assert!(response.ends_with('\n'), "response must be newline framed");
    assert_eq!(response.lines().count(), 1, "expected exactly one frame");
    let actual: serde_json::Value = serde_json::from_str(response).expect("response json");
    assert_eq!(actual, expected);
}

#[path = "local_protocol_v2_admission_contract/correlation_secrecy.rs"]
mod correlation_secrecy;
#[path = "local_protocol_v2_admission_contract/downgrade.rs"]
mod downgrade;
#[path = "local_protocol_v2_admission_contract/operations.rs"]
mod operations;
#[path = "local_protocol_v2_admission_contract/version_admission.rs"]
mod version_admission;
