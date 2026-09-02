use tak_core::v2::{AuthoredModule, AuthoredTask, PassEnv};

use super::graph::{canonical, selected_tasks};

#[test]
fn package_labels_and_dependencies_remain_absolute() {
    let module = AuthoredModule {
        tasks: vec![
            task("//:root", &[]),
            task("//apps/web:build", &["//:root"]),
            task("//apps/web:test", &["//apps/web:build"]),
        ],
        ..AuthoredModule::default()
    };

    assert_eq!(canonical("//apps/web:test").unwrap(), "//apps/web:test");
    let selected = selected_tasks(&module, &["//apps/web:test".into()]).unwrap();
    assert_eq!(
        selected
            .iter()
            .map(|task| task.name.as_str())
            .collect::<Vec<_>>(),
        ["//:root", "//apps/web:build", "//apps/web:test"]
    );
}

#[test]
fn path_shorthand_reports_run_label_guidance_and_available_targets() {
    let module = AuthoredModule {
        tasks: vec![task("//:build", &[]), task("//:test", &["//:build"])],
        ..AuthoredModule::default()
    };

    let error = selected_tasks(&module, &[".".into()])
        .unwrap_err()
        .to_string();

    assert!(error.contains("`.` is not a valid task label"), "{error}");
    assert!(error.contains("tak list"), "{error}");
    assert!(error.contains("//:build"), "{error}");
    assert!(error.contains("//:test"), "{error}");
}

fn task(name: &str, deps: &[&str]) -> AuthoredTask {
    AuthoredTask {
        name: name.into(),
        doc: String::new(),
        deps: deps.iter().map(ToString::to_string).collect(),
        steps: Vec::new(),
        outputs: Vec::new(),
        context: None,
        execution: None,
        retry: None,
        queue: None,
        limiter_claims: Vec::new(),
        session: None,
        cascade_session: false,
        idempotent: false,
        pass_env: PassEnv::default(),
        affinity: None,
        tags: Vec::new(),
        timeout_s: None,
    }
}
