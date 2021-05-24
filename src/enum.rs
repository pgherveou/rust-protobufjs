use serde::Serialize;
use std::collections::HashMap;

/// Enum defines a proto [emum]
/// [enum] https://developers.google.com/protocol-buffers/docs/proto3#enum
#[derive(Debug, Default, Serialize)]
pub struct Enum {
    /// a map of name => field id
    pub values: HashMap<String, i32>,
}

impl Enum {
    /// Insert a new field with the given key and id
    pub fn insert(&mut self, key: String, id: i32) {
        self.values.insert(key, id);
    }
}
