#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Weighted<T> {
    pub element: T,
    pub weight: usize,
}

impl<T> Weighted<T> {
    pub fn new(element: T, weight: usize) -> Self {
        Self { element, weight }
    }
}

impl<T> std::ops::Deref for Weighted<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.element
    }
}

impl<T: PartialEq> PartialEq<T> for Weighted<T> {
    fn eq(&self, other: &T) -> bool {
        &self.element == other
    }
}

pub trait IntoWeighted<T> {
    fn into_weighted(self) -> Weighted<T>;
}

impl<T> IntoWeighted<T> for T {
    fn into_weighted(self) -> Weighted<T> {
        Weighted::new(self, 1)
    }
}

impl<T> IntoWeighted<T> for (T, usize) {
    fn into_weighted(self) -> Weighted<T> {
        Weighted::new(self.0, self.1)
    }
}
