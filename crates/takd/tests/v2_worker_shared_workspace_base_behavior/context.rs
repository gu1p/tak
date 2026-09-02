use tak_proto::worker_v2::WorkerTerminalOutcome;

use crate::support::{
    v2_worker_shared::{context_archive, dispatch_with_context, send, wait_terminal},
    worker_http::start_server,
};

#[tokio::test]
async fn remote_shared_jobs_receive_their_own_context_and_shared_writes() {
    let server = start_server().await;
    let archive = context_archive();
    let first = dispatch_with_context(
        1,
        "context-1",
        "producer.txt",
        "test -f producer.txt; test ! -e consumer.txt; printf shared > shared.txt",
    );
    send(&server, &first, &archive).await;
    assert_eq!(
        wait_terminal(&server, &first)
            .await
            .terminal
            .unwrap()
            .outcome,
        WorkerTerminalOutcome::Succeeded
    );

    let second = dispatch_with_context(
        2,
        "context-2",
        "consumer.txt",
        "test -f consumer.txt; test ! -e producer.txt; test \"$(cat shared.txt)\" = shared",
    );
    send(&server, &second, &archive).await;
    assert_eq!(
        wait_terminal(&server, &second)
            .await
            .terminal
            .unwrap()
            .outcome,
        WorkerTerminalOutcome::Succeeded
    );
}
