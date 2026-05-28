use super::*;
use anyhow::Result;

pub(super) async fn get_protobuf(
    broker: &TorBroker,
    endpoint: &str,
    node_id: &str,
    path: &str,
    bearer_token: &str,
) -> Result<(u16, Vec<u8>)> {
    let response = remote_exchange::remote_http_exchange(
        broker,
        BrokerRemoteHttpRequest {
            endpoint,
            node_id,
            bearer_token,
            method: "GET",
            path,
            headers: &[],
            body: &[],
        },
    )
    .await?;
    Ok((response.status, response.body))
}
