mod markdown;

use std::collections::BTreeMap;
use std::fmt::Write;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use self::markdown::{extract_bullets, extract_first_fenced_block, extract_intro_paragraph};

const ROOT_README: &str = include_str!("../../../README.md");
const EXAMPLES_README: &str = include_str!("../../../examples/README.md");
const DSL_STUBS: &str = include_str!("../../tak-loader/src/loader/dsl_stubs.pyi");
const CATALOG_TOML: &str = include_str!("../../../examples/catalog.toml");

#[derive(Debug, Deserialize)]
struct Catalog {
    example: Vec<ExampleEntry>,
}

#[derive(Debug, Deserialize)]
struct ExampleEntry {
    name: String,
    run_target: String,
    #[serde(default)]
    capabilities: Vec<String>,
    #[serde(default)]
    use_when: String,
    #[serde(default)]
    project_shapes: Vec<String>,
    #[serde(default)]
    avoid_when: Vec<String>,
}

pub(crate) fn render_docs_dump() -> Result<String> {
    let catalog: Catalog =
        toml::from_str(CATALOG_TOML).context("failed to parse embedded examples catalog")?;
    let intro = extract_intro_paragraph(ROOT_README)
        .context("failed to extract Tak overview from embedded README")?;
    let capabilities = extract_bullets(ROOT_README, "## Core Capabilities")
        .context("failed to extract core capabilities from embedded README")?;
    let starter = extract_first_fenced_block(ROOT_README, "## Copy-Paste TASKS.py Starter")
        .context("failed to extract starter TASKS.py block from embedded README")?;
    let workflow = extract_first_fenced_block(EXAMPLES_README, "## Standard Command Workflow")
        .context(
            "failed to extract standard command workflow block from embedded examples README",
        )?;

    let documented_examples = catalog
        .example
        .iter()
        .filter(|entry| {
            !entry.capabilities.is_empty()
                && !entry.use_when.trim().is_empty()
                && !entry.project_shapes.is_empty()
        })
        .collect::<Vec<_>>();
    if documented_examples.is_empty() {
        bail!("embedded examples catalog does not expose any authoring metadata");
    }

    let mut project_shapes = BTreeMap::<String, Vec<&str>>::new();
    for entry in &documented_examples {
        for shape in &entry.project_shapes {
            project_shapes
                .entry(shape.clone())
                .or_default()
                .push(entry.name.as_str());
        }
    }

    let mut output = String::new();
    output.push_str("# Tak Agent Docs\n\n");

    output.push_str("## What Tak Is For\n\n");
    output.push_str(intro.trim());
    output.push_str("\n\n");
    output.push_str(
        "Use this bundle when an agent needs to draft or review `TASKS.py` for another project. \
Start from the nearest example, then adapt the smallest pattern that matches the project shape.\n\n",
    );

    output.push_str("## Core Capabilities\n\n");
    for bullet in &capabilities {
        let _ = writeln!(output, "- {bullet}");
    }
    output.push('\n');

    output.push_str("## TASKS.py API Surface\n\n");
    output.push_str("The shipped Python DSL surface is:\n\n```pyi\n");
    output.push_str(DSL_STUBS.trim());
    output.push_str("\n```\n\n");

    output.push_str("## Project Patterns\n\n");
    for (shape, examples) in &project_shapes {
        let _ = writeln!(
            output,
            "- `{shape}`: start from {}",
            examples
                .iter()
                .map(|name| format!("`{name}`"))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    output.push('\n');

    output.push_str("## Example Chooser\n\n");
    for entry in documented_examples {
        let _ = writeln!(output, "### `{}`\n", entry.name);
        let _ = writeln!(output, "- Use when: {}", entry.use_when.trim());
        let _ = writeln!(
            output,
            "- Project shapes: {}",
            entry.project_shapes.join(", ")
        );
        let _ = writeln!(output, "- Capabilities: {}", entry.capabilities.join(", "));
        let _ = writeln!(output, "- Run target: `{}`", entry.run_target);
        if !entry.avoid_when.is_empty() {
            let _ = writeln!(output, "- Avoid when: {}", entry.avoid_when.join(", "));
        }
        output.push('\n');
    }

    output.push_str("## Authoring Workflow\n\n");
    output.push_str("1. Pick the closest example from the chooser.\n");
    output.push_str("2. Start from a small `module_spec(...)` and add only the execution, retry, context, and coordination features the project actually needs.\n");
    output.push_str("3. Validate the graph and labels before running anything expensive.\n");
    output.push_str("4. Keep task docs, outputs, and dependencies explicit.\n\n");

    output.push_str("Starter shape from the Tak README:\n\n```python\n");
    output.push_str(starter.trim());
    output.push_str("\n```\n\n");

    output.push_str("Standard inspection workflow:\n\n```bash\n");
    output.push_str(workflow.trim());
    output.push_str("\n```\n\n");

    output.push_str("Smallest explicit root-task flow:\n\n```bash\n");
    output.push_str("tak list\n");
    output.push_str("tak explain //:hello\n");
    output.push_str("tak graph //:hello --format dot\n");
    output.push_str("tak run //:hello\n");
    output.push_str("```\n");

    Ok(output)
}
