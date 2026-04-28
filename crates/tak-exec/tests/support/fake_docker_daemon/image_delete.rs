use std::io;

use tokio::net::UnixStream;

use super::request::FakeDockerRequest;
use super::response::write_response;
use super::state::{FakeDockerDaemonState, ImageDeleteResult};

pub(super) async fn write_image_delete_response(
    stream: &mut UnixStream,
    state: &FakeDockerDaemonState,
    request: &FakeDockerRequest,
) -> io::Result<()> {
    let Some(image) = request.deleted_image_name() else {
        write_not_found(stream).await?;
        return Ok(());
    };
    match state.delete_image(&image) {
        ImageDeleteResult::Removed => {
            let body = format!(r#"[{{"Deleted":"{image}"}}]"#);
            write_response(stream, "200 OK", "application/json", body.as_bytes()).await
        }
        ImageDeleteResult::NotFound => write_not_found(stream).await,
        ImageDeleteResult::Failed(status_code) => {
            let status = format!("{status_code} Conflict");
            write_response(
                stream,
                &status,
                "application/json",
                br#"{"message":"remove failed"}"#,
            )
            .await
        }
    }
}

async fn write_not_found(stream: &mut UnixStream) -> io::Result<()> {
    write_response(
        stream,
        "404 Not Found",
        "application/json",
        br#"{"message":"not found"}"#,
    )
    .await
}
