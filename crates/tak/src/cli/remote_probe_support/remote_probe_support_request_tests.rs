#![cfg(test)]

use super::http::build_get_request;

#[test]
fn get_request_omits_blank_bearer_tokens_and_trims_present_values() {
    let without_auth =
        build_get_request("/v1/node/info", "builder-a.onion:80", "   ").expect("request");
    assert!(
        !without_auth
            .headers()
            .contains_key(hyper::header::AUTHORIZATION),
        "blank bearer tokens should omit Authorization"
    );

    let with_auth = build_get_request("/v1/node/info", "builder-a.onion:80", "  secret-token  ")
        .expect("request");
    assert_eq!(
        with_auth
            .headers()
            .get(hyper::header::AUTHORIZATION)
            .expect("authorization header"),
        "Bearer secret-token"
    );
}
