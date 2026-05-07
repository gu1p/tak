use super::*;

use bollard::models::BuildInfo;

pub(super) fn emit_docker_build_info(
    build_info: &BuildInfo,
    run_context: &ContainerStepRunContext<'_>,
) -> Result<()> {
    if let Some(stream) = build_info
        .stream
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        crate::emit_task_output(
            run_context.output_observer,
            run_context.task_run_id,
            run_context.task_label,
            run_context.attempt,
            OutputStream::Stdout,
            stream.as_bytes(),
        )?;
    }
    if let Some(status) = docker_build_status_message(build_info) {
        emit_build_line(&status, OutputStream::Stdout, run_context)?;
    }
    if let Some(error) = docker_build_error_message(build_info) {
        emit_build_line(&error, OutputStream::Stderr, run_context)?;
    }
    Ok(())
}

pub(super) fn emit_build_line(
    message: &str,
    stream: OutputStream,
    run_context: &ContainerStepRunContext<'_>,
) -> Result<()> {
    if message.is_empty() {
        return Ok(());
    }
    let mut line = message.as_bytes().to_vec();
    if !line.ends_with(b"\n") {
        line.push(b'\n');
    }
    crate::emit_task_output(
        run_context.output_observer,
        run_context.task_run_id,
        run_context.task_label,
        run_context.attempt,
        stream,
        &line,
    )
}

fn docker_build_status_message(build_info: &BuildInfo) -> Option<String> {
    let status = build_info.status.as_deref()?.trim();
    if status.is_empty() {
        return None;
    }
    let mut message = String::new();
    if let Some(id) = build_info
        .id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
    {
        message.push_str(id);
        message.push_str(": ");
    }
    message.push_str(status);
    if let Some(progress) = build_info
        .progress
        .as_deref()
        .map(str::trim)
        .filter(|progress| !progress.is_empty())
    {
        message.push(' ');
        message.push_str(progress);
    }
    Some(message)
}

pub(super) fn docker_build_error_message(build_info: &BuildInfo) -> Option<String> {
    build_info
        .error_detail
        .as_ref()
        .and_then(|detail| detail.message.as_deref())
        .or(build_info.error.as_deref())
        .map(str::trim)
        .filter(|message| !message.is_empty())
        .map(str::to_string)
}
