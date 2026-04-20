use crate::support::root_task_contracts::{load_root_spec, parse};
use anyhow::Result;
use tak_core::model::StepDef;

#[test]
fn repo_root_docs_wiki_tasks_build_checkout_tak_then_run_inline_driver() -> Result<()> {
    let spec = load_root_spec()?;
    for (label, mode) in [("//:docs-wiki", "build"), ("//:docs-wiki-serve", "serve")] {
        let task = spec.tasks.get(&parse(label)).expect("docs wiki task");
        let steps = &task.steps;
        assert_eq!(
            steps.len(),
            2,
            "{label} should build tak first, then run the inline Python driver"
        );

        let build_step = match &steps[0] {
            StepDef::Cmd { argv, cwd, env } => {
                assert!(cwd.is_none(), "{label} build step should not override cwd");
                assert!(env.is_empty(), "{label} build step should not override env");
                argv
            }
            other => panic!("{label} build step should be a cmd step: {other:?}"),
        };
        assert_eq!(
            build_step,
            &["cargo", "build", "-p", "tak", "--bin", "tak"],
            "{label} should build the checkout tak binary first"
        );
        let driver_step = match &steps[1] {
            StepDef::Cmd { argv, cwd, env } => {
                assert!(cwd.is_none(), "{label} driver step should not override cwd");
                assert_eq!(
                    env.len(),
                    0,
                    "{label} should resolve TAK_BIN inside the inline driver"
                );
                argv
            }
            other => panic!("{label} driver step should be a cmd step: {other:?}"),
        };
        assert!(
            driver_step.len() >= 4,
            "{label} argv too short: {driver_step:?}"
        );
        assert_eq!(driver_step[0], "python3", "{label} should run python3");
        assert_eq!(
            driver_step[1], "-c",
            "{label} should use an inline Python snippet"
        );
        assert_eq!(
            driver_step[3], mode,
            "{label} should select the expected mode"
        );
        assert!(
            driver_step[2].contains("os.environ.get(\"CARGO_TARGET_DIR\", \"target\")"),
            "{label} should resolve the checkout tak binary from the active target dir"
        );
    }
    Ok(())
}

#[test]
fn repo_root_docs_wiki_serve_embeds_quiet_disconnect_tolerant_server() -> Result<()> {
    let spec = load_root_spec()?;
    let task = spec
        .tasks
        .get(&parse("//:docs-wiki-serve"))
        .expect("docs-wiki-serve task");
    let snippet = match &task.steps[1] {
        StepDef::Cmd { argv, .. } => &argv[2],
        other => panic!("docs-wiki-serve driver step should be cmd: {other:?}"),
    };
    for needle in [
        "ThreadingHTTPServer",
        "QuietSimpleHTTPRequestHandler",
        "BrokenPipeError",
        "ConnectionResetError",
        "ConnectionAbortedError",
        "serve_forever",
    ] {
        assert!(
            snippet.contains(needle),
            "docs wiki serve snippet missing `{needle}`:\n{snippet}"
        );
    }
    assert!(
        snippet.contains("os.environ.get(\"TAK_BIN\""),
        "docs wiki serve should allow an explicit TAK_BIN override:\n{snippet}"
    );
    assert!(
        snippet.contains("os.environ.get(\"CARGO_TARGET_DIR\", \"target\")"),
        "docs wiki serve should fall back to the active cargo target dir:\n{snippet}"
    );
    Ok(())
}
