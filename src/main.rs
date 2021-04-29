use glob::glob;

use protobuf::parse_error::ParseError;
use protobuf::parser::Parser;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new();

    let entries = glob("/Users/pgherveou/src/idl/protos/pb/lyft/hello/**/*.proto")
        .expect("Failed to read glob pattern");
    // let entries =
    //     glob("/Users/pgherveou/src/idl/protos/pb/**/*.proto").expect("Failed to read glob pattern");

    for (index, entry) in entries.enumerate() {
        let path = entry?;
        let file_name = path.to_str().unwrap();
        println!("{} parsing  {}", index, file_name);
        let content = std::fs::read_to_string(file_name)?;

        match parser.parse_file(file_name, &content) {
            Ok(_) => {}

            Err(err) => match err.error {
                ParseError::ProtoSyntaxNotSupported(_) => {
                    println!("skip {}", err);
                }
                _ => {
                    println!("{}", err);
                    break;
                }
            },
        }
    }

    let json = serde_json::to_string_pretty(&parser.root).unwrap();
    let output_file = "/tmp/descriptors.json";
    std::fs::write(output_file, json)?;
    println!("wrote {}", output_file);
    Ok(())
}

// fn test_serde_json_serializer(root: &Box<Namespace>) -> String {
//     serde_json::to_string(root).unwrap()
// }
