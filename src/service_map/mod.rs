//! Generate a service map file from a Namespace, so we can quickly resolve request and response type for a given URL path
//!
//! # Example:
//! Given the following proto file:
//!
//! ```proto
//! package pb.hello;
//!
//! service HelloWorld {
//!   rpc LotsOfGreetings(stream SayHelloRequest) returns (SayHelloResponses) {}
//!   rpc SayHello (SayHelloRequest) returns (SayHelloResponse) {
//!       option (pgm.http.rule) = { GET: "/hello/<string:name>" };
//!   }
//! }
//! ```
//!
//! We will generate:
//! ```json
//! {
//!   "pb.hello.HelloWorld": {
//!     "LotsOfGreetings": {
//!       "grpc": ["pb.hello.SayHelloRequest", "pb.hello.SayHelloResponses", "/pb.hello.HelloWorld/LotsOfGreetings"]
//!     },
//!     "hello": {
//!       "*": {
//!         "get": ["pb.hello.SayHelloRequest", "pb.hello.SayHelloResponse", "/hello/:name"]
//!       }
//!    }
//! }
//!```

use crate::{http_options::HTTPOptions, namespace::Namespace, service::Rpc};
use serde::{Serialize, Serializer};
use std::{borrow::Cow, cell::Cell, collections::BTreeMap, vec};

/// A service tree map is a tree where:
///
/// - branches are segments of the url with dynamic segments replaced by "*", the final segment is the method type (grpc, get, post, ...)
/// - leaves are array [RequestTypeName, ResponseTypeName, URL]
pub type ServiceTreeMap<'a> = BTreeMap<Cow<'a, str>, ServiceMapNode<'a>>;

/// A branch or leaf of the service tree map
#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum ServiceMapNode<'a> {
    Branch(ServiceTreeMap<'a>),

    #[serde(serialize_with = "serialize_leaf")]
    Leaf {
        rpc: &'a Rpc,
        url: Cow<'a, str>,
    },
}

/// Remove the leading . from a type path
fn no_leading_dot(s: &str) -> &str {
    s.strip_prefix('.').unwrap_or(s)
}

/// Helper serde serializer function the serialize a leaf of a service tree
fn serialize_leaf<S>(rpc: &Rpc, url: &str, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let req = rpc.request_type.borrow();
    let req = req.as_str();

    let resp = rpc.response_type.borrow();
    let resp = resp.as_str();

    [no_leading_dot(req), no_leading_dot(resp), url].serialize(serializer)
}

impl<'a> ServiceMapNode<'a> {
    /// Unwrap a node as a branch of theservice tree map.
    /// This method will panicked if used on a leaf
    fn unwrap_as_branch(&mut self) -> &mut ServiceTreeMap<'a> {
        match self {
            Self::Branch(v) => v,
            Self::Leaf { rpc: _, url: _ } => panic!("unexpected service type"),
        }
    }
}

/// Create the service tree map with the given namespace
pub fn create(ns: &Namespace) -> ServiceTreeMap<'_> {
    let map = Cell::new(BTreeMap::new());
    populate(&map, &ns);
    map.take()
}

/// Recursively populate the service tree map with the given namespace
fn populate<'a, 'b>(src: &'b Cell<ServiceTreeMap<'a>>, ns: &'a Namespace) {
    let mut map = src.take();

    for service in ns.services.values() {
        for (name, rpc) in service.methods.iter() {
            let (segments, last_segment, url) = match HTTPOptions::from(&rpc.md.options) {
                Some(HTTPOptions { method, path, .. }) => (
                    path.split('/')
                        .skip(1)
                        .map(|seg| match seg.starts_with(':') {
                            true => Cow::from("*"),
                            false => Cow::from(seg.to_string()),
                        })
                        .collect::<Vec<_>>(),
                    Cow::from(method.to_lowercase()),
                    path,
                ),
                None => {
                    let segments = vec![Cow::from(ns.path.join(".")), name.into()];
                    let url = format!("/{}", segments.join("/"));
                    (segments, Cow::from("grpc"), Cow::from(url))
                }
            };

            let mut ptr = &mut map;

            for path in segments {
                ptr = ptr
                    .entry(path)
                    .or_insert_with(|| ServiceMapNode::Branch(BTreeMap::new()))
                    .unwrap_as_branch();
            }

            ptr.insert(last_segment, ServiceMapNode::Leaf { rpc, url });
        }
    }

    src.set(map);
    for child in ns.nested.values() {
        populate(src, child)
    }
}

#[cfg(test)]
mod tests {
    use crate::{parser::test_util::parse_test_file, service_map::no_leading_dot};
    use indoc::indoc;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_no_leading_dot() {
        assert_eq!(no_leading_dot(".pb.foo.Bar"), "pb.foo.Bar")
    }

    #[test]
    fn test_generate_service_tree_map() {
        let ns = parse_test_file(indoc! {r#"
        package pb.hello;
        
        service HelloWorld {
          rpc LotsOfGreetings(stream SayHelloRequest) returns (SayHelloResponse) {}
          rpc SayHello (SayHelloRequest) returns (SayHelloResponse) { option (pgm.http.rule) = { GET: "/hello/<string:name>" }; }
        }
        
        message SayHelloRequest {}        
        message SayHelloResponse {}
        "#});

        let map = super::create(&ns);
        let output = serde_json::to_string_pretty(&map).unwrap();

        let result = indoc! {r#"
          {
            "hello": {
              "*": {
                "get": [
                  "pb.hello.SayHelloRequest",
                  "pb.hello.SayHelloResponse",
                  "/hello/:name"
                ]
              }
            },
            "pb.hello": {
              "LotsOfGreetings": {
                "grpc": [
                  "pb.hello.SayHelloRequest",
                  "pb.hello.SayHelloResponse",
                  "/pb.hello/LotsOfGreetings"
                ]
              }
            }
          }"#};

        assert_eq!(output, result);
    }
}
