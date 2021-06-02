/// Comment represents a [proto comment]
///
/// [proto comment]: https://developers.google.com/protocol-buffers/docs/proto#adding_comments
#[derive(Debug, PartialEq)]
pub enum CommentKind {
    StarSlash,
    DoubleSlash,
}

#[derive(Debug, PartialEq)]
pub struct Comment {
    pub kind: CommentKind,
    pub text: String,
    pub start_line: usize,
    pub end_line: usize,
}

impl Comment {
    pub fn star_slash(text: String, start_line: usize, end_line: usize) -> Self {
        Self {
            kind: CommentKind::StarSlash,
            text,
            start_line,
            end_line,
        }
    }
    pub fn double_slash(text: String, start_line: usize, end_line: usize) -> Self {
        Self {
            kind: CommentKind::DoubleSlash,
            text,
            start_line,
            end_line,
        }
    }
}
