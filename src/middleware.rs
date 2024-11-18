use crate::lb::LoadBalancerFactory;
use crate::BoxError;
use async_trait::async_trait;
use http::Extensions;
use reqwest::{Request, Response, Url};
use reqwest_middleware::{Middleware, Next};
use std::convert::Infallible;
use std::fmt::Debug;
use thiserror::Error;
use tracing::debug;

fn is_lb_schema(schema: &str) -> bool {
    match (schema.get(0..1), schema.get(1..2)) {
        (Some(a), Some(b)) => (a == "l" || a == "L") && (b == "b" || b == "B"),
        _ => false,
    }
}

pub struct LoadBalancerMiddleware<I, E = Infallible> {
    factory: LoadBalancerFactory<I, E>,
}

impl<I, E> LoadBalancerMiddleware<I, E> {
    pub fn new(factory: LoadBalancerFactory<I, E>) -> Self {
        Self { factory }
    }
}

#[async_trait]
impl<I, E> Middleware for LoadBalancerMiddleware<I, E>
where
    I: Into<Url> + 'static,
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
            let load_balancer = self.factory.get(host).ok_or(Error::NotFoundLoadBalancer)?;
            let item = load_balancer
                .choose(extensions)
                .await
                .map_err(|e| Error::Customize(e.into()))?
                .ok_or(Error::NoSuchElement)?;
            let old_url = request.url();
            let mut new_url = item.into();
            reconstruct(old_url, &mut new_url);
            debug!("reconstruct new url: {}", new_url.as_str());
            *request.url_mut() = new_url;
        }
        next.run(request, extensions).await
    }
}

fn reconstruct(old_url: &Url, new_url: &mut Url) {
    new_url.set_path(old_url.path());
    new_url.set_query(old_url.query());
    new_url.set_fragment(old_url.fragment());
}

#[derive(Debug, Error)]
enum Error {
    #[error("Not found load balancer")]
    NotFoundLoadBalancer,

    #[error("Load balancer not found element")]
    NoSuchElement,

    #[error("Request miss host")]
    MissHost,

    #[error("{0}")]
    Customize(BoxError),
}

impl From<Error> for reqwest_middleware::Error {
    fn from(value: Error) -> Self {
        Self::middleware(value)
    }
}
