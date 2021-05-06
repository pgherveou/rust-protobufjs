use glob::glob;
use protobuf::parser::Parser;
use std::collections::HashSet;

fn main() {
    match run() {
        Err(err) => println!("{}", err),
        Ok(()) => println!("Ok"),
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let home = dirs::home_dir().unwrap();
    let root_dir = home.join("src/idl/protos");

    let mut ignored_files = HashSet::new();
    ignored_files.insert(root_dir.join("validate/validate.proto"));
    ignored_files.insert(root_dir.join("google/rpc/status.proto"));
    ignored_files.insert(root_dir.join("google/api/annotations.proto"));
    ignored_files.insert(root_dir.join("google/api/expr/v1alpha1/syntax.proto"));

    let pattern = root_dir.join("pb/**/*.proto");
    let entries = glob(pattern.to_string_lossy().as_ref())?;

    let mut parser = Parser::new(root_dir, ignored_files);
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
