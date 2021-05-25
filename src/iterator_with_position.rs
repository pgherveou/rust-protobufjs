use crate::position::Position;

/// A Peekable iterator that keeps track of the current position
pub struct IteratorWithPosition<I: Iterator> {
    /// The underlying iterator
    iter: I,

    // The current position
    position: Position,

    // Peeked iterator item if any
    peeked: Option<Option<I::Item>>,
}

impl<I: Iterator<Item = char>> IteratorWithPosition<I> {
    /// Returns a new IteratorWithPosition
    pub fn new(iter: I) -> Self {
        Self {
            iter,
            position: Position::default(),
            peeked: None,
        }
    }

    /// Returns the next iterator item if the given closure returns true.
    pub fn next_if(&mut self, func: impl FnOnce(&I::Item) -> bool) -> Option<I::Item> {
        match self.next() {
            Some(matched) if func(&matched) => Some(matched),
            other => {
                self.peeked = Some(other);
                None
            }
        }
    }

    /// Returns a copy of the current position
    pub fn current_position(&self) -> Position {
        let mut position = self.position.clone();
        if let Some(Some(c)) = self.peeked {
            match c {
                '\n' => position.remove_line(),
                _ => position.remove_column(),
            }
        }

        position
    }

    /// Returns the current line
    pub fn current_line(&self) -> usize {
        match self.peeked {
            Some(Some('\n')) => self.position.line - 1,
            _ => self.position.line,
        }
    }
}

impl<I: Iterator<Item = char>> Iterator for IteratorWithPosition<I> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(v) = self.peeked.take() {
            return v;
        }

        self.iter.next().map(|c| {
            match c {
                '\n' => self.position.add_line(),
                _ => self.position.add_column(),
            }
            c
        })
    }
}
