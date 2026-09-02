use std::future::Future;

use crate::RunCancellation;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum DeadlineOutcome<T> {
    Cancelled,
    TimedOut,
    Completed(T),
}

pub(crate) async fn select_deadline_outcome<T>(
    completion: impl Future<Output = T>,
    deadline: impl Future<Output = ()>,
    cancellation: &RunCancellation,
) -> DeadlineOutcome<T> {
    tokio::pin!(completion);
    tokio::pin!(deadline);
    tokio::select! {
        biased;
        _ = cancellation.cancelled() => DeadlineOutcome::Cancelled,
        _ = &mut deadline => DeadlineOutcome::TimedOut,
        result = &mut completion => DeadlineOutcome::Completed(result),
    }
}
