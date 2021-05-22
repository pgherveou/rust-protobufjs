use glob::glob;
use protobuf::{namespace::Namespace, parser::Parser, ts_serializer};
use std::{array::IntoIter, collections::HashMap};
use std::{iter::FromIterator, path::PathBuf};

fn main() {
    let home = dirs::home_dir().unwrap();
    // let root_dir = home.join("src/rust-protobufjs/protos");
    let root_dir = home.join("src/idl/protos");

    match parse(root_dir, "pb/lyft/hello/hello_world.proto") {
        Err(err) => println!("{}", err),
        Ok(_) => println!("Ok"),
    }
}

#[allow(dead_code)]
fn parse(root_dir: PathBuf, pattern: &str) -> Result<Namespace, Box<dyn std::error::Error>> {
    let ignored_files = HashMap::from_iter(
        IntoIter::new([
            root_dir.join("validate/validate.proto"),
            root_dir.join("google/rpc/status.proto"),
            root_dir.join("google/api/annotations.proto"),
            root_dir.join("google/api/expr/v1alpha1/syntax.proto"),
        ])
        .map(|file| (file, Namespace::empty())),
    );

    let pattern = root_dir.join(pattern);
    let entries = glob(pattern.to_string_lossy().as_ref())?;

    let mut parser = Parser::new(root_dir, ignored_files);
    for entry in entries {
        let file_name = entry?;
        parser.parse_file(file_name)?;
    }

    let root = parser.build_root()?;

    let output = serde_json::to_string_pretty(&root).unwrap();
    let output_file = "/tmp/descriptors.json";
    std::fs::write(output_file, output)?;
    println!("wrote {}", output_file);

    let printer = ts_serializer::Printer::new();
    let output_file = "/tmp/router.d.ts";
    let output = printer.to_string(&root);
    std::fs::write(output_file, output)?;
    println!("wrote {}", output_file);

    Ok(root)
}
