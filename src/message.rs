use std::collections::HashMap;

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Message {
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    oneofs: HashMap<String, Vec<String>>,

    fields: HashMap<String, Field>,

    #[serde(skip_serializing_if = "HashMap::is_empty")]
    nested: HashMap<String, Message>,

    #[serde(skip_serializing_if = "HashMap::is_empty")]
    enums: HashMap<String, Vec<EnumTuple>>,
}

#[derive(Debug, Serialize)]
pub struct EnumTuple(pub String, pub u32);

fn is_false(value: &bool) -> bool {
    *value == false
}

#[derive(Debug, Serialize)]
pub struct Field {
    pub id: u32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_type: Option<String>,
    pub type_name: String,

    #[serde(skip_serializing_if = "is_false")]
    pub repeated: bool,
}

impl Field {
    pub fn new(id: u32, type_name: String, repeated: bool, key_type: Option<String>) -> Field {
        Self {
            id,
            type_name,
            repeated,
            key_type,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Oneof(pub String, pub Vec<String>);

impl Oneof {
    pub fn new(name: String) -> Self {
        Oneof(name, Vec::new())
    }

    pub fn add_field_name(&mut self, value: String) {
        self.1.push(value);
    }
}

impl Message {
    pub fn new() -> Message {
        Self {
            fields: HashMap::new(),
            nested: HashMap::new(),
            oneofs: HashMap::new(),
            enums: HashMap::new(),
        }
    }

    pub fn add_oneof(&mut self, oneof: Oneof) {
        let Oneof(name, value) = oneof;
        self.oneofs.insert(name, value);
    }

    pub fn add_enum(&mut self, name: String, enum_tuples: Vec<EnumTuple>) {
        self.enums.insert(name, enum_tuples);
    }

    pub fn add_field(&mut self, name: String, field: Field) {
        self.fields.insert(name, field);
    }

    pub fn add_nested(&mut self, name: String, message: Message) {
        self.nested.insert(name, message);
    }
}
