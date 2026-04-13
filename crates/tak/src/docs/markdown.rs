pub(super) fn extract_intro_paragraph(markdown: &str) -> Option<String> {
    let mut seen_title = false;
    let mut lines = Vec::new();

    for line in markdown.lines() {
        let trimmed = line.trim();
        if !seen_title {
            if trimmed.starts_with("# ") {
                seen_title = true;
            }
            continue;
        }

        if trimmed.is_empty() {
            if !lines.is_empty() {
                break;
            }
            continue;
        }

        if trimmed.starts_with("## ") {
            break;
        }

        lines.push(trimmed);
    }

    if lines.is_empty() {
        None
    } else {
        Some(lines.join(" "))
    }
}

pub(super) fn extract_bullets(markdown: &str, heading: &str) -> Option<Vec<String>> {
    let section = extract_section(markdown, heading)?;
    let bullets = section
        .iter()
        .filter_map(|line| line.trim().strip_prefix("- ").map(ToOwned::to_owned))
        .collect::<Vec<_>>();
    if bullets.is_empty() {
        None
    } else {
        Some(bullets)
    }
}

pub(super) fn extract_first_fenced_block(markdown: &str, heading: &str) -> Option<String> {
    let section = extract_section(markdown, heading)?;
    let mut in_fence = false;
    let mut lines = Vec::new();

    for line in section {
        let trimmed = line.trim();
        if !in_fence {
            if trimmed.starts_with("```") {
                in_fence = true;
            }
            continue;
        }

        if trimmed == "```" {
            return Some(lines.join("\n"));
        }

        lines.push(line.to_string());
    }

    None
}

fn extract_section<'a>(markdown: &'a str, heading: &str) -> Option<Vec<&'a str>> {
    let mut in_section = false;
    let mut lines = Vec::new();

    for line in markdown.lines() {
        let trimmed = line.trim_end();
        if !in_section {
            if trimmed == heading {
                in_section = true;
            }
            continue;
        }

        if trimmed.starts_with("## ") {
            break;
        }

        lines.push(line);
    }

    if in_section { Some(lines) } else { None }
}
