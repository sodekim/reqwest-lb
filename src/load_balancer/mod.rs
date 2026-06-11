mod policy;
mod registry;
mod weight;

pub use policy::{LoadBalancerPolicy, LoadBalancerPolicyTrait};
pub use registry::LoadBalancerRegistry;
pub use weight::WeightProvider;

use crate::supplier::Supplier;
use async_trait::async_trait;
use http::Extensions;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::{fmt::Debug, sync::atomic::Ordering};

pub type BoxLoadBalancer<I, E> = Box<dyn LoadBalancerTrait<Element = I, Error = E> + Send + Sync>;

#[async_trait]
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
    /// load balancer choose a effect element
    ///
    async fn choose(
        &self,
        extensions: &mut Extensions,
    ) -> Result<Option<Self::Element>, Self::Error>;

    ///
    /// Wrap to boxed load balancer
    ///
    fn boxed(self) -> BoxLoadBalancer<Self::Element, Self::Error>
    where
        Self: Sized + Send + Sync + 'static,
    {
        Box::new(MapFuture::new(self))
    }
}

struct MapFuture<L> {
    inner: L,
}

impl<L> MapFuture<L> {
    pub fn new(inner: L) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl<L> LoadBalancerTrait for MapFuture<L>
where
    L: LoadBalancerTrait + Sync,
{
    type Element = L::Element;
    type Error = L::Error;

    async fn choose(
        &self,
        extensions: &mut Extensions,
    ) -> Result<Option<Self::Element>, Self::Error> {
        self.inner.choose(extensions).await
    }
}

#[derive(Debug, Clone, Default)]
pub struct Statistic {
    pub count: Arc<AtomicU64>,
}

pub struct LoadBalancer<S: Supplier> {
    supplier: S,
    policy: LoadBalancerPolicy<S::Element>,
    statistic: Statistic,
}

impl<S: Supplier> LoadBalancer<S> {
    pub fn new(supplier: S, policy: LoadBalancerPolicy<S::Element>) -> Self {
        Self {
            supplier,
            policy,
            statistic: Statistic::default(),
        }
    }
}

#[async_trait]
impl<S> LoadBalancerTrait for LoadBalancer<S>
where
    S: Supplier + Sync,
    S::Future: Send,
{
    type Element = S::Element;
    type Error = S::Error;

    async fn choose(
        &self,
        extensions: &mut Extensions,
    ) -> Result<Option<Self::Element>, Self::Error> {
        // touch statistic
        self.statistic.count.fetch_add(1, Ordering::SeqCst);
        extensions.insert(self.statistic.clone());
        let mut elements = self.supplier.get().await?;
        let size = elements.len();
        match size {
            0 => Ok(None),
            1 => Ok(Some(elements.remove(0))),
            _ => {
                // use policy choose and return the index
                let index = self.policy.choose(&elements, extensions);
                Ok(Some(elements.remove(index)))
            }
        }
    }
}
