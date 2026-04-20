use std::fmt::Write;

use super::dsl::DslDocs;

pub(super) fn render_dsl_docs(output: &mut String, docs: &DslDocs) {
    output.push_str("## TASKS.py API Surface\n\n");

    output.push_str("### Types\n\n");
    for entry in &docs.types {
        let _ = writeln!(output, "#### `{}`\n", entry.name);
        let _ = writeln!(output, "`{}`\n", entry.signature.trim());
        if !entry.summary.trim().is_empty() {
            output.push_str(entry.summary.trim());
            output.push_str("\n\n");
        }
        for field in &entry.fields {
            let _ = writeln!(output, "- `{}`: `{}`", field.name, field.ty);
        }
        if !entry.fields.is_empty() {
            output.push('\n');
        }
    }

    output.push_str("### Constants\n\n");
    for entry in &docs.constants {
        let _ = writeln!(output, "#### `{}`\n", entry.name);
        let _ = writeln!(output, "`{}`\n", entry.signature.trim());
        if !entry.summary.trim().is_empty() {
            output.push_str(entry.summary.trim());
            output.push_str("\n\n");
        }
    }

    output.push_str("### Functions\n\n");
    for entry in &docs.functions {
        let _ = writeln!(output, "#### `{}`\n", entry.name);
        let _ = writeln!(output, "`{}`\n", entry.signature.trim());
        if !entry.summary.trim().is_empty() {
            output.push_str(entry.summary.trim());
            output.push_str("\n\n");
        }
    }

    output.push_str("### Methods\n\n");
    if docs.methods.is_empty() {
        output.push_str("No public methods are currently exposed by the shipped TASKS.py DSL.\n\n");
        return;
    }

    for entry in &docs.methods {
        let _ = writeln!(output, "#### `{}.{}`\n", entry.owner, entry.name);
        let _ = writeln!(output, "`{}`\n", entry.signature.trim());
        if !entry.summary.trim().is_empty() {
            output.push_str(entry.summary.trim());
            output.push_str("\n\n");
        }
    }
}
