use crate::{
    message::{Enum, Message, Type, SCALARS},
    service::Service,
};
use serde::{ser::SerializeStruct, Serialize, Serializer};
use std::{
    collections::{HashMap, HashSet},
    ptr::NonNull,
};

pub trait IntoPath {
    fn into_path(self) -> Vec<String>;
    fn to_path(&self) -> Vec<&str>;

    fn to_absolute_path(&self, type_name: &str) -> String {
        format!(".{}.{}", self.to_path().join("."), type_name)
    }
}

impl IntoPath for &str {
    fn into_path(self) -> Vec<String> {
        self.split('.').into_iter().map(|s| s.to_string()).collect()
    }

    fn to_path(&self) -> Vec<&str> {
        self.split('.').into_iter().collect()
    }
}

impl IntoPath for String {
    fn into_path(self) -> Vec<String> {
        self.split('.').into_iter().map(|s| s.to_string()).collect()
    }

    fn to_path(&self) -> Vec<&str> {
        self.split('.').into_iter().collect()
    }
}

impl IntoPath for Vec<String> {
    fn into_path(self) -> Vec<String> {
        self
    }

    fn to_path(&self) -> Vec<&str> {
        self.iter().map(|s| s.as_str()).collect()
    }
}

/// A Namespace represents a serialized proto package
#[derive(Serialize, Debug)]
#[serde(remote = "Self")]
pub struct Namespace {
    /// The namespace's path: e.g pb.foo.bar => ["pb", "foo", "bar"]
    #[serde(skip_serializing)]
    pub path: Vec<String>,

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

    /// A map of name => Type (Enum or Message) defined in this namespace
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    types: HashMap<String, Type>,
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

pub struct TypePath<'a> {
    /// The namespace path, e.g "pb.api.foo"
    pub namespace_path: &'a str,

    /// For nested messages, the nested message path e.g "SearchResponse.Inner"
    pub message_path: Option<&'a str>,
}

impl Namespace {
    /// Returns a new namespace
    pub fn new<P>(path: P, parent: Option<NonNull<Namespace>>) -> Box<Namespace>
    where
        P: IntoPath,
    {
        Box::new(Self {
            path: path.into_path(),
            imports: HashSet::new(),
            nested: HashMap::new(),
            types: HashMap::new(),
            services: HashMap::new(),
            parent,
        })
    }

    /// Returns a root namespace with no parent
    pub fn empty() -> Box<Namespace> {
        Self::new(Vec::new(), None)
    }

    /// Add an import statement
    pub fn add_import(&mut self, import: String) {
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
    /// If there is already a namespace with the same path, it will be merged with child
    pub fn append_child(&mut self, child: Box<Namespace>) {
        let mut ptr = self;

        let Namespace {
            path,
            types,
            services,
            ..
        } = *child;

        for key in path {
            ptr = ptr.nested.entry(key).or_insert(Namespace::empty())
        }

        ptr.types.extend(types);
        ptr.services.extend(services);
    }

    /// Go through all types referenced by this namespace, and normalize them
    /// references to types within the same namespace
    pub fn normalize_types(&self, dependencies: Vec<&Namespace>) {
        // collect services request and response types
        // we want to resolve services with an absolute path so we map to (service_type, false)
        let service_types = self
            .services
            .values()
            .flat_map(|service| service.methods.values())
            .flat_map(|method| [&method.request_type, &method.response_type])
            .map(|t| (t, false));

        // collect message types
        // we want to resolve messages relative to the namespace with a relative path so we map to (msg_type, false)
        let msg_types = self
            .types
            .values()
            .flat_map(|t| t.as_message())
            .flat_map(|msg| msg.messages_iter())
            .flat_map(|t| t.fields.values())
            .map(|f| (&f.type_name, true));

        // concat both
        let all_types = service_types.chain(msg_types);

        'outer: for (type_name, resolve_local) in all_types {
            let mut borrowed_type = type_name.borrow_mut();

            // skip scalar types
            if SCALARS.contains(borrowed_type.as_str()) {
                continue;
            }

            let path = borrowed_type.to_path();

            match (resolve_local, self.resolve_path(&path)) {
                (true, Some(path)) => {
                    *borrowed_type = path;
                    continue;
                }
                (false, Some(path)) => {
                    *borrowed_type = self.path.to_absolute_path(path.as_str());
                    continue;
                }
                _ => {}
            }

            // resolve imported type
            for ns in &dependencies {
                if let Some(path) = ns.resolve_path(&path) {
                    *borrowed_type = ns.path.to_absolute_path(path.as_str());
                    continue 'outer;
                }
            }

            // miss
            println!("miss for {:?}", borrowed_type);
        }
    }

    /// Given a type path (e.g pb.hello.HelloRequest)
    /// return the path relative to the namespace if this namespace contain a message or nested message at this location    
    pub fn resolve_path(&self, path: &Vec<&str>) -> Option<String> {
        // get the path relative to the namespace
        let index = self.relative_path_index(path);
        let mut segments = path.iter().skip(index).map(|it| *it);

        // look for the type in the namespace using the first segment
        let mut found_type = match segments.next() {
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
            found_type = match segments.next() {
                None => {
                    // we have exhausted the iterator, we can return the current resolved type
                    let relative_path: Vec<&str> = path.iter().skip(index).map(|it| *it).collect();
                    return Some(relative_path.join("."));
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

    fn relative_path_index(&self, object_type: &Vec<&str>) -> usize {
        // // e.g [example, hello, Request]
        let mut obj_it = object_type.iter().map(|it| *it);

        // // get the first object segment
        if let Some(first_segment) = obj_it.next() {
            // find the position of the first segment in the namespace
            if let Some(n) = self
                .path
                .iter()
                .position(|segment| segment == first_segment)
            {
                // iterate as long as namespace and object segments match
                let mut zip = self.path.iter().skip(n + 1).zip(obj_it);
                let mut n: usize = 1;
                while let Some((ns_segment, obj_segment)) = zip.next() {
                    if ns_segment == obj_segment {
                        n += 1
                    } else {
                        break;
                    }
                }

                return n;
            } else {
                return 0;
            }
        }

        return 0;
    }
}

#[cfg(test)]
mod tests {

    use crate::message::Message;

    use super::{IntoPath, Namespace};

    #[test]
    fn test_add_child() {
        let mut root = Namespace::empty();
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
    }

    macro_rules! test_relative_path {
        ($name:ident, $object:expr, $namespace:expr, $result:expr) => {
            #[test]
            fn $name() {
                let object = $object.split('.').collect();
                let ns = Namespace::new($namespace.into_path(), None);
                let index = ns.relative_path_index(&object);
                let res = object.iter().skip(index).map(|x| *x).collect::<Vec<&str>>();
                assert!(res.join(".") == $result);
            }
        };
    }

    test_relative_path!(
        test_relative_path_from_fully_qualified_type,
        "pb.example.Request",
        "pb.example",
        "Request"
    );

    test_relative_path!(
        test_relative_path_from_partial_qualified_type,
        "example.Request",
        "pb.example",
        "Request"
    );

    test_relative_path!(
        test_relative_path_from_different_namespace,
        "example.Request",
        "pb.other",
        "example.Request"
    );

    #[test]
    fn test_resolve_path() {
        let mut ns = Namespace::new("pb.lyft.otamanager".into_path(), None);
        ns.add_message("CheckInRequest", Message::new());
        let res = ns.resolve_path(&"otamanager.CheckInRequest".split('.').collect());

        println!("resolve_path {:?}", res);
    }
}
