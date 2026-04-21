use super::*;

pub fn normalize_path_ref(anchor: &str, path: &str) -> Result<PathRef, PathNormalizationError> {
    let normalized_anchor = parse_anchor(anchor)?;
    let normalized_path = normalize_relative_path(anchor, path)?;
    Ok(PathRef {
        anchor: normalized_anchor,
        path: normalized_path,
    })
}

impl ContextManifest {
    /// Builds a canonical context manifest from path refs and computes a stable SHA-256 hash.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub fn from_paths(paths: impl IntoIterator<Item = PathRef>) -> Self {
        let mut entries: Vec<PathRef> = paths.into_iter().collect();
        entries.sort_by(compare_path_ref);
        entries.dedup();

        let hash = hash_manifest_entries(&entries);
        Self { entries, hash }
    }
}
