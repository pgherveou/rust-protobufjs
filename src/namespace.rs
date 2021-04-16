use std::{collections::HashMap, ptr::NonNull};

use crate::{
    message::{Enum, Message},
    service::Service,
};

#[derive(Debug)]
pub struct Namespace {
    pub fullname: String,
    parent: Option<NonNull<Namespace>>,
    nested: HashMap<String, Box<Namespace>>,
    messages: Vec<Message>,
    enums: Vec<Enum>,
    services: Vec<Service>,
}

impl Namespace {
    fn new(fullname: &str, parent: Option<NonNull<Namespace>>) -> Box<Namespace> {
        Box::new(Self {
            fullname: fullname.to_string(),
            nested: HashMap::new(),
            messages: Vec::new(),
            enums: Vec::new(),
            services: Vec::new(),
            parent,
        })
    }

    pub fn root() -> Box<Namespace> {
        Namespace::new("", None)
    }

    pub fn as_ptr(&mut self) -> Option<NonNull<Namespace>> {
        NonNull::new(self)
    }

    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    pub fn add_enum(&mut self, e: Enum) {
        self.enums.push(e);
    }

    pub fn add_service(&mut self, service: Service) {
        self.services.push(service);
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
