use crate::position::Position;

pub struct IteratorWithPosition<I: Iterator> {
    iter: I,
    position: Position,
    peeked: Option<Option<I::Item>>,
}

impl<I: Iterator<Item = char>> IteratorWithPosition<I> {
    pub fn new(iter: I) -> Self {
        Self {
            iter,
            position: Position::default(),
            peeked: None,
        }
    }

    pub fn next_if(&mut self, func: impl FnOnce(&I::Item) -> bool) -> Option<I::Item> {
        match self.next() {
            Some(matched) if func(&matched) => Some(matched),
            other => {
                self.peeked = Some(other);
                None
            }
        }
    }

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
