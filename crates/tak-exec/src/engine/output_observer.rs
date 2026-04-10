fn emit_task_output(
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
    task_label: &TaskLabel,
    attempt: u32,
    stream: OutputStream,
    bytes: &[u8],
) -> Result<()> {
    if bytes.is_empty() {
        return Ok(());
    }

    let Some(observer) = output_observer else {
        return Ok(());
    };
    observer.observe_output(TaskOutputChunk {
        task_label: task_label.clone(),
        attempt,
        stream,
        bytes: bytes.to_vec(),
    })
}
