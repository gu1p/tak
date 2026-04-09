use super::*;

/// Renders a DOT graph for the selected task scope.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub(super) fn print_dot_graph(spec: &WorkspaceSpec, scope: &[TaskLabel]) {
    println!("digraph tak {{");
    for label in scope {
        if let Some(task) = spec.tasks.get(label) {
            if task.deps.is_empty() {
                println!("  \"{}\";", canonical_label(label));
            } else {
                for dep in &task.deps {
                    println!(
                        "  \"{}\" -> \"{}\";",
                        canonical_label(dep),
                        canonical_label(label)
                    );
                }
            }
        }
    }
    println!("}}");
}
