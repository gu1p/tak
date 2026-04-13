use std::env;
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

fn main() {
    let version = env::var("TAK_BUILD_VERSION")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let repo_root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .expect("repo root should be two levels above tak crate");
    let generated = render_docs_dump_examples(repo_root);
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let generated_path = out_dir.join("docs_dump_examples.rs");

    fs::write(&generated_path, generated)
        .unwrap_or_else(|err| panic!("failed to write {}: {err}", generated_path.display()));

    println!("cargo:rustc-env=TAK_VERSION={version}");
    println!("cargo:rerun-if-env-changed=TAK_BUILD_VERSION");
    println!("cargo:rerun-if-changed=../../README.md");
    println!("cargo:rerun-if-changed=../../examples/README.md");
    println!("cargo:rerun-if-changed=../../examples/catalog.toml");
    println!("cargo:rerun-if-changed=../tak-loader/src/loader/dsl_stubs.pyi");
}

fn render_docs_dump_examples(repo_root: &Path) -> String {
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
        let example_dir = repo_root.join("examples").join(&entry.name);
        println!("cargo:rerun-if-changed={}", example_dir.display());

        let source_files = collect_example_tasks_files(&example_dir);
        if source_files.is_empty() {
            panic!(
                "documented example `{}` does not include any TASKS.py files under {}",
                entry.name,
                example_dir.display()
            );
        }

        let _ = writeln!(output, "    EmbeddedExampleSources {{");
        let _ = writeln!(output, "        name: {:?},", entry.name);
        output.push_str("        source_files: &[\n");
        for source_file in source_files {
            println!("cargo:rerun-if-changed={}", source_file.display());

            let relative = source_file
                .strip_prefix(&example_dir)
                .expect("source file should live under example dir");
            let relative = normalize_path(relative);
            let body = fs::read_to_string(&source_file)
                .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_file.display()));

            let _ = writeln!(output, "            EmbeddedSourceFile {{");
            let _ = writeln!(output, "                path: {:?},", relative);
            let _ = writeln!(output, "                body: {:?},", body);
            output.push_str("            },\n");
        }
        output.push_str("        ],\n");
        output.push_str("    },\n");
    }

    output.push_str("];\n");
    output
}

fn collect_example_tasks_files(example_dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    visit_tasks_files(example_dir, &mut files);
    files.sort_by_key(|path| {
        let relative = normalize_path(
            path.strip_prefix(example_dir)
                .expect("TASKS.py path should live under example dir"),
        );
        let rank = usize::from(relative != "TASKS.py");
        (rank, relative)
    });
    files
}

fn visit_tasks_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir)
        .unwrap_or_else(|err| panic!("failed to read example directory {}: {err}", dir.display()));

    for entry in entries {
        let entry =
            entry.unwrap_or_else(|err| panic!("failed to read entry in {}: {err}", dir.display()));
        let path = entry.path();

        if path.is_dir() {
            visit_tasks_files(&path, files);
            continue;
        }

        if path.file_name().and_then(|name| name.to_str()) == Some("TASKS.py") {
            files.push(path);
        }
    }
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
