use serde::Serialize;
use std::collections::HashMap;

use crate::metadata::Metadata;

/// Enum defines a proto [emum]
/// [enum] https://developers.google.com/protocol-buffers/docs/proto3#enum
#[derive(Debug, Serialize)]
pub struct Enum {
    /// a map of name => field id
    pub values: HashMap<String, i32>,

    /// metadata associated to the Enum
    #[serde(skip_serializing)]
    pub md: Metadata,
}

impl Enum {
    /// Rerturns a new Enum
    pub fn new(md: Metadata) -> Self {
        Self {
            values: HashMap::new(),
            md,
        }
    }

    /// Insert a new field with the given key and id
    pub fn insert(&mut self, key: String, id: i32) {
        self.values.insert(key, id);
    }
}
