use crate::{
    import::Import,
    into_path::{IntoPath, ToPath},
    iter_ext::IterExt,
    message::Message,
    parse_error::ResolveError,
    r#enum::Enum,
    r#type::Type,
    service::Service,
};
use linked_hash_map::LinkedHashMap;
use serde::{ser::SerializeStruct, Serialize, Serializer};
use std::{
    collections::{BTreeMap, HashSet},
    str::Split,
};

/// A Namespace represents a serialized proto package
#[derive(Serialize, Default, Debug)]
#[serde(remote = "Self")]
pub struct Namespace {
    /// The namespace's path: e.g pb.foo.bar => ["pb", "foo", "bar"]
    #[serde(skip_serializing)]
    pub path: Vec<String>,

    /// List of import statements used to resolve this package's dependencies
    #[serde(skip_serializing)]
    pub imports: HashSet<Import>,

    /// A list of nested namespaces
    #[serde(flatten, skip_serializing_if = "BTreeMap::is_empty")]
    pub nested: BTreeMap<String, Namespace>,

    /// A map of name => Service defined in this namespace
    #[serde(flatten, skip_serializing_if = "LinkedHashMap::is_empty")]
    pub services: LinkedHashMap<String, Service>,

    /// A map of name => Type (Enum or Message) defined in this namespace
    #[serde(flatten, skip_serializing_if = "LinkedHashMap::is_empty")]
    pub types: LinkedHashMap<String, Type>,
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
    pub fn new<T: IntoPath>(path: T) -> Self {
        Self {
            path: path.into_path(),
            imports: HashSet::new(),
            nested: BTreeMap::new(),
            types: LinkedHashMap::new(),
            services: LinkedHashMap::new(),
        }
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
        let paths = path.split('.');
        let mut ptr = self;

        for name in paths {
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
            ptr = ptr.nested.entry(key).or_insert_with(Namespace::default)
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
    pub fn resolve_path<'a>(&'a self, type_path: Split<'a, char>) -> Option<String> {
        let relative_path = type_path.relative_to(self.path.iter().map(|s| s.as_str()));
        let mut path = relative_path.clone();

        // look for the type in the namespace using the first segment
        let mut found_type = match path.next() {
            None => return None,
            Some(name) => self.types.get(name)?,
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
                Some(name) => found_type.get(name)?,
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::{message::Message, metadata::Metadata, namespace::Namespace};

    #[test]
    fn test_add_child() {
        let mut root = Namespace::default();
        root.append_child(Namespace::new("pb.foo.bar"));

        assert!(
            root.child("pb")
                .and_then(|c| c.child("foo"))
                .and_then(|c| c.child("bar"))
                .is_some(),
            "root should have pb.foo.bar"
        )
    }

    #[test]
    fn test_resolve_path() {
        let mut ns = Namespace::new("pb.foo.bar");
        let path: PathBuf = "test.proto".into();
        let md = Metadata::new(path.into(), None, 1);

        ns.add_message("Bar", Message::new(md));
        let path = ns.resolve_path("Bar".split('.'));
        assert_eq!(path, Some("pb.foo.bar.Bar".into()))
    }
}
