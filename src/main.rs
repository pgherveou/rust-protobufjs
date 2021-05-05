use glob::glob;
use protobuf::parser::Parser;
use std::{collections::HashSet, path::PathBuf};

fn main() {
    match run() {
        Err(err) => println!("{}", err),
        Ok(()) => println!("Ok"),
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let root_dir = PathBuf::from("/Users/pgherveou/src/idl/protos");

    let mut ignored_files = HashSet::new();
    ignored_files.insert(root_dir.join("validate/validate.proto"));
    ignored_files.insert(root_dir.join("google/rpc/status.proto"));
    ignored_files.insert(root_dir.join("google/api/annotations.proto"));
    ignored_files.insert(root_dir.join("google/api/expr/v1alpha1/syntax.proto"));

    let mut parser = Parser::new(root_dir, ignored_files);

    let entries = glob("/Users/pgherveou/src/idl/protos/pb/**/*.proto")?;

    for entry in entries {
        let file_name = entry?;
        parser.parse_file(file_name)?;
    }

    let json = serde_json::to_string_pretty(&parser.root).unwrap();
    let output_file = "/tmp/rust-bubble-pb.json";
    std::fs::write(output_file, json)?;
    println!("wrote {}", output_file);
    Ok(())
}
