use crate::{
    file_parser::FileParser, import::Import, namespace::Namespace, parse_error::ParseFileError,
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    rc::Rc,
};

/// The parser parse files and populate the root namespace
///
/// # Example:
///
/// Basic usage:
///
/// ```no_run
/// # use std::path::Path;
/// # use prosecco::parser::Parser;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // root dir containing the proto files
/// let root_dir = Path::new("protos");
///
/// // create a new parser
/// let mut parser = Parser::new(root_dir);
///
/// // parse one or more files.
/// // Imports will be resolved and parsed relatively to the root_dir
/// parser.parse_file(Path::new("pb/hello/hello_world.json"))?;
///
/// // build the root namespace.
/// let root = parser.build_root()?;
///
/// // generate descriptors
/// let output = serde_json::to_string_pretty(&root).unwrap();
/// std::fs::write(Path::new("descriptors.json"), output)?;
/// # Ok(())
/// # }
/// ```
pub struct Parser {
    /// The root directory used to resolve import statements
    root_dir: PathBuf,

    /// List of parsed files
    pub parsed_files: HashMap<Rc<Path>, Namespace>,
}

impl Parser {
    /// Returns a new parser with the given root directory and a list of files we want to ignore    
    pub fn new<T: Into<PathBuf>>(root_dir: T) -> Self {
        Self {
            root_dir: root_dir.into(),
            parsed_files: HashMap::new(),
        }
    }

    pub fn ignore_files(&mut self, files: &[&str]) {
        for file in files {
            let path = PathBuf::from(file);
            self.parsed_files
                .insert(Rc::from(path.as_path()), Namespace::default());
        }
    }

    /// Parse the given file, and it's import dependencies
    /// The result will be merged into the root namespace of the parser
    pub fn parse_file<T: Into<Rc<Path>>>(&mut self, file_path: T) -> Result<(), ParseFileError> {
        let file_path = file_path.into();

        if self.parsed_files.contains_key(&file_path) {
            return Ok(());
        }

        let path = self.root_dir.join(file_path.as_ref());
        let content = match std::fs::read_to_string(&path) {
            Ok(r) => r,
            Err(error) => return Err(ParseFileError::Read(path, error)),
        };

        // create the parser
        let file_parser = FileParser::new(file_path.clone(), content.chars());

        // parse the namespace
        let ns = file_parser
            .parse()
            .map_err(|error| error.into_file_error(path, content.as_str()))?;

        // get the list of imported files and parse them
        for import in ns.imports.iter() {
            self.parse_file(import.as_path())?;
        }

        self.parsed_files.insert(file_path, ns);
        Ok(())
    }

    /// Build the namespace graph by consuming all the parsed files
    pub fn build_root(self) -> Result<Namespace, ParseFileError> {
        // normalize all files
        for (path, namespace) in self.parsed_files.iter() {
            let dependencies = self.get_dependencies(namespace);

            namespace
                .resolve_types(dependencies)
                .map_err(|err| err.into_parse_file_error(self.root_dir.join(path.as_ref())))?;
        }

        // build the namespace tree
        let mut root = Namespace::default();
        for child in self.parsed_files.into_values() {
            root.append_child(child)
        }

        Ok(root)
    }

    fn get_dependencies(&self, namespace: &Namespace) -> Vec<&Namespace> {
        namespace
            .imports
            .iter()
            .flat_map(|import| {
                let ns = &self.parsed_files[import.as_path()];
                let mut vec = vec![ns];
                vec.append(&mut self.get_transitive_dependencies(ns));
                vec
            })
            .collect()
    }

    fn get_transitive_dependencies(&self, namespace: &Namespace) -> Vec<&Namespace> {
        namespace
            .imports
            .iter()
            .flat_map(|f| match f {
                Import::Public(path) => {
                    let ns = &self.parsed_files[path.as_path()];
                    let mut vec = vec![ns];
                    vec.append(&mut self.get_transitive_dependencies(ns));
                    vec
                }
                Import::Internal(_) => Vec::new(),
            })
            .collect()
    }
}

#[cfg(test)]
pub mod test_util {
    use crate::{file_parser::FileParser, namespace::Namespace, parser::Parser};
    use std::{
        path::{Path, PathBuf},
        rc::Rc,
    };

    pub fn parse_test_file(text: &'static str) -> Namespace {
        let file_path: PathBuf = "test.proto".into();
        let file_path: Rc<Path> = file_path.into();
        let file_parser = FileParser::new(file_path.clone(), text.chars());

        let ns = file_parser
            .parse()
            .expect("parse test.proto without errors");

        let root_dir: PathBuf = ".".into();
        let mut parser = Parser::new(root_dir);
        parser.parsed_files.insert(file_path.into(), ns);

        parser
            .build_root()
            .expect("create root namespace without errors")
    }
}

#[cfg(test)]
mod tests {
    use super::Parser;
    use pretty_assertions::assert_eq;
    use std::path::PathBuf;

    #[test]
    fn test_serialize_root() {
        let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("protos");
        let expected_output = std::fs::read_to_string(&root_dir.join("descriptors.json"))
            .expect("descriptors.json should exist");

        let mut parser = Parser::new(root_dir);

        parser
            .parse_file(PathBuf::from("foo.proto").into())
            .expect("it should parse one.proto");

        let root = parser.build_root().expect("it should build root");
        let output = serde_json::to_string_pretty(&root).unwrap();

        assert_eq!(output, expected_output)
    }
}
