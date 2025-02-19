pub mod iter;
mod discovery;
mod lb;

pub use discovery::*;
pub use lb::*;

use std::future::Future;

pub trait Supplier {
    ///
    /// Supplier contains element type
    ///
    type Element;

    ///
    /// Supplier get element maybe error type
    ///
    type Error;

    ///
    /// Get all element future type
    ///
    type Future: Future<Output = Result<Vec<Self::Element>, Self::Error>>;

    ///
    /// Get current all elements
    ///
    fn get(&self) -> Self::Future;
}
