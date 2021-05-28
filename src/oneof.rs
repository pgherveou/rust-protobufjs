use serde::Serialize;

use crate::metadata::Metadata;

/// Oneof represents a proto [oneof] field
/// [oneof] https://developers.google.com/protocol-buffers/docs/proto#oneof
#[derive(Debug, Serialize)]
pub struct Oneof {
    #[serde(rename = "oneof")]
    pub values: Vec<String>,

    /// metadata associated to the Enum
    #[serde(skip_serializing)]
    pub md: Metadata,
}

impl Oneof {
    // Returns a new Oneof with the provided metadata
    pub fn new(md: Metadata) -> Self {
        Self {
            values: Vec::new(),
            md,
        }
    }

    /// Add a field to the oneof
    pub fn add_field_name(&mut self, value: String) {
        self.values.push(value);
    }
}
