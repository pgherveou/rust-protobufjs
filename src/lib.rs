#![feature(iter_advance_by)]
#![feature(bool_to_option)]
#![feature(iter_intersperse)]
#![feature(map_try_insert)]

mod r#enum;
mod field;
mod file_parser;
mod import;
mod into_path;
mod iter_ext;
mod iterator_with_position;
pub mod message;
pub mod namespace;
mod oneof;
pub mod parse_error;
pub mod parser;
mod position;
mod scalar;
mod service;
mod token;
mod tokenizer;
pub mod ts_serializer;
mod r#type;
