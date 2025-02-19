use crate::lb::{LoadBalancerPolicy, LoadBalancerPolicyTrait, Statistic};
use crate::supplier::Supplier;
use crate::LoadBalancerTrait;
use http::Extensions;
use pin_project_lite::pin_project;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::Ordering;
use std::task::{ready, Context, Poll};

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
