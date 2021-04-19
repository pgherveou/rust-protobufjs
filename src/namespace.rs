use serde::{ser::SerializeStruct, Serialize, Serializer};
use std::{collections::HashMap, ptr::NonNull};

use crate::{
    message::{EnumTuple, Message},
    service::Service,
};

// pub fn ser_with<S>(id: &String, s: S) -> Result<S::Ok, S::Error>
// where
//     S: Serializer,
// {
//     let mut ser = s.serialize_map(Some(1))?;
//     ser.serialize_entry("$oid", &id)?;
//     ser.end()
// }

#[derive(Serialize, Debug)]
#[serde(remote = "Self")]
pub struct Namespace {
    #[serde(skip_serializing, skip_deserializing)]
    pub fullname: String,

    #[serde(skip_serializing, skip_deserializing)]
    parent: Option<NonNull<Namespace>>,

    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    nested: HashMap<String, Box<Namespace>>,

    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    services: HashMap<String, Service>,

    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    messages: HashMap<String, Message>,

    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    enums: HashMap<String, Vec<EnumTuple>>,
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
    fn new(fullname: &str, parent: Option<NonNull<Namespace>>) -> Box<Namespace> {
        Box::new(Self {
            fullname: fullname.to_string(),
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

    pub fn add_message(&mut self, name: String, message: Message) {
        self.messages.insert(name, message);
    }

    pub fn add_enum(&mut self, name: String, enum_tuples: Vec<EnumTuple>) {
        self.enums.insert(name, enum_tuples);
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

    pub fn define<'a>(&mut self, path: &'a str) -> &mut Namespace {
        let mut paths = path.split(".");
        let mut fullname = self.fullname.to_string();
        let mut ptr = self;

        while let Some(name) = paths.next() {
            if !fullname.is_empty() {
                fullname.push('.');
            }
            fullname.push_str(name);
            ptr = ptr.or_insert_with_child(name, fullname.as_str());
        }

        ptr
    }
}

#[cfg(test)]
mod tests {
    use super::Namespace;

    #[test]
    fn test_add_child() {
        let mut root = Namespace::root();
        let ns = root.define("pb.foo.bar");

        assert_eq!(ns.fullname, "pb.foo.bar");
        assert_eq!(ns.parent().unwrap().fullname, "pb.foo");

        assert_eq!(root.child("pb").unwrap().fullname, "pb");
        assert_eq!(root.child("pb.foo").unwrap().fullname, "pb.foo");
    }
}
