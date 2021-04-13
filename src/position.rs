#[derive(Clone, Debug, PartialEq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
    pub offset: usize,
}

impl Position {
    pub fn add_line(&mut self) {
        self.offset += 1;
        self.line += 1;
        self.column = 1;
    }

    pub fn remove_line(&mut self) {
        self.offset -= 1;
        self.line -= 1;
        self.column = 1;
    }

    pub fn add_column(&mut self) {
        self.offset += 1;
        self.column += 1;
    }

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
