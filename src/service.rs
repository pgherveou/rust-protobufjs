use serde::Serialize;
use std::{cell::RefCell, collections::HashMap};

/// utility function used by serde skip_serializing_if directive
/// is_false is used to remove false boolean from the serialized output
fn is_false(value: &bool) -> bool {
    !(*value)
}

/// Defines a rpc service
/// [service] https://developers.google.com/protocol-buffers/docs/proto3#services
#[derive(Debug, Default, Serialize)]
pub struct Service {
    /// The list of rpc methods defined by this service
    pub methods: HashMap<String, Rpc>,
}

impl Service {
    /// Add a new rpc method
    pub fn add_rpc(&mut self, name: String, rpc: Rpc) {
        self.methods.insert(name, rpc);
    }
}

/// Rpc defines a [rpc] method of a Service
/// [rpc] https://developers.google.com/protocol-buffers/docs/proto3#services
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Rpc {
    // #[serde(skip_serializing, skip_deserializing)]
    // name: String,
    /// The rpc request type
    pub request_type: RefCell<String>,

    /// Define whether the rpc request is streaming or not
    #[serde(skip_serializing_if = "is_false")]
    pub request_stream: bool,

    /// The rpc response type
    pub response_type: RefCell<String>,

    /// Define whether the rpc response is streaming or not
    #[serde(skip_serializing_if = "is_false")]
    pub response_stream: bool,

    // a list of options associated with this method
    pub options: Vec<Vec<String>>,
}

impl Rpc {
    /// Returns a new rpc method
    pub fn new(
        request_type: String,
        request_stream: bool,
        response_type: String,
        response_stream: bool,
        options: Vec<Vec<String>>,
    ) -> Self {
        Self {
            request_type: RefCell::new(request_type),
            request_stream,
            response_type: RefCell::new(response_type),
            response_stream,
            options,
        }
    }
}
