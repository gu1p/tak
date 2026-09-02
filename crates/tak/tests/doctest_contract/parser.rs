/// Parses function name from a line when it begins a Rust function declaration.
pub(crate) fn parse_function_name(line: &str) -> Option<String> {
    let mut trimmed = line.trim_start();

    loop {
        if let Some(rest) = trimmed.strip_prefix("pub ") {
            trimmed = rest;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("async ") {
            trimmed = rest;
            continue;
        }
        if trimmed.starts_with("pub(") {
            let close = trimmed.find(')')?;
            let rest = trimmed.get(close + 1..)?;
            trimmed = rest.trim_start();
            continue;
        }
        break;
    }

    let rest = trimmed.strip_prefix("fn ")?;
    let name: String = rest
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .collect();

    if name.is_empty() { None } else { Some(name) }
}

/// Returns contiguous `///` doc lines associated with the function at `function_line`.
pub(crate) fn collect_function_doc_lines<'a>(
    lines: &'a [&str],
    function_line: usize,
) -> Option<Vec<&'a str>> {
    if function_line == 0 {
        return None;
    }

    let mut cursor = function_line;
    let mut seen_doc = false;

    while cursor > 0 {
        let previous = lines[cursor - 1].trim_start();
        if previous.starts_with("///") {
            seen_doc = true;
            cursor -= 1;
            continue;
        }
        if previous.starts_with("#[") || previous.trim().is_empty() {
            cursor -= 1;
            continue;
        }
        break;
    }

    if !seen_doc {
        return None;
    }

    let docs = lines[cursor..function_line]
        .iter()
        .copied()
        .filter(|line| line.trim_start().starts_with("///"))
        .collect::<Vec<_>>();

    Some(docs)
}
