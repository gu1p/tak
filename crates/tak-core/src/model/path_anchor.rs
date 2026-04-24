use super::*;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PathNormalizationError {
    #[error("path anchor cannot be empty")]
    EmptyAnchor,
    #[error("repo anchor name cannot be empty")]
    EmptyRepoAnchor,
    #[error("unsupported anchor `{0}`")]
    UnsupportedAnchor(String),
    #[error("path escapes anchor `{anchor}`: {path}")]
    EscapesAnchor { anchor: String, path: String },
}

pub(crate) fn parse_anchor(anchor: &str) -> Result<PathAnchor, PathNormalizationError> {
    let normalized = anchor.trim();
    if normalized.is_empty() {
        return Err(PathNormalizationError::EmptyAnchor);
    }

    match normalized {
        "workspace" => Ok(PathAnchor::Workspace),
        "package" => Ok(PathAnchor::Package),
        _ => {
            if let Some(repo) = normalized.strip_prefix("repo:") {
                let repo = repo.trim();
                if repo.is_empty() {
                    return Err(PathNormalizationError::EmptyRepoAnchor);
                }
                return Ok(PathAnchor::Repo(repo.to_string()));
            }
            Err(PathNormalizationError::UnsupportedAnchor(
                normalized.to_string(),
            ))
        }
    }
}
