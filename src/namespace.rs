use serde::{ser::SerializeStruct, Serialize, Serializer};
use std::{
    collections::{HashMap, HashSet},
    ptr::NonNull,
};

use crate::{
    message::{Enum, Message},
    service::Service,
};

#[derive(Serialize, Debug)]
#[serde(remote = "Self")]
pub struct Namespace {
    #[serde(skip_serializing, skip_deserializing)]
    pub fullname: String,

    #[serde(skip_serializing, skip_deserializing)]
    pub imports: HashSet<String>,

    #[serde(skip_serializing, skip_deserializing)]
    parent: Option<NonNull<Namespace>>,

    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    nested: HashMap<String, Box<Namespace>>,

    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    services: HashMap<String, Service>,

    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    messages: HashMap<String, Message>,

    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    enums: HashMap<String, Enum>,
}

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

    pub fn root() -> Box<Namespace> {
        Namespace::new("", None)
    }

    pub fn as_ptr(&mut self) -> Option<NonNull<Namespace>> {
        NonNull::new(self)
    }

    pub fn add_import(&mut self, import: String) {
        self.imports.insert(import);
    }

    pub fn add_message(&mut self, name: String, message: Message) {
        self.messages.insert(name, message);
    }

    pub fn add_enum(&mut self, name: String, e: Enum) {
        self.enums.insert(name, e);
    }

    pub fn add_service(&mut self, service: Service) {
        self.services.insert(service.name.to_string(), service);
    }

    fn or_insert_with_child(&mut self, name: &str, fullname: &str) -> &mut Box<Namespace> {
        let parent = NonNull::new(self);
        self.nested
            .entry(name.to_string())
            .or_insert_with(|| Namespace::new(fullname, parent))
    }

    pub fn parent(&self) -> Option<&Namespace> {
        self.parent.as_ref().map(|x| unsafe { x.as_ref() })
        // Should be ok, because if this Node has a parent,
        // then the parent must be currently borrowed
        // so that self could be borrowed
    }

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

    pub fn append_child<'a>(&mut self, child: Box<Namespace>) {
        let mut paths = child.fullname.split(".");
        let mut fullname = self.fullname.to_string();
        let mut ptr = self;

        while let Some(name) = paths.next() {
            if !fullname.is_empty() {
                fullname.push('.');
            }
            fullname.push_str(name);
            ptr = ptr.or_insert_with_child(name, fullname.as_str());
        }

        ptr.merge_with(child);
    }

    fn merge_with(&mut self, other: Box<Namespace>) {
        let Namespace {
            messages,
            enums,
            services,
            ..
        } = *other;

        self.messages.extend(messages);
        self.enums.extend(enums);
        self.services.extend(services);
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
