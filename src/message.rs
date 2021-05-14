use derive_more::Display;
use phf::phf_set;
use serde::Serialize;
use std::{cell::RefCell, collections::HashMap};

/// Message defines a proto [message]
/// [message] https://developers.google.com/protocol-buffers/docs/proto3#simple
#[derive(Debug, Serialize)]
pub struct Message {
    /// A map of name => fields
    pub fields: HashMap<String, Field>,

    /// A map of name => oneof
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub oneofs: HashMap<String, Oneof>,

    /// A map of name => [nested] message or enum
    /// [nested] https://developers.google.com/protocol-buffers/docs/proto3#nested
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub nested: HashMap<String, Type>,
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

    /// returns true if the message contains the given path
    pub fn has<'a, 'b>(&'a self, mut paths: impl Iterator<Item = &'b str>) -> bool {
        let mut ptr = self;

        while let Some(name) = paths.next() {
            match ptr.nested.get(name) {
                None => return false,
                Some(Type::Message(msg)) => ptr = msg,
                Some(Type::Enum(_)) => return paths.next().is_none(),
            }
        }

        return true;
    }

    /// Add a oneof field
    pub fn add_oneof(&mut self, name: String, oneof: Oneof) {
        self.oneofs.insert(name, oneof);
    }

    /// Add a nested enum
    pub fn add_nested_enum(&mut self, name: String, e: Enum) {
        println!("add nested enum {}", name);
        self.nested.insert(name, Type::Enum(e));
    }

    /// Add a nested message
    pub fn add_nested_message(&mut self, name: String, message: Message) {
        self.nested.insert(name, Type::Message(message));
    }

    /// Add a message field
    pub fn add_field(&mut self, name: String, field: Field) {
        self.fields.insert(name, field);
    }

    pub fn messages_iter<'a>(&'a self) -> Box<dyn Iterator<Item = &Message> + 'a> {
        let iter = self
            .nested
            .values()
            .flat_map(|v| v.as_message())
            .flat_map(|msg| msg.messages_iter())
            .chain([self]);

        return Box::new(iter);
    }
}

/// scalars defines all the possible [scalar value types]
/// [scalar value types] https://developers.google.com/protocol-buffers/docs/overview#scalar
pub static SCALARS: phf::Set<&'static str> = phf_set! {
    "double", "float",
    "int32", "int64", "uint32", "uint64", "sint32", "sint64",
    "fixed32", "fixed64", "sfixed32", "sfixed64",
    "bool", "string", "bytes"
};

/// Type can be a message or enum
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum Type {
    Message(Message),
    Enum(Enum),
}

impl Type {
    pub fn has<'a, 'b>(&'a self, paths: impl Iterator<Item = &'b str>) -> bool {
        match self {
            Type::Enum(_) => false,
            Type::Message(msg) => msg.has(paths),
        }
    }

    pub fn get<'a, 'b>(&'a self, key: &str) -> Option<&Type> {
        match self {
            Type::Enum(_) => None,
            Type::Message(msg) => msg.nested.get(key),
        }
    }

    pub fn as_message(&self) -> Option<&Message> {
        match self {
            Type::Enum(_) => None,
            Type::Message(msg) => Some(msg),
        }
    }
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
#[derive(Serialize, Debug)]
pub struct Field {
    // The field Id
    pub id: u32,

    // For map the type of the key
    #[serde(rename = "keyType", skip_serializing_if = "Option::is_none")]
    pub key_type: Option<String>,

    // the type of the field
    #[serde(rename = "type")]
    pub type_name: RefCell<String>,

    // the field rule associated with this type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule: Option<FieldRule>,
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
            type_name: RefCell::new(type_name),
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
