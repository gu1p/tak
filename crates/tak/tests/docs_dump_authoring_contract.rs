use crate::support;

use anyhow::Result;
use std::collections::BTreeMap;

use support::run_tak_expect_success;

fn docs_dump() -> Result<String> {
    run_tak_expect_success(
        tempfile::tempdir()?.path(),
        &["docs", "dump"],
        &BTreeMap::new(),
    )
}

#[test]
fn docs_dump_leads_with_authoring_decisions_before_reference_material() -> Result<()> {
    let output = docs_dump()?;
    let workflow = output
        .find("## Authoring Workflow")
        .expect("workflow section");
    let cli = output.find("## CLI Surface").expect("CLI section");
    let api = output
        .find("## TASKS.py API Surface")
        .expect("TASKS.py API section");

    assert!(
        workflow < cli,
        "authoring guidance should precede CLI reference"
    );
    assert!(
        workflow < api,
        "authoring guidance should precede DSL reference"
    );
    Ok(())
}

#[test]
fn docs_dump_teaches_an_llm_to_annotate_an_existing_makefile() -> Result<()> {
    let output = docs_dump()?;
    for token in [
        "### Annotate an existing Makefile",
        "Do not create a `TASKS.py` just to wrap an existing Make goal.",
        "# tak: default.execution=remote",
        "# tak: default.container-image=ghcr.io/acme/build:latest",
        "# tak: execution=remote",
        "# tak: container-dockerfile=docker/test.Dockerfile",
        "# tak: container-build-context=.",
        "# tak: parallel=lint,test",
        "Supported goal annotation keys",
        "A blank line or ordinary comment breaks that association.",
        "`container-image` and `container-dockerfile` are mutually exclusive",
        "`container-build-context` requires `container-dockerfile`",
        "`tak make check`",
    ] {
        assert!(
            output.contains(token),
            "missing Makefile guidance `{token}`"
        );
    }
    Ok(())
}

#[test]
fn docs_dump_teaches_an_llm_to_create_and_validate_tasks_py() -> Result<()> {
    let output = docs_dump()?;
    for token in [
        "### Create a TASKS.py workspace",
        "build = task(",
        "check = task(",
        "outputs=[path(\"out/build.txt\")]",
        "SPEC = module_spec(",
        "Inspect the graph without executing it:",
        "Then execute the task:",
        "tak list",
        "tak explain //:check",
        "tak graph //:check --format dot",
        "tak run //:check",
    ] {
        assert!(
            output.contains(token),
            "missing TASKS.py guidance `{token}`"
        );
    }
    Ok(())
}
