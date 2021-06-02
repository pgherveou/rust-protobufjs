//! Parse a set of .proto files into a namespace struct.
//! 
//! A [Namespace](crate::namespace::Namespace) is a loose translation of [FileDescriptorSet]. 
//! It's the main reflection object used by the [protobuf.js] library.
//! 
//! Although the [protobuf.js] library comes with it's own parser, 
//! It fails to parse a large number of files in a relatively short time.
//! 
//! The goal of this library is to parse our growing set of proto files very quickly, 
//! and generate IDL derived files that can be consumed by our Typescript codebase.
//! 
//! These 3 files are:
//! 
//! ## descriptors
//! 
//! The parsed proto files that we load with [protobuf.js] to encode and decode proto object.
//! See [crate::parser::Parser] for more details
//! 
//! ## service-map  
//! 
//! A map of the rpc services, used to quickly resolve request and response types for our APIs.
//! See [crate::service_map] for more details
//! 
//! ## Typescript definition file
//! 
//! Typescript definition are used to provide type hint and type checking.
//! See [crate::typescript] for more details
//! 
//! 
//! [FileDescriptorSet]: https://github.com/protocolbuffers/protobuf/blob/master/src/google/protobuf/descriptor.proto#L57 
//! [protobuf.js]: https://github.com/protobufjs/protobuf.js


extern crate lazy_static;

mod comment;
mod r#enum;
mod field;
mod file_parser;
mod http_options;
mod import;
mod into_path;
mod iter_ext;
mod iterator_with_position;
mod message;
mod metadata;
pub mod namespace;
mod oneof;
mod parse_error;
pub mod parser;
mod position;
mod scalar;
mod service;
pub mod service_map;
mod token;
mod tokenizer;
mod r#type;
pub mod typescript;
