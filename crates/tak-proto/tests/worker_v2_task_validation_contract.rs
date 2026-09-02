use tak_core::v2::{EnvironmentValue, OutputSelector, Step};
use tak_proto::worker_v2::{DispatchAttemptRequest, encode_dispatch_request, payload_digest};

use crate::worker_v2_attempt_support::{payload, request};

#[test]
fn worker_dispatch_rejects_step_and_output_paths_outside_the_workspace() {
    let mut cwd = request(payload());
    cwd.payload.tasks[0].steps = vec![Step::Cmd {
        argv: vec!["true".into()],
        cwd: Some("/tmp".into()),
        env: Default::default(),
    }];
    reject(cwd);

    let mut script = request(payload());
    script.payload.tasks[0].steps = vec![Step::Script {
        path: "../../escape.sh".into(),
        argv: vec![],
        interpreter: None,
        cwd: None,
        env: Default::default(),
    }];
    reject(script);

    for output in [
        OutputSelector::Path {
            value: "../result".into(),
        },
        OutputSelector::Glob {
            value: "../../*".into(),
        },
    ] {
        let mut invalid = request(payload());
        invalid.payload.tasks[0].outputs = vec![output];
        reject(invalid);
    }
}

#[test]
fn worker_dispatch_rejects_malformed_task_environment_and_context_projection() {
    let mut duplicate = request(payload());
    duplicate
        .payload
        .tasks
        .push(duplicate.payload.tasks[0].clone());
    reject(duplicate);

    let mut invalid_id = request(payload());
    invalid_id.payload.tasks[0].task_id.clear();
    reject(invalid_id);

    let mut environment = request(payload());
    environment.payload.environment_values = vec![
        EnvironmentValue::new("A", "one").unwrap(),
        EnvironmentValue::new("B", "two").unwrap(),
    ];
    environment.payload.tasks[0].pass_env_names = vec!["B".into(), "A".into()];
    reject(environment);

    let mut context = request(payload());
    context.payload.context_manifest.paths = vec!["missing".into()];
    reject(context);
}

fn reject(mut request: DispatchAttemptRequest) {
    request.payload_digest = payload_digest(&request.payload).unwrap();
    assert!(encode_dispatch_request(&request).is_err());
}
