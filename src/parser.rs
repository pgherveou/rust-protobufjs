use crate::{
    file_parser::FileParser, import::Import, namespace::Namespace, parse_error::ParseFileError,
};
use std::{collections::HashMap, path::PathBuf};

/// The parser parse files and populate the root namespace
pub struct Parser {
    /// The root directory used to resolve import statements
    root_dir: PathBuf,

    /// List of parsed files
    parsed_files: HashMap<PathBuf, Namespace>,
}

impl Parser {
    /// Returns a new parser with the given root directory and a list of files we want to ignore    
    pub fn new(root_dir: PathBuf, parsed_files: HashMap<PathBuf, Namespace>) -> Self {
        Self {
            root_dir,
            parsed_files,
        }
    }

    /// Parse the given file, and it's import dependencies
    /// The result will be merged into the root namespace of the parser
    pub fn parse_file(&mut self, file_name: PathBuf) -> Result<(), ParseFileError> {
        if self.parsed_files.contains_key(&file_name) {
            return Ok(());
        }

        let content = match std::fs::read_to_string(&file_name) {
            Ok(r) => r,
            Err(error) => return Err(ParseFileError::Read(file_name.clone(), error)),
        };

        // create the parser
        let file_parser = FileParser::new(file_name.clone(), content.chars());

        // parse the namespace
        let ns = file_parser.parse(&content)?;

        // get the list of imported files and parse them
        for import in ns.imports.iter() {
            let file_name = self.root_dir.join(import.as_str());
            self.parse_file(file_name)?;
        }

        self.parsed_files.insert(file_name, ns);
        return Ok(());
    }

    /// Build the namespace graph by consuming all the parsed files
    pub fn build_root(self) -> Result<Namespace, ParseFileError> {
        // normalize all files
        for (path, namespace) in self.parsed_files.iter() {
            let dependencies = self.get_dependencies(namespace);

            namespace
                .resolve_types(dependencies)
                .map_err(|err| err.to_parse_file_error(path.into()))?;
        }

        // build the namespace tree
        let mut root = Namespace::empty();
        for child in self.parsed_files.into_values() {
            root.append_child(child)
        }

        return Ok(root);
    }

    fn get_dependencies(&self, namespace: &Namespace) -> Vec<&Namespace> {
        namespace
            .imports
            .iter()
            .flat_map(|f| {
                let file_path = self.root_dir.join(f.as_str());
                let ns = &self.parsed_files[&file_path];
                let mut vec = vec![ns];
                vec.append(&mut self.get_transitive_dependencies(ns));
                return vec;
            })
            .collect()
    }

    fn get_transitive_dependencies(&self, namespace: &Namespace) -> Vec<&Namespace> {
        namespace
            .imports
            .iter()
            .flat_map(|f| match f {
                Import::Public(path) => {
                    let file_path = self.root_dir.join(path);
                    let ns = &self.parsed_files[&file_path];
                    let mut vec = vec![ns];
                    vec.append(&mut self.get_transitive_dependencies(ns));
                    return vec;
                }
                Import::Internal(_) => Vec::new(),
            })
            .collect()
    }
}
