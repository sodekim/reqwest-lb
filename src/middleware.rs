use crate::lb::LoadBalancerRegistry;
use crate::BoxError;
use async_trait::async_trait;
use http::Extensions;
use reqwest::{Request, Response, Url};
use reqwest_middleware::{Middleware, Next};
use std::fmt::Debug;
use thiserror::Error;
use tracing::debug;

fn is_lb_schema(schema: &str) -> bool {
    match (schema.get(0..1), schema.get(1..2)) {
        (Some(a), Some(b)) => (a == "l" || a == "L") && (b == "b" || b == "B"),
        _ => false,
    }
}

pub struct LoadBalancerMiddleware<I, E> {
    registry: LoadBalancerRegistry<I, E>,
}

impl<I, E> LoadBalancerMiddleware<I, E> {
    pub fn new(registry: LoadBalancerRegistry<I, E>) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl<I, E, IE> Middleware for LoadBalancerMiddleware<I, E>
where
    I: TryInto<Url, Error = IE> + 'static,
    IE: Into<BoxError> + 'static,
    E: Into<BoxError> + 'static,
{
    async fn handle(
        &self,
        mut request: Request,
        extensions: &mut Extensions,
        next: Next<'_>,
    ) -> reqwest_middleware::Result<Response> {
        let schema = request.url().scheme();
        if is_lb_schema(schema) {
            let host = request.url().host_str().ok_or(Error::MissHost)?;
            let load_balancer = self
                .registry
                .find(host)
                .ok_or(Error::NotFoundLoadBalancer)?;
            let item = load_balancer
                .choose(extensions)
                .await
                .map_err(|e| Error::Customize(e.into()))?
                .ok_or(Error::NotFoundElement)?;
            let source = request.url();
            let mut target = item.try_into().map_err(|e| Error::InvalidUrl(e.into()))?;
            reconstruct(source, &mut target);
            debug!("reconstruct new url: {}", target.as_str());
            *request.url_mut() = target;
        }
        next.run(request, extensions).await
    }
}

fn reconstruct(source: &Url, target: &mut Url) {
    target.set_path(source.path());
    target.set_query(source.query());
    target.set_fragment(source.fragment());
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Invalid url: {0}")]
    InvalidUrl(BoxError),

    #[error("Registry not found load balancer")]
    NotFoundLoadBalancer,

    #[error("Load balancer not found element")]
    NotFoundElement,

    #[error("Request miss host")]
    MissHost,

    #[error("{0}")]
    Customize(BoxError),
}

impl Error {
    pub fn customize<E: Into<BoxError>>(error: E) -> Self {
        Self::Customize(error.into())
    }
}

impl From<Error> for reqwest_middleware::Error {
    fn from(value: Error) -> Self {
        Self::middleware(value)
    }
}
