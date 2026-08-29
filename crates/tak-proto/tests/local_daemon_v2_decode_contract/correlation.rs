use tak_proto::local_daemon::v2::decode_request;

#[test]
fn incomplete_and_multi_value_frames_never_correlate() {
    let cases = [
        r#"{"protocol_version":2,"request_id":"partial","operation":{"type":"ListRuns"}"#,
        concat!(
            r#"{"protocol_version":2,"request_id":"first","operation":{"type":"ListRuns"}}"#,
            r#" {"request_id":"second"}"#,
        ),
        concat!(
            r#"{"protocol_version":2,"request_id":"first","operation":{"type":"ListRuns"}}"#,
            " trailing",
        ),
    ];

    for raw in cases {
        let error = decode_request(raw).expect_err("incomplete frame must fail");
        assert_eq!(error.request_id, None, "unsafe correlation for {raw}");
    }
}
