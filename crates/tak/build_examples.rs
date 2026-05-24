use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Catalog {
    example: Vec<ExampleEntry>,
}

#[derive(Debug, Deserialize)]
struct ExampleEntry {
    name: String,
    #[serde(default)]
    capabilities: Vec<String>,
    #[serde(default)]
    use_when: String,
    #[serde(default)]
    project_shapes: Vec<String>,
}

pub(crate) fn write(repo_root: &Path, out_dir: &Path) {
    let generated = render(repo_root);
    let generated_path = out_dir.join("docs_dump_examples.rs");
    fs::write(&generated_path, generated)
        .unwrap_or_else(|err| panic!("failed to write {}: {err}", generated_path.display()));
}

fn render(repo_root: &Path) -> String {
    let catalog_path = repo_root.join("examples/catalog.toml");
    let catalog_body = fs::read_to_string(&catalog_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", catalog_path.display()));
    let catalog: Catalog = toml::from_str(&catalog_body)
        .unwrap_or_else(|err| panic!("failed to parse {}: {err}", catalog_path.display()));

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
        panic!("embedded examples catalog does not expose any authoring metadata");
    }

    let mut output = String::new();
    output.push_str("const DOCUMENTED_EXAMPLE_SOURCES: &[EmbeddedExampleSources] = &[\n");
    for entry in documented_examples {
        render_example(repo_root, entry, &mut output);
    }
    output.push_str("];\n");
    output
}

fn render_example(repo_root: &Path, entry: &ExampleEntry, output: &mut String) {
    let example_dir = repo_root.join("examples").join(&entry.name);
    println!("cargo:rerun-if-changed={}", example_dir.display());

    let source_files = collect_example_source_files(&example_dir);
    if source_files.is_empty() {
        panic!(
            "documented example `{}` does not include source files",
            entry.name
        );
    }

    let _ = writeln!(output, "    EmbeddedExampleSources {{");
    let _ = writeln!(output, "        name: {:?},", entry.name);
    output.push_str("        source_files: &[\n");
    for source_file in source_files {
        render_source_file(&example_dir, &source_file, output);
    }
    output.push_str("        ],\n");
    output.push_str("    },\n");
}

fn render_source_file(example_dir: &Path, source_file: &Path, output: &mut String) {
    println!("cargo:rerun-if-changed={}", source_file.display());
    let relative = source_file
        .strip_prefix(example_dir)
        .expect("source file should live under example dir");
    let body = fs::read_to_string(source_file)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_file.display()));

    let _ = writeln!(output, "            EmbeddedSourceFile {{");
    let _ = writeln!(
        output,
        "                path: {:?},",
        normalize_path(relative)
    );
    let _ = writeln!(output, "                body: {:?},", body);
    output.push_str("            },\n");
}

fn collect_example_source_files(example_dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    visit_example_source_files(example_dir, &mut files);
    files.sort_by_key(|path| {
        let relative = normalize_path(path.strip_prefix(example_dir).expect("example source path"));
        (usize::from(relative != "TASKS.py"), relative)
    });
    files
}

fn visit_example_source_files(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap_or_else(|err| panic!("read {}: {err}", dir.display())) {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            visit_example_source_files(&path, files);
        } else if is_example_source_file(&path) {
            files.push(path);
        }
    }
}

fn is_example_source_file(path: &Path) -> bool {
    path.file_name().and_then(|name| name.to_str()) == Some("TASKS.py")
        || path
            .components()
            .any(|component| component.as_os_str().to_str() == Some("scripts"))
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
