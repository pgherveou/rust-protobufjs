use glob::glob;
use protobuf::service_map;
use protobuf::typescript::serializer::{PrintConfig, Printer};
use protobuf::{namespace::Namespace, parser::Parser};
use std::array::IntoIter;
use std::cell::Cell;
use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Instant;

fn main() {
    let home = dirs::home_dir().unwrap();
    let root_dir = home.join("src/idl/protos");

    // match parse(root_dir.into(), "**/*.proto") {
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
    let output_file = "/Users/pgherveou/.bbl/descriptors.json";
    std::fs::write(output_file, output)?;
    println!("wrote {}", output_file);

    let config = PrintConfig {
        root_url: "https://github.com/lyft/idl/blob/master/protos".into(),
        print_bubble_client: true,
        print_network_client: true,
    };

    let printer = Printer::new(&config);
    let output = printer.into_string(&root);
    let output_file = "/Users/pgherveou/.bbl/routes.d.ts";
    std::fs::write(output_file, output)?;
    println!("wrote {}", output_file);

    let map = Cell::new(BTreeMap::new());
    service_map::build(&map, &root);
    let map = map.take();

    let output = serde_json::to_string_pretty(&map).unwrap();
    let output_file = "/Users/pgherveou/.bbl/service-map.json";
    std::fs::write(output_file, output)?;
    println!("wrote {}", output_file);

    Ok(root)
}
