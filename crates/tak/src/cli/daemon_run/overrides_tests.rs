use tak_core::v2::{ContainerSource, Execution, RemoteExecution, RemoteSelection, TaskRuntime};

use super::overrides;
use crate::cli::run_command::RunCliArgs;

#[test]
fn local_override_preserves_the_authored_container_runtime() {
    let runtime = TaskRuntime::container(ContainerSource::Image {
        image: "alpine:3.20".into(),
    });
    let authored = Execution::RemoteOnly {
        remote: RemoteExecution {
            reason: String::new(),
            pool: Some("build".into()),
            required_tags: vec!["linux".into()],
            required_capabilities: vec![],
            transport: Some("direct".into()),
            selection: RemoteSelection::Balanced,
            session: None,
            runtime: Some(runtime.clone()),
        },
    };
    let override_ = overrides::resolve(&args()).unwrap().unwrap();
    let resolved = override_.execution(Some(&authored)).unwrap();

    assert!(matches!(
        resolved,
        Execution::LocalOnly { local } if local.runtime == Some(runtime)
    ));
}

fn args() -> RunCliArgs {
    RunCliArgs {
        labels: vec!["//:check".into()],
        jobs: 1,
        keep_going: false,
        pass_env: vec![],
        local: true,
        local_no_container: false,
        remote: false,
        container: false,
        container_image: None,
        container_dockerfile: None,
        container_build_context: None,
    }
}
