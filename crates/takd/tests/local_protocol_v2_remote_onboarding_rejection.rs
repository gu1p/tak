use crate::support::raw_local_protocol::RawLocalProtocol;

#[tokio::test]
async fn direct_v1_onboarding_is_rejected_with_upgrade_together_guidance() {
    let mut daemon = RawLocalProtocol::start().await;
    let secret = "takd:v1:legacy-secret-material";
    let request = format!(
        r#"{{"protocol_version":2,"request_id":"legacy-add","operation":{{"type":"AddRemote","invite":"{secret}"}}}}"#
    );

    let response = daemon.exchange(&request).await;

    assert!(response.contains(r#""code":"remote_invite_unsupported""#));
    assert!(response.contains("upgrade tak, takd, and workers together"));
    assert!(
        !response.contains(secret),
        "response reflected onboarding secret"
    );
}
