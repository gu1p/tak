use super::text::normalize_doc_text;

pub(super) fn extract_rust_module_docs(source: &str) -> Option<String> {
    let mut raw_lines = Vec::new();
    let mut saw_docs = false;

    for line in source.lines() {
        let trimmed = line.trim_start();
        if let Some(doc) = trimmed.strip_prefix("//!") {
            saw_docs = true;
            raw_lines.push(doc.trim_start().to_string());
            continue;
        }

        if !saw_docs && trimmed.is_empty() {
            continue;
        }

        if saw_docs && trimmed.is_empty() {
            raw_lines.push(String::new());
            continue;
        }

        break;
    }

    if raw_lines.is_empty() {
        None
    } else {
        Some(normalize_doc_text(&raw_lines.join("\n")))
    }
}
