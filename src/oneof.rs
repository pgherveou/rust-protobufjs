use serde::Serialize;

/// Oneof represents a proto [oneof] field
/// [oneof] https://developers.google.com/protocol-buffers/docs/proto#oneof
#[derive(Debug, Default, Serialize)]
pub struct Oneof {
    #[serde(rename = "oneof")]
    values: Vec<String>,
}

impl Oneof {
    /// Add a field to the oneof
    pub fn add_field_name(&mut self, value: String) {
        self.values.push(value);
    }
}
