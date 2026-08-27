use super::*;

pub(super) fn exit_code_for_payload(
    state: &FakeDockerDaemonState,
    cmd: &[String],
    binds: &[String],
) -> i64 {
    let Some(bind_source) = binds
        .first()
        .and_then(|bind| bind.split(':').next())
        .map(Path::new)
    else {
        return 1;
    };
    let visible = state.path_is_visible(bind_source);
    let is_probe = cmd.iter().any(|value| value.contains(".tak-mount-visible"));

    if is_probe {
        let sentinel = bind_source.join(".tak-mount-visible");
        return if visible && sentinel.is_file() { 0 } else { 1 };
    }

    if cmd.iter().any(|value| value.contains("exit 137")) {
        return 137;
    }
    if cmd.iter().any(|value| value.contains("exit 1")) {
        return 1;
    }

    if visible { 0 } else { 1 }
}
