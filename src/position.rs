/// Defines a position in a file
#[derive(Clone, Debug, PartialEq)]
pub struct Position {
    /// the line index starting at 1
    pub line: usize,

    /// the column index starting at 1
    pub column: usize,

    /// the characte offset starting at 0
    pub offset: usize,
}

impl Position {
    /// Increment the line number by 1
    pub fn add_line(&mut self) {
        self.offset += 1;
        self.line += 1;
        self.column = 1;
    }

    /// Decrement the line number by 1
    pub fn remove_line(&mut self) {
        self.offset -= 1;
        self.line -= 1;
        self.column = 1;
    }

    /// Increment the column number by 1
    pub fn add_column(&mut self) {
        self.offset += 1;
        self.column += 1;
    }

    /// Decrement the column number by 1
    pub fn remove_column(&mut self) {
        self.offset -= 1;
        self.column -= 1;
    }
}

impl Default for Position {
    fn default() -> Self {
        Self {
            line: 1,
            column: 1,
            offset: 0,
        }
    }
}
