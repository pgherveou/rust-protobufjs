/// An Iterator wrapper that end with a last value
#[derive(Clone)]
pub struct EndWithIterator<I: Iterator> {
    iter: I,
    last: Option<I::Item>,
}

impl<I> Iterator for EndWithIterator<I>
where
    I: Iterator,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().or_else(|| self.last.take())
    }
}

/// An Iterator wrapper that start with an initial value
#[derive(Clone)]
pub struct StartWithIterator<I: Iterator> {
    iter: I,
    first: Option<I::Item>,
}

impl<I> Iterator for StartWithIterator<I>
where
    I: Iterator,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.first.take().or_else(|| self.iter.next())
    }
}

/// An Iterator blanket implementation that provides extra adaptors and
/// methods.
pub trait IterExt: Iterator {
    /// Start the iterator with the specified value
    fn start_with<T>(self, first: T) -> StartWithIterator<Self>
    where
        Self: Sized,
        T: Into<Self::Item>,
    {
        StartWithIterator {
            iter: self,
            first: Some(first.into()),
        }
    }

    /// End the iterator with the specified value
    fn end_with<T>(self, last: T) -> EndWithIterator<Self>
    where
        Self: Sized,
        T: Into<Self::Item>,
    {
        EndWithIterator {
            iter: self,
            last: Some(last.into()),
        }
    }

    /// compute the path relative to
    fn relative_to<'a, 'b, T>(mut self, mut dest: T) -> Self
    where
        Self: Sized + Clone,
        Self: Iterator<Item = &'a str>,
        T: Iterator<Item = &'b str>,
    {
        let mut src = self.clone();

        // // get the first object segment
        if let Some(first_segment) = src.next() {
            // find the position of the first segment in the destination
            if dest.any(|segment| segment == first_segment) {
                self.next();
                // iterate as long as src and destination segments match
                loop {
                    match (src.next(), dest.next()) {
                        (Some(s1), Some(s2)) if s1 == s2 => {
                            self.next();
                        }
                        _ => break,
                    }
                }
            }
        }

        self
    }
}

impl<T> IterExt for T where T: Iterator {}

#[cfg(test)]
mod tests {
    use crate::iter_ext::IterExt;

    #[test]
    fn test_start_with() {
        let iter = vec!["2", "3", "4"].into_iter().start_with("1");
        assert_eq!(iter.collect::<Vec<_>>(), vec!["1", "2", "3", "4"]);
    }

    #[test]
    fn test_end_with() {
        let iter = vec!["1", "2", "3"].into_iter().end_with("4");
        assert_eq!(iter.collect::<Vec<_>>(), vec!["1", "2", "3", "4"]);
    }

    fn test_relative_path(obj: &str, from: &str, expected: &str) {
        let result = obj
            .split('.')
            .relative_to(from.split('.'))
            .collect::<Vec<&str>>()
            .join(".");

        assert_eq!(result, expected);
    }

    #[test]
    fn test_relative_path_from_fully_qualified_type() {
        test_relative_path("pb.example.Request", "pb.example", "Request");
    }

    #[test]
    fn test_relative_path_from_partial_qualified_type() {
        test_relative_path("example.Request", "pb.example", "Request");
    }

    #[test]
    fn test_relative_path_from_unqualified_type() {
        test_relative_path("Request", "pb.example", "Request");
    }

    #[test]
    fn test_relative_path_from_different_namespace() {
        test_relative_path("example.Request", "pb.other", "example.Request");
    }
}
