use std::fs;
use std::path::Path;

use crate::application::{MakefileReadError, MakefileReader, MakefileSource};

const DEFAULT_MAKEFILES: [&str; 3] = ["GNUmakefile", "makefile", "Makefile"];

/// Filesystem adapter implementing GNU Make's default Makefile precedence.
#[derive(Debug, Default, Clone, Copy)]
pub struct FilesystemMakefileReader;

impl MakefileReader for FilesystemMakefileReader {
    fn read(&self, workspace_root: &Path) -> Result<MakefileSource, MakefileReadError> {
        for name in DEFAULT_MAKEFILES {
            let path = workspace_root.join(name);
            match fs::read_to_string(&path) {
                Ok(contents) => {
                    return Ok(MakefileSource {
                        makefile_path: name.into(),
                        contents,
                    });
                }
                Err(source) if source.kind() == std::io::ErrorKind::NotFound => continue,
                Err(source) => return Err(MakefileReadError::Read { path, source }),
            }
        }
        Err(MakefileReadError::NotFound {
            workspace_root: workspace_root.to_path_buf(),
        })
    }
}
