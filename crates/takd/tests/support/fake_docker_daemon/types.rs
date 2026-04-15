use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateRecord {
    pub container_id: String,
    pub image: Option<String>,
    pub cmd: Vec<String>,
    pub working_dir: Option<String>,
    pub binds: Vec<String>,
}

impl CreateRecord {
    pub fn is_probe(&self) -> bool {
        self.cmd
            .iter()
            .any(|value| value.contains(".tak-mount-visible"))
    }

    pub fn bind_source(&self) -> Option<PathBuf> {
        self.binds
            .first()
            .and_then(|bind| bind.split(':').next())
            .map(PathBuf::from)
    }
}

#[derive(Debug, Clone)]
pub struct FakeDockerConfig {
    pub visible_roots: Vec<PathBuf>,
    pub image_present: bool,
    pub arch: String,
}

impl Default for FakeDockerConfig {
    fn default() -> Self {
        Self {
            visible_roots: Vec::new(),
            image_present: true,
            arch: "x86_64".to_string(),
        }
    }
}
