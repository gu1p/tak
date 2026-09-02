use crate::support::{
    worker_http::start_server,
    v2_worker_capacity::{cancel, dispatch, request, snapshot, wait_released, wait_terminal},
};

#[tokio::test]
async fn worker_v2_atomically_admits_capacity_and_releases_it_after_terminal_state() {
    let server = start_server().await;
    let initial = snapshot(&server).await;
    let slots = initial.capacity.execution_slots;
    let resources = (1, 1024 * 1024, slots);
    let blocker = request(
        ("run-capacity-a", "job-a", "fence-capacity-a"),
        resources,
        "sleep 30",
    );
    let successor = request(
        ("run-capacity-b", "job-b", "fence-capacity-b"),
        resources,
        "sleep 30",
    );

    let (first, second) = tokio::join!(dispatch(&server, &blocker), dispatch(&server, &successor));
    let (winner, loser) = match (first, second) {
        (202, 429) => (&blocker, &successor),
        (429, 202) => (&successor, &blocker),
        statuses => panic!("expected one admission and one rejection, got {statuses:?}"),
    };
    let used = snapshot(&server).await.usage;
    assert_eq!(used.cpu_millis, 1);
    assert_eq!(used.memory_bytes, 1024 * 1024);
    assert_eq!(used.execution_slots, slots);

    cancel(&server, winner).await;
    wait_terminal(&server, winner).await;
    wait_released(&server).await;
    assert_eq!(dispatch(&server, loser).await, 202);
    cancel(&server, loser).await;
    wait_terminal(&server, loser).await;
    wait_released(&server).await;
    let finisher = request(("run-finish", "job-finish", "fence-finish"), resources, "true");
    assert_eq!(dispatch(&server, &finisher).await, 202);
    wait_terminal(&server, &finisher).await;
    wait_released(&server).await;
    let released = snapshot(&server).await.usage;
    assert_eq!(released.cpu_millis, 0);
    assert_eq!(released.memory_bytes, 0);
    assert_eq!(released.execution_slots, 0);
}
