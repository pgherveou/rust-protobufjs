use serde::Serialize;
use std::{cell::RefCell, collections::HashMap};

use crate::metadata::Metadata;

/// utility function used by serde skip_serializing_if directive
/// is_false is used to remove false boolean from the serialized output
fn is_false(value: &bool) -> bool {
    !(*value)
}

/// Defines a rpc service
/// [service] https://developers.google.com/protocol-buffers/docs/proto3#services
#[derive(Debug, Serialize)]
pub struct Service {
    /// The list of rpc methods defined by this service
    pub methods: HashMap<String, Rpc>,

    /// metadata associated to the Enum
    #[serde(skip_serializing)]
    pub md: Metadata,
}

impl Service {
    /// Add a new rpc method
    pub fn add_rpc(&mut self, name: String, rpc: Rpc) {
        self.methods.insert(name, rpc);
    }

    // Returns a new Service with the provided metadata
    pub fn new(md: Metadata) -> Self {
        Self {
            methods: HashMap::new(),
            md,
        }
    }
}

/// Rpc defines a [rpc] method of a Service
/// [rpc] https://developers.google.com/protocol-buffers/docs/proto3#services
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Rpc {
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

    /// metadata associated to the Enum
    #[serde(skip_serializing)]
    pub md: Metadata,
}

impl Rpc {
    /// Returns a new rpc method
    pub fn new(
        request_type: String,
        request_stream: bool,
        response_type: String,
        response_stream: bool,
        md: Metadata,
    ) -> Self {
        Self {
            request_type: RefCell::new(request_type),
            request_stream,
            response_type: RefCell::new(response_type),
            response_stream,
            md,
        }
    }
}
