use anyhow::Result;
use std::fs;
use std::path::PathBuf;

const OLD_PARSER_TOKENS: [&str; 7] = [
    "strip_prefix(\"class \")",
    "strip_prefix(\"def \")",
    "split_once(':')",
    "paren_depth",
    "parse_function_signature",
    "parse_python_docstring",
    "include!(\"dsl_parse.rs\")",
];

#[test]
fn docs_dump_parses_python_dsl_sources_with_ruff_ast() -> Result<()> {
    let source_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/docs");
    let dsl_source = fs::read_to_string(source_dir.join("dsl.rs"))?;
    let mut parser_sources = dsl_source;
    for file_name in ["dsl_ast.rs", "dsl_parse.rs", "dsl_docstrings.rs"] {
        let path = source_dir.join(file_name);
        if path.exists() {
            parser_sources.push_str(&fs::read_to_string(path)?);
        }
    }

    assert!(
        parser_sources.contains("ruff_python_parser::parse_module"),
        "docs DSL extraction should use Ruff's Python parser:\n{parser_sources}"
    );

    for token in OLD_PARSER_TOKENS {
        assert!(
            !parser_sources.contains(token),
            "docs DSL extraction still contains hand parser token `{token}`"
        );
    }

    Ok(())
}
