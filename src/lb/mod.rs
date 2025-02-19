mod registry;
mod policy;
mod weight;

use futures::future::BoxFuture;
use http::Extensions;
use std::fmt::Debug;
use std::future::Future;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

pub use registry::LoadBalancerRegistry;
pub use policy::{LoadBalancerPolicy, LoadBalancerPolicyTrait};
pub use weight::WeightProvider;

pub type BoxLoadBalancer<I, E> = Box<
    dyn LoadBalancerTrait<Element = I, Error = E, Future = BoxFuture<'static, Result<Option<I>, E>>>
        + Send
        + Sync,
>;

pub trait LoadBalancerTrait {
    ///
    /// load balancer element type
    ///
    type Element;

    ///
    /// load balancer choose element maybe error type
    ///
    type Error;

    ///
    /// load balancer choose element future type
    ///
    type Future: Future<Output = Result<Option<Self::Element>, Self::Error>>;

    ///
    /// load balancer choose a effect element
    ///
    fn choose(&self, extensions: &mut Extensions) -> Self::Future;

    ///
    /// Wrap to boxed load balancer
    ///
    fn boxed(self) -> BoxLoadBalancer<Self::Element, Self::Error>
    where
        Self: Sized + Send + Sync + 'static,
        Self::Future: Send + 'static,
    {
        Box::new(BoxFutureLoadBalancer::new(self))
    }
}

struct BoxFutureLoadBalancer<L> {
    inner: L,
}

impl<L> BoxFutureLoadBalancer<L> {
    pub fn new(inner: L) -> Self {
        Self { inner }
    }
}

impl<L> LoadBalancerTrait for BoxFutureLoadBalancer<L>
where
    L: LoadBalancerTrait,
    L::Future: Send + 'static,
{
    type Element = L::Element;
    type Error = L::Error;
    type Future = BoxFuture<'static, Result<Option<Self::Element>, Self::Error>>;

    fn choose(&self, extensions: &mut Extensions) -> Self::Future {
        Box::pin(self.inner.choose(extensions))
    }
}

#[derive(Debug, Clone, Default)]
pub struct Statistic {
    pub count: Arc<AtomicU64>,
}
