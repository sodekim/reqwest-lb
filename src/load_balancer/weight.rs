pub trait WeightProvider<I>: sealed::Sealed<I> {
    fn weight(&self, item: &I) -> usize;
}

impl<I, F> sealed::Sealed<I> for F where F: Fn(&I) -> usize {}

impl<I, F> WeightProvider<I> for F
where
    F: Fn(&I) -> usize,
{
    fn weight(&self, item: &I) -> usize {
        self(item)
    }
}

mod sealed {
    pub trait Sealed<I> {}
}
