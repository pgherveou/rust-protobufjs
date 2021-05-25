use std::{path::Path, rc::Rc};

use crate::comment::Comment;

type ProtoOption = Vec<String>;

#[derive(Debug)]
pub struct Metadata {
    /// a list of options associated with this method
    pub options: Vec<ProtoOption>,

    // the path relative to the proto root folder
    pub file_path: Rc<Path>,

    /// leading comment extracted from the source proto file
    pub comment: Option<Comment>,

    /// Line where this object is defined in the source proto file
    pub line: usize,
}

impl Metadata {
    pub fn new(file_path: Rc<Path>, comment: Option<Comment>, line: usize) -> Self {
        Self {
            options: Vec::new(),
            file_path,
            comment,
            line,
        }
    }

    pub fn add_option(&mut self, option: ProtoOption) {
        self.options.push(option);
    }
}
