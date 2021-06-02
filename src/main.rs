use globwalk::GlobWalkerBuilder;
use prosecco::service_map;
use prosecco::typescript::serializer::{PrintConfig, Printer};
use prosecco::{namespace::Namespace, parser::Parser};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Instant;

fn main() {
    let root_dir = dirs::home_dir().unwrap().join("src/idl/protos");
    let patterns = ["**/*.proto", "!pb/envoy"];

    match parse(root_dir, &patterns) {
        Err(err) => println!("{}", err),
        Ok(_) => println!("Ok"),
    }
}

fn get_files<'a, 'b>(
    root_dir: &'a Path,
    patterns: &'b [&'b str],
) -> impl Iterator<Item = Rc<Path>> + 'a {
    GlobWalkerBuilder::from_patterns(&root_dir, patterns)
        .build()
        .unwrap()
        .into_iter()
        .filter_map(move |entry| {
            let path = entry.ok();
            let path = path?.into_path();
            let path = path.strip_prefix(&root_dir).ok()?;
            Some(Rc::<Path>::from(path))
        })
}

fn parse(root_dir: PathBuf, patterns: &[&str]) -> Result<Namespace, Box<dyn std::error::Error>> {
    let start = Instant::now();

    let mut parser = Parser::new(root_dir.clone());
    parser.ignore_files(&["validate/validate.proto"]);

    let files = get_files(&root_dir, patterns);
    for file_path in files {
        parser.parse_file(file_path)?;
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

    let map = service_map::create(&root);
    let output = serde_json::to_string_pretty(&map).unwrap();
    let output_file = "/Users/pgherveou/.bbl/service-map.json";
    std::fs::write(output_file, output)?;
    println!("wrote {}", output_file);

    Ok(root)
}
