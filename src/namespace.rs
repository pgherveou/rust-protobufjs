use crate::{
    import::Import, into_path::ToPath, iter_ext::IterExt, message::Message,
    parse_error::ResolveError, r#enum::Enum, r#type::Type, service::Service,
};
use serde::{ser::SerializeStruct, Serialize, Serializer};
use std::{
    collections::{HashMap, HashSet},
    str::Split,
};

/// A Namespace represents a serialized proto package
#[derive(Serialize, Debug)]
#[serde(remote = "Self")]
pub struct Namespace {
    /// The namespace's path: e.g pb.foo.bar => ["pb", "foo", "bar"]
    #[serde(skip_serializing)]
    pub path: Vec<String>,

    /// List of import statements used to resolve this package's dependencies
    #[serde(skip_serializing)]
    pub imports: HashSet<Import>,

    /// A list of nested namespaces
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub nested: HashMap<String, Namespace>,

    /// A map of name => Service defined in this namespace
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub services: HashMap<String, Service>,

    /// A map of name => Type (Enum or Message) defined in this namespace
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub types: HashMap<String, Type>,
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
    pub fn new(path: Vec<String>) -> Self {
        Self {
            path: path,
            imports: HashSet::new(),
            nested: HashMap::new(),
            types: HashMap::new(),
            services: HashMap::new(),
        }
    }

    pub fn empty() -> Self {
        Namespace::new(Vec::new())
    }

    /// Add an import statement
    pub fn add_import(&mut self, import: Import) {
        self.imports.insert(import);
    }

    /// Add a message
    pub fn add_message<S>(&mut self, name: S, message: Message)
    where
        S: Into<String>,
    {
        self.types.insert(name.into(), Type::Message(message));
    }

    /// Add an enum
    pub fn add_enum(&mut self, name: String, e: Enum) {
        self.types.insert(name, Type::Enum(e));
    }

    /// Add an service
    pub fn add_service(&mut self, name: String, service: Service) {
        self.services.insert(name, service);
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
    /// If there is already a namespace with the same path, it will be merged with child
    pub fn append_child(&mut self, child: Namespace) {
        let mut ptr = self;

        let Namespace {
            path,
            types,
            services,
            ..
        } = child;

        for key in path.into_iter() {
            ptr = ptr.nested.entry(key).or_insert(Namespace::empty())
        }

        ptr.types.extend(types);
        ptr.services.extend(services);
    }

    /// Resolve and update all the types referenced inside this namespace to their absolute path
    pub fn resolve_types(&self, dependencies: Vec<&Namespace>) -> Result<(), ResolveError> {
        let dependencies: Vec<_> = dependencies.into_iter().start_with(self).collect();

        // loop through all the types in the namespace
        for (name, t) in self.types.iter() {
            // filter messages only, since enum do not have fields to resolve
            let msg = match t {
                Type::Enum(_) => continue,
                Type::Message(msg) => msg,
            };

            msg.resolve_types(&dependencies, [(name.as_str(), &msg.nested)].into())?
        }

        // loop through all the services rpc request and response types
        let service_types = self
            .services
            .values()
            .flat_map(|service| service.methods.values())
            .flat_map(|method| [&method.request_type, &method.response_type]);

        'services: for type_ref in service_types {
            let mut type_ref = type_ref.borrow_mut();
            let path = type_ref.split('.');
            for ns in dependencies.iter() {
                if let Some(v) = ns.resolve_path(path.clone()) {
                    *type_ref = v;
                    continue 'services;
                }
            }

            return Err(ResolveError::UnresolvedRpcType(type_ref.to_string()));
        }

        Ok(())
    }

    /// Resolve the path against the namespace and return the absolute path when found
    pub fn resolve_path<'a, 'b>(&'a self, type_path: Split<'a, char>) -> Option<String> {
        let relative_path = type_path.relative_to(self.path.iter().map(|s| s.as_str()));
        let mut path = relative_path.clone();

        // look for the type in the namespace using the first segment
        let mut found_type = match path.next() {
            None => return None,
            Some(name) => {
                if let Some(t) = self.types.get(name) {
                    t
                } else {
                    return None;
                }
            }
        };

        // loop through nested messages
        loop {
            found_type = match path.next() {
                None => {
                    return Some(
                        self.path
                            .iter()
                            .map(|s| s.as_str())
                            .chain(relative_path)
                            .collect::<Vec<_>>()
                            .to_path_string(),
                    );
                }
                Some(name) => {
                    if let Some(t) = found_type.get(name) {
                        t
                    } else {
                        return None;
                    }
                }
            };
        }
    }
}

#[cfg(test)]
mod tests {

    use super::Namespace;

    // #[test]
    // fn test_add_child() {
    //     let mut root = Namespace::empty();
    //     let path = "pb.foo.bar"
    //         .split('.')
    //         .into_iter()
    //         .map(|s| s.to_string())
    //         .collect();

    //     let child = Namespace::new(path);
    //     root.append_child(child);

    //     let pb = root.child("pb");
    //     assert!(pb.is_some(), "should have a pb child");
    //     let pb = pb.unwrap();
    //     assert!(pb.parent.is_some(), "should have a pb child");

    //     let foo = pb.child("foo");
    //     assert!(foo.is_some(), "should have a pb.foo child");
    //     let foo = foo.unwrap();
    //     assert!(foo.parent.is_some(), "should have a pb child");

    //     let bar = foo.child("bar");
    //     assert!(bar.is_some(), "should have a pb.foo.bar child");
    //     let bar = bar.unwrap();
    //     assert!(bar.parent.is_some(), "should have a pb child");
    // }

    // #[test]
    // fn test_resolve_path() {
    //     let mut ns = Namespace::new("pb.lyft.otamanager".into_path(), None);
    //     ns.add_message("CheckInRequest", Message::new());
    //     let res = ns.resolve_path(&"otamanager.CheckInRequest".split('.').collect());

    //     println!("resolve_path {:?}", res);
    // }
}
