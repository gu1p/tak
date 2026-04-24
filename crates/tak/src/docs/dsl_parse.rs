fn parse_typed_dict_class(
    lines: &[&str],
    start: usize,
    summary: String,
) -> Result<(DslTypeEntry, Vec<DslMethodEntry>, usize)> {
    let header = lines[start].trim().to_string();
    let name = parse_class_name(&header)?;
    let mut fields = Vec::new();
    let mut methods = Vec::new();
    let mut pending_comments = Vec::new();
    let mut index = start + 1;

    while index < lines.len() {
        let raw_line = lines[index];
        let trimmed = raw_line.trim();

        if trimmed.is_empty() {
            pending_comments.clear();
            index += 1;
            continue;
        }
        if is_top_level(raw_line) {
            break;
        }
        if let Some(comment) = parse_stub_comment(trimmed) {
            pending_comments.push(comment.to_string());
            index += 1;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("def ") {
            let Some(name_end) = rest.find('(') else {
                bail!("failed to parse Python method name from `{trimmed}`");
            };
            let (signature, next_index) = parse_function_signature(lines, index);
            methods.push(DslMethodEntry {
                owner: name.clone(),
                name: rest[..name_end].trim().to_string(),
                signature,
                summary: consume_pending_comments(&mut pending_comments),
            });
            index = next_index;
            continue;
        }
        if let Some((field_name, field_ty)) = parse_annotated_name_and_type(trimmed) {
            let summary = consume_pending_comments(&mut pending_comments);
            fields.push(DslFieldEntry {
                name: field_name,
                ty: field_ty,
                summary,
            });
        }
        index += 1;
    }

    Ok((
        DslTypeEntry {
            name,
            signature: header,
            summary,
            fields,
        },
        methods,
        index,
    ))
}

fn parse_class_name(header: &str) -> Result<String> {
    let Some(rest) = header.strip_prefix("class ") else {
        bail!("expected class header, got `{header}`");
    };
    let Some(name_end) = rest.find(['(', ':']) else {
        bail!("failed to parse class name from `{header}`");
    };
    Ok(rest[..name_end].trim().to_string())
}

fn parse_annotated_name_and_type(line: &str) -> Option<(String, String)> {
    let (name, ty) = line.split_once(':')?;
    let name = name.trim();
    let ty = ty.trim();
    if name.is_empty() || ty.is_empty() {
        return None;
    }
    Some((name.to_string(), ty.to_string()))
}

fn parse_stub_comment(line: &str) -> Option<&str> {
    line.strip_prefix('#').map(str::trim_start)
}

fn parse_function_signature(lines: &[&str], start: usize) -> (String, usize) {
    let mut parts = Vec::new();
    let mut depth = 0_i32;
    let mut index = start;

    while index < lines.len() {
        let trimmed = lines[index].trim();
        parts.push(trimmed.to_string());
        depth += trimmed.matches('(').count() as i32;
        depth -= trimmed.matches(')').count() as i32;
        index += 1;
        if depth <= 0 && trimmed.ends_with(": ...") {
            break;
        }
    }

    (parts.join(" "), index)
}

fn consume_pending_comments(comments: &mut Vec<String>) -> String {
    if comments.is_empty() {
        return String::new();
    }
    let summary = normalize_doc_text(&comments.join("\n"));
    comments.clear();
    summary
}

fn is_top_level(line: &str) -> bool {
    !line.starts_with(' ') && !line.starts_with('\t')
}

fn extract_python_docstrings(source: &str) -> Result<BTreeMap<String, String>> {
    let lines = source.lines().collect::<Vec<_>>();
    let mut docs = BTreeMap::new();
    let mut index = 0;

    while index < lines.len() {
        let trimmed = lines[index].trim_start();
        let Some(rest) = trimmed.strip_prefix("def ") else {
            index += 1;
            continue;
        };
        let Some(name_end) = rest.find('(') else {
            bail!("failed to parse Python function name from `{trimmed}`");
        };
        let name = rest[..name_end].trim().to_string();

        let mut doc_index = index + 1;
        while doc_index < lines.len() && lines[doc_index].trim().is_empty() {
            doc_index += 1;
        }
        if doc_index >= lines.len() {
            break;
        }

        let doc_line = lines[doc_index].trim_start();
        if doc_line.starts_with("\"\"\"") || doc_line.starts_with("'''") {
            let (doc, next_index) = parse_python_docstring(&lines, doc_index)?;
            docs.insert(name, doc);
            index = next_index;
            continue;
        }

        index = doc_index;
    }

    Ok(docs)
}

fn parse_python_docstring(lines: &[&str], start: usize) -> Result<(String, usize)> {
    let line = lines[start].trim_start();
    let quote = if line.starts_with("\"\"\"") {
        "\"\"\""
    } else if line.starts_with("'''") {
        "'''"
    } else {
        bail!("expected Python docstring at line {}", start + 1);
    };

    let rest = &line[quote.len()..];
    if let Some(end) = rest.find(quote) {
        return Ok((normalize_doc_text(&rest[..end]), start + 1));
    }

    let mut parts = vec![rest.to_string()];
    let mut index = start + 1;
    while index < lines.len() {
        let current = lines[index];
        if let Some(end) = current.find(quote) {
            parts.push(current[..end].to_string());
            return Ok((normalize_doc_text(&parts.join("\n")), index + 1));
        }
        parts.push(current.to_string());
        index += 1;
    }

    Err(anyhow!(
        "unterminated Python docstring starting at line {}",
        start + 1
    ))
}
