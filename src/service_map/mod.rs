use crate::{http_options::HTTPOptions, namespace::Namespace, service::Rpc};
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Serialize, Serializer};
use std::{borrow::Cow, cell::Cell, collections::BTreeMap};

type ServiceMap<'a> = BTreeMap<Cow<'a, str>, ServiceMapOrType<'a>>;

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum ServiceMapOrType<'a> {
    ServiceMap(ServiceMap<'a>),

    #[serde(serialize_with = "serialize_service")]
    ServiceType {
        rpc: &'a Rpc,
        url: Cow<'a, str>,
    },
}

fn no_leading_dot(s: &str) -> &str {
    if s.starts_with(".") {
        &s[1..]
    } else {
        s
    }
}

fn serialize_service<S>(rpc: &Rpc, url: &str, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let req = rpc.request_type.borrow();
    let req = req.as_str();

    let resp = rpc.response_type.borrow();
    let resp = resp.as_str();

    [no_leading_dot(req), no_leading_dot(resp), url].serialize(serializer)
}

impl<'a> ServiceMapOrType<'a> {
    fn unwrap_as_map(&mut self) -> &mut ServiceMap<'a> {
        match self {
            Self::ServiceMap(v) => v,
            Self::ServiceType { rpc: _, url: _ } => panic!("unexpected service type"),
        }
    }
}

pub fn build<'a>(src: &'a Cell<ServiceMap<'a>>, ns: &'a Namespace) {
    let mut map = src.take();

    for service in ns.services.values() {
        for (name, rpc) in service.methods.iter() {
            let (segments, last_segment, url) = match HTTPOptions::from(&rpc.md.options) {
                Some(HTTPOptions { method, path, .. }) => {
                    lazy_static! {
                        static ref HTTP_REGEX: Regex = Regex::new("(<.*?:(.*?)>)").unwrap();
                    }

                    (
                        path.split('/')
                            .skip(1)
                            .map(|path| match path.starts_with('<') {
                                true => "*",
                                false => path,
                            })
                            .collect::<Vec<_>>(),
                        Cow::from(method.to_lowercase()),
                        HTTP_REGEX.replace_all(path, ":$2"),
                    )
                }
                None => (
                    ns.path.iter().map(|v| v.as_str()).collect::<Vec<_>>(),
                    Cow::from(name.as_str()),
                    format!("/{}/{}", ns.path.join("."), name).into(),
                ),
            };

            let mut ptr = &mut map;

            for path in segments {
                ptr = ptr
                    .entry(path.into())
                    .or_insert(ServiceMapOrType::ServiceMap(BTreeMap::new()))
                    .unwrap_as_map();
            }

            ptr.insert(last_segment, ServiceMapOrType::ServiceType { rpc, url });
        }
    }

    src.set(map);
    for child in ns.nested.values() {
        build(src, child)
    }
}
