use crate::supplier::Supplier;
use std::convert::Infallible;
use std::future::{ready, Ready};

impl<T: IntoIterator + Clone> Supplier for T {
	type Element = T::Item;
	type Error = Infallible;
	type Future = Ready<Result<Vec<Self::Element>, Self::Error>>;

	fn get(&self) -> Self::Future {
		let elements = self.clone();
		ready(Ok(elements.into_iter().collect()))
	}
}
