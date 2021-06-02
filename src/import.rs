use std::path::{Path, PathBuf};

/// Import represents a proto [import statement]
/// [import statement]: https://developers.google.com/protocol-buffers/docs/proto#importing_definitions
#[derive(Debug, Hash, PartialEq, Eq)]
pub enum Import {
    Public(PathBuf),
    Internal(PathBuf),
}

impl Import {
    pub fn as_path(&self) -> &Path {
        match self {
            Self::Public(v) | Self::Internal(v) => v.as_path(),
        }
    }
}
