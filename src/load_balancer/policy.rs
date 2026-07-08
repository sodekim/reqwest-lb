use crate::load_balancer::{Statistic, Weighted};
use http::Extensions;
use rand::RngExt;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

#[derive(Default)]
pub enum LoadBalancerPolicy<I> {
    #[default]
    RoundRobin,
    Random,
    First,
    Last,
    Dynamic(Arc<dyn LoadBalancerPolicyTrait<I> + Send + Sync>),
}

impl<I> Debug for LoadBalancerPolicy<I> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadBalancerPolicy::RoundRobin => f.write_str("RoundRobin"),
            LoadBalancerPolicy::Random => f.write_str("Random"),
            LoadBalancerPolicy::First => f.write_str("First"),
            LoadBalancerPolicy::Last => f.write_str("Last"),
            LoadBalancerPolicy::Dynamic(_) => f.write_str("Dynamic(f)"),
        }
    }
}

impl<I> Clone for LoadBalancerPolicy<I> {
    fn clone(&self) -> Self {
        match self {
            LoadBalancerPolicy::RoundRobin => LoadBalancerPolicy::RoundRobin,
            LoadBalancerPolicy::Random => LoadBalancerPolicy::Random,
            LoadBalancerPolicy::First => LoadBalancerPolicy::First,
            LoadBalancerPolicy::Last => LoadBalancerPolicy::Last,
            LoadBalancerPolicy::Dynamic(f) => LoadBalancerPolicy::Dynamic(f.clone()),
        }
    }
}

impl<I> LoadBalancerPolicy<I> {
    pub fn dynamic<F: Fn(&[Weighted<I>], &mut Extensions) -> usize + Send + Sync + 'static>(
        f: F,
    ) -> Self {
        Self::Dynamic(Arc::new(f))
    }
}

pub trait LoadBalancerPolicyTrait<I>: sealed::Sealed<I> {
    fn choose(&self, elements: &[Weighted<I>], extensions: &mut Extensions) -> usize;
}

impl<I> sealed::Sealed<I> for LoadBalancerPolicy<I> {}

impl<I> LoadBalancerPolicyTrait<I> for LoadBalancerPolicy<I> {
    fn choose(&self, elements: &[Weighted<I>], extensions: &mut Extensions) -> usize {
        match self {
            LoadBalancerPolicy::Dynamic(f) => f.choose(elements, extensions),
            policy => {
                let len = elements.iter().map(|element| element.weight).sum::<usize>();
                let index = match policy {
                    LoadBalancerPolicy::RoundRobin => extensions
                        .get::<Statistic>()
                        .map(|Statistic { count }| (*count as usize) % len)
                        .unwrap_or(0),
                    LoadBalancerPolicy::Random => rand::rng().random_range(0..len),
                    LoadBalancerPolicy::First => 0,
                    LoadBalancerPolicy::Last => len.saturating_sub(1),
                    LoadBalancerPolicy::Dynamic(_) => unreachable!(),
                };
                LoadBalancerPolicy::index_of(index, elements)
            }
        }
    }
}

impl<I> LoadBalancerPolicy<I> {
    fn index_of(index: usize, elements: &[Weighted<I>]) -> usize {
        let mut remaining = index;
        for (index, element) in elements.iter().enumerate() {
            if remaining < element.weight {
                return index;
            }
            remaining -= element.weight;
        }
        elements.len().saturating_sub(1)
    }
}

impl<I, F> sealed::Sealed<I> for F where F: Fn(&[Weighted<I>], &mut Extensions) -> usize {}

impl<I, F> LoadBalancerPolicyTrait<I> for F
where
    F: Fn(&[Weighted<I>], &mut Extensions) -> usize,
{
    fn choose(&self, elements: &[Weighted<I>], extensions: &mut Extensions) -> usize {
        self(elements, extensions)
    }
}

mod sealed {
    pub trait Sealed<I> {}
}
