use glob::glob;

use protobuf::parse_error::ParseError;
use protobuf::parser::Parser;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new();

    let entries = glob("/Users/pgherveou/src/idl/protos/pb/lyft/lastmile/**/*.proto")
        .expect("Failed to read glob pattern");
    // let entries =
    //     glob("/Users/pgherveou/src/idl/protos/pb/**/*.proto").expect("Failed to read glob pattern");

    for entry in entries {
        let path = entry?;
        let file_name = path.to_str().unwrap();

        match parser.parse_file(file_name) {
            Ok(_) => {}

            Err(err) => match err.error {
                ParseError::ProtoSyntaxNotSupported(_) => {
                    println!("skip {} {}", file_name, err);
                }
                _ => {
                    println!("{}", err);
                    break;
                }
            },
        }
    }

    let json = serde_json::to_string_pretty(&parser.root).unwrap();
    let output_file = "/tmp/rust-bubble-pb.json";
    std::fs::write(output_file, json)?;
    println!("wrote {}", output_file);
    Ok(())
}

// fn test_serde_json_serializer(root: &Box<Namespace>) -> String {
//     serde_json::to_string(root).unwrap()
// }
