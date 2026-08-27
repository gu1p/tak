#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RemoteFailureKind {
    Task,
    Infrastructure,
    ResourceCapacity,
    Cancellation,
    Unknown,
}

pub(crate) fn classify_remote_result_failure(
    success: bool,
    exit_code: Option<i32>,
    wire_kind: Option<i32>,
) -> Option<RemoteFailureKind> {
    if success {
        return None;
    }
    match wire_kind.and_then(|value| tak_proto::RemoteFailureKind::try_from(value).ok()) {
        Some(tak_proto::RemoteFailureKind::Task) => Some(RemoteFailureKind::Task),
        Some(tak_proto::RemoteFailureKind::ContainerOom) => Some(RemoteFailureKind::Infrastructure),
        Some(tak_proto::RemoteFailureKind::Infrastructure) if exit_code == Some(137) => {
            Some(RemoteFailureKind::Unknown)
        }
        Some(tak_proto::RemoteFailureKind::Infrastructure) => {
            Some(RemoteFailureKind::Infrastructure)
        }
        Some(tak_proto::RemoteFailureKind::Cancellation) => Some(RemoteFailureKind::Cancellation),
        Some(tak_proto::RemoteFailureKind::ResourceCapacity) => {
            Some(RemoteFailureKind::ResourceCapacity)
        }
        Some(tak_proto::RemoteFailureKind::Unknown) => Some(RemoteFailureKind::Unknown),
        _ if exit_code == Some(137) => Some(RemoteFailureKind::Unknown),
        _ => Some(RemoteFailureKind::Task),
    }
}

pub(crate) fn permits_authored_retry(failure_kind: Option<RemoteFailureKind>) -> bool {
    failure_kind != Some(RemoteFailureKind::Cancellation)
}

pub(crate) fn requires_worker_failover(failure_kind: Option<RemoteFailureKind>) -> bool {
    matches!(
        failure_kind,
        Some(RemoteFailureKind::Infrastructure | RemoteFailureKind::ResourceCapacity)
    )
}
