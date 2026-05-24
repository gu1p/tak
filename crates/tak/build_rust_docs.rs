use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Manifest {
    package: Package,
    #[serde(default)]
    lib: Option<Lib>,
}

#[derive(Debug, Deserialize)]
struct Package {
    name: String,
}

#[derive(Debug, Deserialize)]
struct Lib {
    #[serde(default = "default_doctest")]
    doctest: bool,
}

struct CrateSource {
    crate_name: String,
    root: PathBuf,
    doctest_enabled: bool,
}

pub(crate) fn write(repo_root: &Path, out_dir: &Path) {
    let generated = render(repo_root);
    let generated_path = out_dir.join("docs_rust_sources.rs");
    fs::write(&generated_path, generated)
        .unwrap_or_else(|err| panic!("failed to write {}: {err}", generated_path.display()));
}

fn render(repo_root: &Path) -> String {
    let mut output = String::new();
    output.push_str("pub(super) const RUST_DOC_SOURCES: &[EmbeddedRustSource] = &[\n");

    for source in collect_crates(repo_root) {
        for file in collect_rust_files(&source.root.join("src")) {
            render_rust_source(repo_root, &source, &file, &mut output);
        }
    }

    output.push_str("];\n");
    output
}

fn collect_crates(repo_root: &Path) -> Vec<CrateSource> {
    let crates_dir = repo_root.join("crates");
    let mut crates = Vec::new();
    for entry in fs::read_dir(&crates_dir)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", crates_dir.display()))
    {
        let root = entry.expect("crate directory entry").path();
        let manifest_path = root.join("Cargo.toml");
        if root.is_dir() && manifest_path.exists() {
            println!("cargo:rerun-if-changed={}", manifest_path.display());
            crates.push(read_crate_source(root, manifest_path));
        }
    }
    crates.sort_by(|left, right| left.crate_name.cmp(&right.crate_name));
    crates
}

fn read_crate_source(root: PathBuf, manifest_path: PathBuf) -> CrateSource {
    let manifest_body = fs::read_to_string(&manifest_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", manifest_path.display()));
    let manifest: Manifest = toml::from_str(&manifest_body)
        .unwrap_or_else(|err| panic!("failed to parse {}: {err}", manifest_path.display()));
    CrateSource {
        crate_name: manifest.package.name,
        root,
        doctest_enabled: manifest.lib.map(|lib| lib.doctest).unwrap_or(true),
    }
}

fn collect_rust_files(src_dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if src_dir.exists() {
        visit_rust_files(src_dir, &mut files);
    }
    files.sort();
    files
}

fn visit_rust_files(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap_or_else(|err| panic!("read {}: {err}", dir.display())) {
        let path = entry.expect("rust source entry").path();
        if path.is_dir() {
            visit_rust_files(&path, files);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            files.push(path);
        }
    }
}

fn render_rust_source(repo_root: &Path, source: &CrateSource, file: &Path, output: &mut String) {
    println!("cargo:rerun-if-changed={}", file.display());
    let body = fs::read_to_string(file)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", file.display()));
    let relative = file
        .strip_prefix(repo_root)
        .expect("source under repo root");
    let _ = writeln!(output, "    EmbeddedRustSource {{");
    let _ = writeln!(output, "        crate_name: {:?},", source.crate_name);
    let _ = writeln!(output, "        path: {:?},", normalize_path(relative));
    let _ = writeln!(output, "        body: {:?},", body);
    let _ = writeln!(
        output,
        "        doctest_enabled: {},",
        source.doctest_enabled
    );
    output.push_str("    },\n");
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn default_doctest() -> bool {
    true
}
