use std::future::Future;

pub trait Runtime {
    fn spawn<F>(future: F)
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Tokio;

impl Runtime for Tokio {
    fn spawn<F>(future: F)
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        tokio::spawn(future);
    }
}
