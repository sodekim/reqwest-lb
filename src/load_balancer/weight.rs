use std::fmt::{Debug, Display};

#[derive(Debug, Clone)]
pub struct Weighted<T> {
    pub element: T,
    pub weight: usize,
}

impl<T> Weighted<T> {
    pub fn new(element: T, weight: usize) -> Self {
        Self { element, weight }
    }
}

impl<T: Display> Display for Weighted<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}(weight = {})", self.element, self.weight)
    }
}

impl<T> std::ops::Deref for Weighted<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.element
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
