use derive_more::Display;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
pub struct Message {
    fields: HashMap<String, Field>,

    #[serde(skip_serializing_if = "HashMap::is_empty")]
    oneofs: HashMap<String, Oneof>,

    #[serde(skip_serializing_if = "HashMap::is_empty")]
    nested: HashMap<String, NestedObject>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum NestedObject {
    Message(Message),
    Enum(Enum),
}

#[derive(Debug, Serialize)]
pub struct Enum {
    values: HashMap<String, i32>,
}

impl Enum {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: String, value: i32) {
        self.values.insert(key, value);
    }
}

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

#[derive(Debug, Serialize)]
pub struct Field {
    pub id: u32,

    #[serde(rename = "keyType", skip_serializing_if = "Option::is_none")]
    pub key_type: Option<String>,

    #[serde(rename = "type")]
    pub type_name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule: Option<FieldRule>,
}

impl Field {
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

#[derive(Debug, Serialize)]
pub struct Oneof {
    #[serde(rename = "oneof")]
    pub values: Vec<String>,
}

impl Oneof {
    pub fn new() -> Self {
        Self { values: Vec::new() }
    }

    pub fn add_field_name(&mut self, value: String) {
        self.values.push(value);
    }
}

impl Message {
    pub fn new() -> Message {
        Self {
            fields: HashMap::new(),
            nested: HashMap::new(),
            oneofs: HashMap::new(),
        }
    }

    pub fn add_oneof(&mut self, name: String, oneof: Oneof) {
        self.oneofs.insert(name, oneof);
    }

    pub fn add_enum(&mut self, name: String, e: Enum) {
        self.nested.insert(name, NestedObject::Enum(e));
    }

    pub fn add_field(&mut self, name: String, field: Field) {
        self.fields.insert(name, field);
    }

    pub fn add_nested(&mut self, name: String, message: Message) {
        self.nested.insert(name, NestedObject::Message(message));
    }
}
