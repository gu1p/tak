#[allow(dead_code)]
#[path = "../src/cli/tasks_output/client.rs"]
mod client;

use std::io::Write;
use std::os::unix::net::UnixStream;

use prost::Message;
use tak_proto::NodeStatusResponse;

#[test]
fn a_completed_server_response_survives_peer_disconnect() {
    let (client, mut server) = UnixStream::pair().unwrap();
    let status = NodeStatusResponse {
        sampled_at_ms: 123,
        ..Default::default()
    };
    let body = tak_proto::worker_v2::encode_display_payload(&status.encode_to_vec()).unwrap();
    write!(
        server,
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .unwrap();
    server.write_all(&body).unwrap();
    drop(server);

    let response = client::read_live_status(client).unwrap();
    assert_eq!(response.sampled_at_ms, 123);
}
