use crate::{
    field::Field,
    into_path::ToPath,
    namespace::Namespace,
    oneof::Oneof,
    parse_error::ResolveError,
    r#enum::Enum,
    r#type::{Resolver, Type},
    scalar::SCALARS,
};
use serde::Serialize;
use std::collections::HashMap;

/// Message defines a proto [message]
/// [message] https://developers.google.com/protocol-buffers/docs/proto3#simple
#[derive(Debug, Default, Serialize)]
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

        true
    }

    /// Add a oneof field
    pub fn add_oneof(&mut self, name: String, oneof: Oneof) {
        self.oneofs.insert(name, oneof);
    }

    /// Add a nested enum
    pub fn add_nested_enum(&mut self, name: String, e: Enum) {
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

    /// Resolve and update all the types referenced inside this message to their absolute path
    /// We iterate through the fields and the nested messages
    pub fn resolve_types(
        &self,
        dependencies: &[&Namespace],
        resolve_path: Vec<(&str, &HashMap<String, Type>)>,
    ) -> Result<(), ResolveError> {
        'fields: for (field_name, field) in self.fields.iter() {
            let mut type_name = field.type_name.borrow_mut();

            // Skip scalars
            if SCALARS.contains(type_name.as_str()) {
                continue;
            }

            // The field's path (e.g pb.example.one.One.OneInner)
            let mut type_path = type_name.split('.');

            // Resolve absolute types starting with a "." by using the list of namespace dependencies
            if type_name.starts_with('.') {
                type_path.next(); // skip first
                for ns in dependencies {
                    if ns.resolve_path(type_path.clone()).is_some() {
                        continue 'fields;
                    }
                }

                return Err(ResolveError::UnresolvedField {
                    type_name: type_name.to_string(),
                    field: field_name.to_string(),
                });
            }

            // Walk through the resolve path backward until we resolve the type
            // e.g if the message is defined in One.OneInner, we first try to find it in OneInner, then One, ...
            for (index, (_, types)) in resolve_path.iter().rev().enumerate() {
                if types.contains_path(type_path.clone()) {
                    *type_name = resolve_path
                        .iter()
                        .take(resolve_path.len() - index)
                        .map(|(s, _)| *s)
                        .chain(type_path)
                        .collect::<Vec<_>>()
                        .to_path_string();

                    continue 'fields;
                }
            }

            // The type was not found in the nested messages, We try to resolve it through the dependencies
            for ns in dependencies.iter() {
                if let Some(path) = ns.resolve_path(type_path.clone()) {
                    *type_name = path;
                    continue 'fields;
                }
            }

            return Err(ResolveError::UnresolvedField {
                type_name: type_name.to_string(),
                field: field_name.to_string(),
            });
        }

        // Resolve nested messages
        for (name, t) in self.nested.iter() {
            if let Some(msg) = t.as_message() {
                let mut resolve_path = resolve_path.clone();
                resolve_path.push((name.as_str(), &msg.nested));
                msg.resolve_types(dependencies, resolve_path)?;
            }
        }

        Ok(())
    }
}
