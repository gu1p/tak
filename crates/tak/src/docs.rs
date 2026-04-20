mod cli;
mod dsl;
mod dsl_render;
mod examples;
mod examples_parse;
mod model;
mod rustdoc;
mod text;

use std::collections::BTreeMap;
use std::fmt::Write;

use anyhow::{Context, Result, bail};

use self::cli::collect_cli_docs;
use self::dsl::collect_dsl_docs;
use self::dsl_render::render_dsl_docs;
use self::examples::extract_example_source_docs;
use self::model::{documented_example_sources, documented_examples, load_catalog};
use self::rustdoc::extract_rust_module_docs;
use self::text::with_trailing_newline;

const TAK_LIB: &str = include_str!("lib.rs");
const TAK_CORE_LIB: &str = include_str!("../../tak-core/src/lib.rs");
const TAK_EXEC_LIB: &str = include_str!("../../tak-exec/src/lib.rs");
const TAK_LOADER_LIB: &str = include_str!("../../tak-loader/src/lib.rs");
const TAKD_LIB: &str = include_str!("../../takd/src/lib.rs");

pub(crate) fn render_docs_dump() -> Result<String> {
    let catalog = load_catalog()?;
    let documented_examples = documented_examples(&catalog);
    if documented_examples.is_empty() {
        bail!("embedded examples catalog does not expose any authoring metadata");
    }

    let overview = extract_rust_module_docs(TAK_LIB)
        .context("failed to extract Tak overview from source docs")?;
    let crate_docs = [
        (
            "tak-loader",
            extract_rust_module_docs(TAK_LOADER_LIB)
                .context("failed to extract tak-loader overview from source docs")?,
        ),
        (
            "tak-core",
            extract_rust_module_docs(TAK_CORE_LIB)
                .context("failed to extract tak-core overview from source docs")?,
        ),
        (
            "tak-exec",
            extract_rust_module_docs(TAK_EXEC_LIB)
                .context("failed to extract tak-exec overview from source docs")?,
        ),
        (
            "takd",
            extract_rust_module_docs(TAKD_LIB)
                .context("failed to extract takd overview from source docs")?,
        ),
        ("tak", overview.clone()),
    ];
    let cli_docs = collect_cli_docs();
    let dsl_docs = collect_dsl_docs()?;
    let example_sources = documented_example_sources();
    let project_shapes = collect_project_shapes(&documented_examples);

    let mut output = String::new();
    output.push_str("# Tak Agent Docs\n\n");
    output.push_str("## What Tak Is For\n\n");
    output.push_str(overview.trim());
    output.push_str("\n\n");

    output.push_str("## Core Capabilities\n\n");
    for (crate_name, docs) in crate_docs {
        let _ = writeln!(output, "- `{crate_name}`: {}", docs.trim());
    }
    output.push('\n');

    output.push_str("## CLI Surface\n\n");
    for entry in cli_docs {
        let _ = writeln!(output, "### `{}`\n", entry.path);
        output.push_str(entry.summary.trim());
        output.push_str("\n\n");
        for arg in entry.args {
            let _ = writeln!(output, "- {}: {}", arg.syntax, arg.summary);
        }
        if !output.ends_with("\n\n") {
            output.push('\n');
        }
    }

    render_dsl_docs(&mut output, &dsl_docs);

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
        let sources = example_sources
            .get(entry.name.as_str())
            .with_context(|| format!("missing embedded source files for `{}`", entry.name))?;
        let source_docs = extract_example_source_docs(sources);
        render_example_entry(&mut output, entry, &source_docs, sources);
    }

    output.push_str("## Authoring Workflow\n\n");
    output.push_str("1. Start from the closest example and keep intent next to the source with `task(doc=...)`, crate docs, and command doc comments.\n");
    output.push_str("2. Use `tak list`, `tak explain`, and `tak graph` to validate labels and graph shape before you run expensive work.\n");
    output.push_str("3. Add only the execution, retry, coordination, and remote constructs the project actually needs.\n");

    Ok(with_trailing_newline(output))
}

fn collect_project_shapes<'a>(
    entries: &[&'a model::ExampleEntry],
) -> BTreeMap<String, Vec<&'a str>> {
    let mut project_shapes = BTreeMap::<String, Vec<&str>>::new();
    for entry in entries {
        for shape in &entry.project_shapes {
            project_shapes
                .entry(shape.clone())
                .or_default()
                .push(entry.name.as_str());
        }
    }
    project_shapes
}

fn render_example_entry(
    output: &mut String,
    entry: &model::ExampleEntry,
    source_docs: &examples::ExampleSourceDoc,
    sources: &model::EmbeddedExampleSources,
) {
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
    if let Some(scenario) = source_docs.scenario.as_deref() {
        let _ = writeln!(output, "- Scenario: {scenario}");
    }
    if !source_docs.task_docs.is_empty() {
        output.push_str("- Task docs:\n");
        for task_doc in &source_docs.task_docs {
            let _ = writeln!(output, "  - `{}`: {}", task_doc.name, task_doc.doc);
        }
    }
    output.push('\n');

    output.push_str("#### Source Files\n\n");
    for source in sources.source_files {
        let _ = writeln!(output, "##### `{}`\n", source.path);
        output.push_str("```python\n");
        output.push_str(source.body.trim());
        output.push_str("\n```\n\n");
    }
}
