use super::*;

pub(crate) fn normalize_relative_path(
    anchor: &str,
    path: &str,
) -> Result<String, PathNormalizationError> {
    let mut parts = Vec::<String>::new();
    for segment in path.replace('\\', "/").split('/') {
        match segment {
            "" | "." => continue,
            ".." => {
                if parts.pop().is_none() {
                    return Err(PathNormalizationError::EscapesAnchor {
                        anchor: anchor.to_string(),
                        path: path.to_string(),
                    });
                }
            }
            value => parts.push(value.to_string()),
        }
    }

    if parts.is_empty() {
        return Ok(".".to_string());
    }
    Ok(parts.join("/"))
}
