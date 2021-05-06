use crate::{
    message::{Enum, Message},
    service::Service,
};
use serde::{ser::SerializeStruct, Serialize, Serializer};
use std::{
    collections::{HashMap, HashSet},
    ptr::NonNull,
};

/// A Namespace represents a serialized proto package
#[derive(Serialize, Debug)]
#[serde(remote = "Self")]
pub struct Namespace {
    /// The namespace's full name: e.g pb.foo.bar
    #[serde(skip_serializing)]
    pub fullname: String,

    /// List of import statements used to resolve this package's dependencies
    #[serde(skip_serializing)]
    pub imports: HashSet<String>,

    /// A pointer to the parent's namespace
    #[serde(skip_serializing)]
    parent: Option<NonNull<Namespace>>,

    /// A list of nested namespaces
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    nested: HashMap<String, Box<Namespace>>,

    /// A map of name => Service defined in this namespace
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    services: HashMap<String, Service>,

    /// A map of name => Message defined in this namespace
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    messages: HashMap<String, Message>,

    /// A map of name => Enum defined in this namespace
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    enums: HashMap<String, Enum>,
}

/// Wrap the namespace into a wrapper struct to match the serialization format of protobuf.js
impl Serialize for Namespace {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        struct Wrapper<'a> {
            root: &'a Namespace,
        }

        impl<'a> Serialize for Wrapper<'a> {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                Namespace::serialize(&self.root, serializer)
            }
        }

        let mut state = serializer.serialize_struct("Wrapper", 1)?;
        state.serialize_field("nested", &Wrapper { root: self })?;
        state.end()
    }
}

impl Namespace {
    /// Returns a new namespace
    pub fn new(fullname: &str, parent: Option<NonNull<Namespace>>) -> Box<Namespace> {
        Box::new(Self {
            fullname: fullname.to_string(),
            imports: HashSet::new(),
            nested: HashMap::new(),
            messages: HashMap::new(),
            enums: HashMap::new(),
            services: HashMap::new(),
            parent,
        })
    }

    /// Returns a root namespace with no parent
    pub fn root() -> Box<Namespace> {
        Namespace::new("", None)
    }

    /// Add an import statement
    pub fn add_import(&mut self, import: String) {
        self.imports.insert(import);
    }

    /// Add a message
    pub fn add_message(&mut self, name: String, message: Message) {
        self.messages.insert(name, message);
    }

    /// Add an enum
    pub fn add_enum(&mut self, name: String, e: Enum) {
        self.enums.insert(name, e);
    }

    /// Add an service
    pub fn add_service(&mut self, name: String, service: Service) {
        self.services.insert(name, service);
    }

    /// Get a reference to the parent
    pub fn parent(&self) -> Option<&Namespace> {
        // Should be ok, because if this namespace has a parent,
        // then the parent must be currently borrowed so that child could be borrowed
        self.parent.as_ref().map(|x| unsafe { x.as_ref() })
    }

    /// Find the child for the given path
    pub fn child(&self, path: &str) -> Option<&Namespace> {
        let mut paths = path.split(".");
        let mut ptr = self;

        while let Some(name) = paths.next() {
            match ptr.nested.get(name) {
                Some(child) => ptr = child,
                None => return None,
            }
        }

        Some(ptr)
    }

    /// Append a child to the current namespace.
    /// If there is already a namespace with the same name, it will be merged with child
    pub fn append_child(&mut self, child: Box<Namespace>) {
        let mut paths = child.fullname.split(".");
        let mut fullname = self.fullname.to_string();
        let mut ptr = self;

        while let Some(name) = paths.next() {
            if !fullname.is_empty() {
                fullname.push('.');
            }
            fullname.push_str(name);
            ptr = ptr.get_or_insert_with_child(name, fullname.as_str());
        }

        ptr.merge_with(child);
    }

    /// Get the child with the specified name or insert a new one at this location
    fn get_or_insert_with_child(&mut self, name: &str, fullname: &str) -> &mut Box<Namespace> {
        let parent = NonNull::new(self);
        self.nested
            .entry(name.to_string())
            .or_insert_with(|| Namespace::new(fullname, parent))
    }

    /// Merge the `other` namespace into self
    fn merge_with(&mut self, other: Box<Namespace>) {
        let Namespace {
            messages,
            enums,
            services,
            imports,
            nested,
            ..
        } = *other;

        self.nested.extend(nested);
        self.messages.extend(messages);
        self.enums.extend(enums);
        self.services.extend(services);
        self.imports.extend(imports);
    }
}

#[cfg(test)]
mod tests {
    use super::Namespace;

    #[test]
    fn test_add_child() {
        let mut root = Namespace::root();
        let child = Namespace::new("pb.foo.bar", None);
        root.append_child(child);

        let pb = root.child("pb");
        assert!(pb.is_some(), "should have a pb child");
        let pb = pb.unwrap();
        assert!(pb.parent.is_some(), "should have a pb child");

        let foo = pb.child("foo");
        assert!(foo.is_some(), "should have a pb.foo child");
        let foo = foo.unwrap();
        assert!(foo.parent.is_some(), "should have a pb child");

        let bar = foo.child("bar");
        assert!(bar.is_some(), "should have a pb.foo.bar child");
        let bar = bar.unwrap();
        assert!(bar.parent.is_some(), "should have a pb child");

        // assert_eq!(root.child("pb").unwrap().fullname, "pb");
        // assert_eq!(root.child("pb.foo").unwrap().fullname, "pb.foo");
    }
}
