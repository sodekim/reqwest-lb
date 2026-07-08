mod policy;
mod registry;
mod weight;

pub use policy::{LoadBalancerPolicy, LoadBalancerPolicyTrait};
pub use registry::LoadBalancerRegistry;
pub use weight::{IntoWeighted, Weighted};

use crate::supplier::Supplier;
use async_trait::async_trait;
use http::Extensions;
use std::marker::PhantomData;
use std::sync::atomic::AtomicU64;
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
    pub count: u64,
}

pub struct LoadBalancer<S: Supplier, T> {
    supplier: S,
    policy: LoadBalancerPolicy<T>,
    count: AtomicU64,
    marker: PhantomData<T>,
}

impl<S, T> LoadBalancer<S, T>
where
    S: Supplier,
    S::Element: IntoWeighted<T>,
{
    pub fn new(supplier: S, policy: LoadBalancerPolicy<T>) -> Self {
        Self {
            supplier,
            policy,
            count: AtomicU64::new(0),
            marker: PhantomData,
        }
    }
}

#[async_trait]
impl<S, T> LoadBalancerTrait for LoadBalancer<S, T>
where
    S: Supplier + Sync,
    S::Future: Send,
    S::Element: IntoWeighted<T> + Send,
    T: Send + Sync,
{
    type Element = T;
    type Error = S::Error;

    async fn choose(
        &self,
        extensions: &mut Extensions,
    ) -> Result<Option<Self::Element>, Self::Error> {
        // increment count
        let count = self.count.fetch_add(1, Ordering::Relaxed);
        extensions.insert(Statistic { count });
        let mut elements = self
            .supplier
            .get()
            .await?
            .into_iter()
            .map(IntoWeighted::into_weighted)
            .collect::<Vec<Weighted<T>>>();
        match elements.len() {
            0 => Ok(None),
            1 => Ok(Some(elements.remove(0).element)),
            _ => {
                let index = self.policy.choose(&elements, extensions);
                Ok(Some(elements.remove(index).element))
            }
        }
    }
}
