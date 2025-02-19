pub(crate) trait With: Sized {
    fn with<F>(mut self, mut f: F) -> Self
    where
        F: FnMut(&mut Self),
    {
        f(&mut self);
        self
    }
}

impl<T> With for T {}
