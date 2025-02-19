use crate::lb::weight::WeightProvider;
use crate::lb::Statistic;
use crate::with::With;
use http::Extensions;
use rand::Rng;
use std::fmt::{Debug, Formatter};
use std::sync::atomic::Ordering;
use std::sync::Arc;

#[derive(Default)]
pub enum LoadBalancerPolicy<I> {
    #[default]
    RoundRobin,
    Random,
    First,
    Last,
    Weight(Arc<dyn WeightProvider<I> + Send + Sync>),
    Dynamic(Arc<dyn LoadBalancerPolicyTrait<I> + Send + Sync>),
}

impl<I> Debug for LoadBalancerPolicy<I> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadBalancerPolicy::RoundRobin => f.write_str("RoundRobin"),
            LoadBalancerPolicy::Random => f.write_str("Random"),
            LoadBalancerPolicy::First => f.write_str("First"),
            LoadBalancerPolicy::Last => f.write_str("Last"),
            LoadBalancerPolicy::Weight(_) => f.write_str("Weight(f)"),
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
            LoadBalancerPolicy::Weight(f) => LoadBalancerPolicy::Weight(f.clone()),
            LoadBalancerPolicy::Dynamic(f) => LoadBalancerPolicy::Dynamic(f.clone()),
        }
    }
}

impl<I> LoadBalancerPolicy<I> {
    pub fn weight<F: Fn(&I) -> usize + Send + Sync + 'static>(f: F) -> Self {
        Self::Weight(Arc::new(f))
    }

    pub fn dynamic<F: Fn(&[I], &Extensions) -> usize + Send + Sync + 'static>(f: F) -> Self {
        Self::Dynamic(Arc::new(f))
    }
}

pub trait LoadBalancerPolicyTrait<I>: sealed::Sealed<I> {
    fn choose(&self, items: &[I], extensions: &mut Extensions) -> usize;
}

impl<I> sealed::Sealed<I> for LoadBalancerPolicy<I> {}

impl<I> LoadBalancerPolicyTrait<I> for LoadBalancerPolicy<I> {
    fn choose(&self, items: &[I], extensions: &mut Extensions) -> usize {
        let len = items.len();
        assert!(len > 1);
        match self {
            LoadBalancerPolicy::RoundRobin => match extensions.get::<Statistic>() {
                Some(statistic) => {
                    let count = statistic.count.load(Ordering::Relaxed).saturating_sub(1);
                    (count % (len as u64)) as usize
                }
                None => 0,
            },
            LoadBalancerPolicy::Random => rand::thread_rng().gen_range(0..len),
            LoadBalancerPolicy::First => 0,
            LoadBalancerPolicy::Last => items.len() - 1,
            LoadBalancerPolicy::Weight(f) => {
                let indexes = items
                    .iter()
                    .enumerate()
                    .map(|(index, item)| (index, f.weight(item)))
                    .flat_map(|(index, len)| {
                        Vec::with_capacity(len).with(|c| {
                            for _ in 0..len {
                                c.push(index);
                            }
                        })
                    })
                    .collect::<Vec<_>>();
                let index = rand::thread_rng().gen_range(0..indexes.len());
                indexes[index]
            }
            LoadBalancerPolicy::Dynamic(f) => f.choose(items, extensions),
        }
    }
}

impl<I, F> sealed::Sealed<I> for F where F: Fn(&[I], &Extensions) -> usize {}

impl<I, F> LoadBalancerPolicyTrait<I> for F
where
    F: Fn(&[I], &Extensions) -> usize,
{
    fn choose(&self, items: &[I], extensions: &mut Extensions) -> usize {
        self(items, extensions)
    }
}

mod sealed {
    pub trait Sealed<I> {}
}
