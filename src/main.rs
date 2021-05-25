use glob::glob;
use protobuf::{namespace::Namespace, parser::Parser, ts_serializer};
use std::array::IntoIter;
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Instant;

fn main() {
    let home = dirs::home_dir().unwrap();
    // let root_dir = home.join("src/rust-protobufjs/protos");
    let root_dir = home.join("src/idl/protos");

    // match parse(root_dir.into(), "pb/lyft/hello/hello_world.proto") {
    match parse(root_dir.into(), "**/*.proto") {
        Err(err) => println!("{}", err),
        Ok(_) => println!("Ok"),
    }
}

#[allow(dead_code)]
fn parse(root_dir: Rc<Path>, pattern: &str) -> Result<Namespace, Box<dyn std::error::Error>> {
    let start = Instant::now();

    let ignored_files = IntoIter::new([
        "validate/validate.proto",
        "google/rpc/status.proto",
        "google/api/annotations.proto",
        "google/api/expr/v1alpha1/syntax.proto",
    ])
    .map(|file| {
        let path: PathBuf = file.into();
        (Rc::from(path.as_path()), Namespace::default())
    })
    .collect();

    let pattern = root_dir.join(pattern);
    let entries = glob(pattern.to_string_lossy().as_ref())?;

    let mut parser = Parser::new(root_dir.clone(), ignored_files);
    for entry in entries {
        let file_path = entry?;
        let file_path = file_path.strip_prefix(root_dir.as_ref()).unwrap();
        parser.parse_file(file_path.into())?;
    }

    println!(
        "Parsed {} files in {:?}",
        parser.parsed_files.len(),
        start.elapsed()
    );

    let root = parser.build_root()?;

    let output = serde_json::to_string_pretty(&root).unwrap();
    let output_file = "/tmp/descriptors.json";
    std::fs::write(output_file, output)?;
    println!("wrote {}", output_file);

    let mut printer = ts_serializer::Printer::default();
    printer.print_bubble_client = true;
    printer.print_network_client = true;

    let output = printer.into_string(&root);
    let output_file = "/tmp/router.d.ts";
    std::fs::write(output_file, output)?;
    println!("wrote {}", output_file);

    Ok(root)
}
