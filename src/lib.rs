#![feature(iter_advance_by)]
#![feature(bool_to_option)]
#![feature(iter_intersperse)]
#![feature(map_try_insert)]

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
pub mod message;
mod metadata;
pub mod namespace;
mod oneof;
pub mod parse_error;
pub mod parser;
mod position;
mod scalar;
mod service;
pub mod service_map;
mod token;
mod tokenizer;
mod r#type;
pub mod typescript;
