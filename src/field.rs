use derive_more::Display;
use serde::Serialize;
use std::cell::RefCell;

use crate::metadata::Metadata;

/// FieldRule represents a proto [field rule]
/// [field rule] https://developers.google.com/protocol-buffers/docs/proto#specifying_field_rules
#[derive(Display, Debug, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FieldRule {
    #[display(fmt = "repeated")]
    Repeated,

    #[display(fmt = "optional")]
    Optional,

    #[display(fmt = "required")]
    Required,
}

/// Field represents a proto message [field]
/// [field] https://developers.google.com/protocol-buffers/docs/proto#specifying_field_types
#[derive(Serialize, Debug)]
pub struct Field {
    // The type of the field
    #[serde(rename = "type")]
    pub type_name: RefCell<String>,

    // The field Id
    pub id: u32,

    // For map the type of the key
    #[serde(rename = "keyType", skip_serializing_if = "Option::is_none")]
    pub key_type: Option<String>,

    // The field rule associated with this type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule: Option<FieldRule>,

    /// metadata associated to the Enum
    #[serde(skip_serializing)]
    pub md: Metadata,
}

impl Field {
    /// Creates a new field
    pub fn new(
        id: u32,
        type_name: String,
        rule: Option<FieldRule>,
        key_type: Option<String>,
        md: Metadata,
    ) -> Field {
        Self {
            id,
            type_name: RefCell::new(type_name),
            rule,
            key_type,
            md,
        }
    }
}
