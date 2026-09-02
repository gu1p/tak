use std::path::Path;

use anyhow::Result;
use tak_proto::local_daemon::v2::{Operation, Request, Response};

#[derive(Clone, Copy)]
pub(super) enum Policy {
    Management,
    Foreground,
}

impl Policy {
    pub(super) async fn response(
        self,
        socket: &Path,
        request_id: &str,
        operation: Operation,
    ) -> Result<Response> {
        match self {
            Self::Management => super::super::request(socket, request_id, operation, false).await,
            Self::Foreground => {
                crate::cli::daemon_run::foreground_response(
                    socket,
                    &Request {
                        request_id: request_id.into(),
                        operation,
                    },
                )
                .await
            }
        }
    }
}
