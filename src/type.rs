use std::{collections::HashMap, str::Split};

use crate::{message::Message, r#enum::Enum};
use serde::Serialize;

/// Type can be a message or enum
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum Type {
    Message(Message),
    Enum(Enum),
}

impl Type {
    /// Get the nested type with the provided key
    pub fn get<'a, 'b>(&'a self, key: &str) -> Option<&Type> {
        match self {
            Type::Enum(_) => None,
            Type::Message(msg) => msg.nested.get(key),
        }
    }

    /// Convert type to a message
    pub fn as_message(&self) -> Option<&Message> {
        match self {
            Type::Enum(_) => None,
            Type::Message(msg) => Some(msg),
        }
    }
}

//a trait used to look for a path inside a Type
pub trait Resolver {
    fn contains_path(&self, path: Split<char>) -> bool;
}

impl Resolver for Type {
    fn contains_path(&self, mut path: Split<char>) -> bool {
        match self {
            Type::Enum(_) => path.next().is_none(),
            Type::Message(msg) => msg.nested.contains_path(path),
        }
    }
}

impl Resolver for HashMap<String, Type> {
    fn contains_path(&self, mut path: Split<char>) -> bool {
        match path.next() {
            None => return true,
            Some(segment) => match self.get(segment) {
                None => return false,
                Some(t) => return t.contains_path(path),
            },
        }
    }
}
