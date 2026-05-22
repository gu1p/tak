use tak_core::model::{
    CurrentStateSpec, RemoteSelectionSpec, ResolvedTask, RetryDef, SessionReuseSpec,
    SessionUseSpec, TaskExecutionSpec, TaskLabel,
};

use crate::engine::PlacementMode;
use crate::engine::remote_models::{StrictRemoteTarget, StrictRemoteTransportKind, TaskPlacement};
use crate::engine::remote_selection::{RemoteSelectionState, ordered_remote_targets_for_attempt};

pub(super) fn run_id_that_prefers(targets: &[StrictRemoteTarget], node_id: &str) -> String {
    for index in 0..10_000 {
        let run_id = format!("run-{index}");
        let ordered = ordered_remote_targets_for_attempt(
            targets,
            RemoteSelectionSpec::Shuffle,
            "//:root-b",
            &run_id,
            1,
            &RemoteSelectionState::default(),
        );
        if ordered[0].node_id == node_id {
            return run_id;
        }
    }
    panic!("could not find deterministic run id for {node_id}");
}

pub(super) fn aggregate_task() -> ResolvedTask {
    ResolvedTask {
        label: TaskLabel {
            package: "//".into(),
            name: "root".into(),
        },
        doc: String::new(),
        deps: Vec::new(),
        steps: Vec::new(),
        needs: Vec::new(),
        queue: None,
        retry: RetryDef::default(),
        timeout_s: None,
        context: CurrentStateSpec::default(),
        outputs: Vec::new(),
        container_runtime: None,
        execution: TaskExecutionSpec::default(),
        session: None,
        cascade_execution: true,
        tags: Vec::new(),
    }
}

pub(super) fn placement_with_session(
    node_id: &str,
    session: Option<SessionUseSpec>,
) -> TaskPlacement {
    TaskPlacement {
        placement_mode: PlacementMode::Remote,
        remote_node_id: Some(node_id.to_string()),
        strict_remote_target: None,
        ordered_remote_targets: targets(),
        remote_selection: RemoteSelectionSpec::Shuffle,
        decision_reason: None,
        local: None,
        remote: None,
        session,
    }
}

pub(super) fn container_session() -> SessionUseSpec {
    SessionUseSpec {
        name: "container".into(),
        display_name: "container".into(),
        execution: None,
        reuse: SessionReuseSpec::Container,
        context: None,
    }
}

pub(super) fn targets() -> Vec<StrictRemoteTarget> {
    ["builder-a", "builder-b"]
        .into_iter()
        .map(|node_id| StrictRemoteTarget {
            node_id: node_id.into(),
            endpoint: "http://127.0.0.1:1".into(),
            transport_kind: StrictRemoteTransportKind::Direct,
            bearer_token: "secret".into(),
            runtime: None,
        })
        .collect()
}
