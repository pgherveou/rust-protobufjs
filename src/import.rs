/// Import represents a proto [import statement]
/// [import statement] https://developers.google.com/protocol-buffers/docs/proto#importing_definitions
#[derive(Debug, Hash, PartialEq, Eq)]
pub enum Import {
    Public(String),
    Internal(String),
}

impl Import {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Public(v) => v.as_str(),
            Self::Internal(v) => v.as_str(),
        }
    }
}
