use std::collections::HashMap;

use serde::Serialize;

fn is_false(value: &bool) -> bool {
    *value == false
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Rpc {
    #[serde(skip_serializing, skip_deserializing)]
    name: String,

    request_type: String,

    #[serde(skip_serializing_if = "is_false")]
    request_stream: bool,

    response_type: String,

    #[serde(skip_serializing_if = "is_false")]
    response_stream: bool,
}

impl Rpc {
    pub fn new(
        name: String,
        request_type: String,
        request_stream: bool,
        response_type: String,
        response_stream: bool,
    ) -> Self {
        Self {
            name,
            request_type,
            request_stream,
            response_type,
            response_stream,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Service {
    #[serde(skip_serializing, skip_deserializing)]
    pub name: String,

    pub methods: HashMap<String, Rpc>,
}

impl Service {
    pub fn new(name: String) -> Service {
        Self {
            name,
            methods: HashMap::new(),
        }
    }

    pub fn add_rpc(&mut self, rpc: Rpc) {
        self.methods.insert(rpc.name.to_string(), rpc);
    }
}
