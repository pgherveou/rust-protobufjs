use crate::{
    file_parser::FileParser, import::Import, namespace::Namespace, parse_error::ParseFileError,
};
use std::{collections::HashMap, path::Path, rc::Rc};

/// The parser parse files and populate the root namespace
pub struct Parser {
    /// The root directory used to resolve import statements
    root_dir: Rc<Path>,

    /// List of parsed files
    pub parsed_files: HashMap<Rc<Path>, Namespace>,
}

impl Parser {
    /// Returns a new parser with the given root directory and a list of files we want to ignore    
    pub fn new(root_dir: Rc<Path>, parsed_files: HashMap<Rc<Path>, Namespace>) -> Self {
        Self {
            root_dir,
            parsed_files,
        }
    }

    /// Parse the given file, and it's import dependencies
    /// The result will be merged into the root namespace of the parser
    pub fn parse_file(&mut self, file_path: Rc<Path>) -> Result<(), ParseFileError> {
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
            self.parse_file(import.as_path().into())?;
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
