use serde::Serialize;
use std::collections::HashMap;

/// utility function used by serde skip_serializing_if directive
/// is_false is used to remove false boolean from the serialized output
fn is_false(value: &bool) -> bool {
    *value == false
}

/// Defines a rpc Service
/// See https://developers.google.com/protocol-buffers/docs/proto3#services
#[derive(Debug, Serialize)]
pub struct Service {
    /// The list of rpc methods defined by this service
    methods: HashMap<String, Rpc>,
}

impl Service {
    /// Returns a new service
    pub fn new() -> Service {
        Self {
            methods: HashMap::new(),
        }
    }

    /// Add a new rpc method
    pub fn add_rpc(&mut self, name: String, rpc: Rpc) {
        self.methods.insert(name, rpc);
    }
}

/// Rpc defines a rpc method of a Service
/// See https://developers.google.com/protocol-buffers/docs/proto3#services
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Rpc {
    // #[serde(skip_serializing, skip_deserializing)]
    // name: String,
    /// The rpc request type
    request_type: String,

    /// Define whether the rpc request is streaming or not
    #[serde(skip_serializing_if = "is_false")]
    request_stream: bool,

    /// The rpc response type
    response_type: String,

    /// Define whether the rpc response is streaming or not
    #[serde(skip_serializing_if = "is_false")]
    response_stream: bool,
}

impl Rpc {
    /// Returns a new rpc method
    pub fn new(
        request_type: String,
        request_stream: bool,
        response_type: String,
        response_stream: bool,
    ) -> Self {
        Self {
            // name,
            request_type,
            request_stream,
            response_type,
            response_stream,
        }
    }
}
