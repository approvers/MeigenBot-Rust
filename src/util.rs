pub trait IteratorEditExt<T> {
    fn edit<F>(self, f: F) -> Self
    where
        F: FnOnce(&mut T);
}

impl<T, E> IteratorEditExt<T> for std::result::Result<T, E> {
    fn edit<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&mut T),
    {
        if let Ok(ref mut v) = self {
            f(v);
        }

        self
    }
}

impl<T, E> IteratorEditExt<T> for std::result::Result<Option<T>, E> {
    fn edit<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&mut T),
    {
        if let Ok(Some(ref mut v)) = self {
            f(v);
        }

        self
    }
}
