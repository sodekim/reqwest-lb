mod policy;
mod registry;
mod weight;

pub use policy::{LoadBalancerPolicy, LoadBalancerPolicyTrait};
pub use registry::LoadBalancerRegistry;
pub use weight::WeightProvider;

use crate::supplier::Supplier;
use futures::future::BoxFuture;
use futures::ready;
use http::Extensions;
use pin_project_lite::pin_project;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::{fmt::Debug, sync::atomic::Ordering};

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

impl<L> LoadBalancerTrait for MapFuture<L>
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

impl<S> LoadBalancerTrait for LoadBalancer<S>
where
    S: Supplier,
{
    type Element = S::Element;
    type Error = S::Error;
    type Future = ChooseFuture<S::Element, S::Future>;

    fn choose(&self, extensions: &mut Extensions) -> Self::Future {
        // touch statistic
        self.statistic.count.fetch_add(1, Ordering::SeqCst);
        extensions.insert(self.statistic.clone());
        let extensions = extensions.clone();
        let future = self.supplier.get();
        let policy = self.policy.clone();
        ChooseFuture {
            extensions,
            policy,
            future,
        }
    }
}

pin_project! {
    pub struct ChooseFuture<I, F> {
        extensions: Extensions,
        policy: LoadBalancerPolicy<I>,
        #[pin]
        future: F,
    }
}

impl<I, E, F> Future for ChooseFuture<I, F>
where
    F: Future<Output = Result<Vec<I>, E>>,
{
    type Output = Result<Option<I>, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let project = self.project();
        match ready!(project.future.poll(cx)) {
            Ok(mut elements) => {
                let size = elements.len();
                Poll::Ready(match size {
                    0 => Ok(None),
                    1 => Ok(Some(elements.remove(0))),
                    _ => {
                        // use policy choose and return the index
                        let index = project.policy.choose(&elements, project.extensions);
                        Ok(Some(elements.remove(index)))
                    }
                })
            }
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}
