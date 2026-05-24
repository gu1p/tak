use std::fmt::Write;

use super::rustdoc::VerifiedRustExample;

pub(super) fn render_verified_rust_examples(output: &mut String, examples: &[VerifiedRustExample]) {
    if examples.is_empty() {
        return;
    }

    output.push_str("## Verified Rust Examples\n\n");
    for example in examples {
        let path = example.path.strip_prefix("crates/").unwrap_or(example.path);
        let _ = writeln!(output, "#### {}\n", example.title);
        let _ = writeln!(
            output,
            "From `{}`. Crate: `{}`.\n",
            path, example.crate_name
        );
        let _ = writeln!(output, "```{}", example.language);
        output.push_str(example.code.trim());
        output.push_str("\n```\n\n");
    }
}
