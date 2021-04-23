use glob::glob;

use protobuf::parser::Parser;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // for entry in glob(
    //     "/Users/pgherveou/src/idl/protos/pb/api/endpoints/legacy/user_update/user_update.proto",
    // )
    // .expect("Failed to read glob pattern")

    let entries =
        glob("/Users/pgherveou/src/idl/protos/pb/**/*.proto").expect("Failed to read glob pattern");

    for (index, entry) in entries.enumerate() {
        let path = entry?;
        let file_name = path.to_str().unwrap();
        println!("{} parsing  {}", index, file_name);
        let content = std::fs::read_to_string(file_name)?;
        let mut parser = Parser::new(file_name, &content);
        serde_json::to_string(&parser.root).unwrap();
        match parser.parse() {
            Ok(_) => {}
            Err(err) => {
                println!("{}", err);
                break;
            }
        }
    }

    Ok(())
}

// fn test_serde_json_serializer(root: &Box<Namespace>) -> String {
//     serde_json::to_string(root).unwrap()
// }
