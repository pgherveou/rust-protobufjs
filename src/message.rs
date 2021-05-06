use derive_more::Display;
use serde::Serialize;
use std::collections::HashMap;

/// Message defines a proto [message]
/// [message] https://developers.google.com/protocol-buffers/docs/proto3#simple
#[derive(Debug, Serialize)]
pub struct Message {
    /// A map of name => fields
    fields: HashMap<String, Field>,

    /// A map of name => oneof
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    oneofs: HashMap<String, Oneof>,

    /// A map of name => nested message or enum
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    nested: HashMap<String, NestedObject>,
}

impl Message {
    /// Returns a new Message
    pub fn new() -> Message {
        Self {
            fields: HashMap::new(),
            nested: HashMap::new(),
            oneofs: HashMap::new(),
        }
    }

    /// Add a oneof field
    pub fn add_oneof(&mut self, name: String, oneof: Oneof) {
        self.oneofs.insert(name, oneof);
    }

    /// Add a nested enum
    pub fn add_nested_enum(&mut self, name: String, e: Enum) {
        self.nested.insert(name, NestedObject::Enum(e));
    }

    /// Add a nested message
    pub fn add_nested_message(&mut self, name: String, message: Message) {
        self.nested.insert(name, NestedObject::Message(message));
    }

    /// Add a message field
    pub fn add_field(&mut self, name: String, field: Field) {
        self.fields.insert(name, field);
    }
}

/// NestedObject defines a nested proto object
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum NestedObject {
    Message(Message),
    Enum(Enum),
}

/// Enum defines a proto [emum]
/// [enum] https://developers.google.com/protocol-buffers/docs/proto3#enum
#[derive(Debug, Serialize)]
pub struct Enum {
    /// a map of name => field id
    values: HashMap<String, i32>,
}

impl Enum {
    /// Returns a new Enum
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    /// Insert a new field with the given key and id
    pub fn insert(&mut self, key: String, id: i32) {
        self.values.insert(key, id);
    }
}

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
#[derive(Debug, Serialize)]
pub struct Field {
    // The field Id
    id: u32,

    // For map the type of the key
    #[serde(rename = "keyType", skip_serializing_if = "Option::is_none")]
    key_type: Option<String>,

    // the type of the field
    #[serde(rename = "type")]
    type_name: String,

    // the field rule associated with this type
    #[serde(skip_serializing_if = "Option::is_none")]
    rule: Option<FieldRule>,
}

impl Field {
    /// Creates a new field
    pub fn new(
        id: u32,
        type_name: String,
        rule: Option<FieldRule>,
        key_type: Option<String>,
    ) -> Field {
        Self {
            id,
            type_name,
            rule,
            key_type,
        }
    }
}

/// Oneof represents a proto [oneof] field
/// [oneof] https://developers.google.com/protocol-buffers/docs/proto#oneof
#[derive(Debug, Serialize)]
pub struct Oneof {
    #[serde(rename = "oneof")]
    values: Vec<String>,
}

impl Oneof {
    /// Returns a new oneof
    pub fn new() -> Self {
        Self { values: Vec::new() }
    }

    /// Add a field to the oneof
    pub fn add_field_name(&mut self, value: String) {
        self.values.push(value);
    }
}
