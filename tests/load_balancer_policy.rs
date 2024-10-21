use http::Extensions;
use reqwest_lb::{LoadBalancer, LoadBalancerPolicy, SimpleLoadBalancer};

const ITEMS: [usize; 10] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];

async fn choose<F>(policy: LoadBalancerPolicy<usize>, f: F)
where
    F: Fn(usize, usize) -> bool,
{
    let load_balancer = SimpleLoadBalancer::new(ITEMS, policy);
    let mut extensions = Extensions::new();
    for expect in ITEMS {
        let selected = load_balancer.choose(&mut extensions).await;
        assert!(matches!(selected, Ok(Some(selected)) if f(expect,selected)));
    }
}

#[tokio::test]
async fn round_robin() {
    choose(LoadBalancerPolicy::RoundRobin, |expect, selected| {
        expect == selected
    })
        .await;
}

#[tokio::test]
async fn random() {
    choose(LoadBalancerPolicy::Random, |_, selected| {
        ITEMS.contains(&selected)
    })
        .await;
}

#[tokio::test]
async fn first() {
    choose(LoadBalancerPolicy::First, |_, selected| {
        selected == ITEMS[0]
    })
        .await;
}

#[tokio::test]
async fn last() {
    choose(LoadBalancerPolicy::Last, |_, selected| {
        selected == ITEMS[ITEMS.len() - 1]
    })
        .await;
}

#[tokio::test]
async fn weight() {
    choose(LoadBalancerPolicy::weight(|i| *i), |_, selected| {
        ITEMS.contains(&selected)
    })
        .await;
}

#[tokio::test]
async fn dynamic() {
    choose(LoadBalancerPolicy::dynamic(|_, _| 0), |_, selected| {
        selected == 0
    })
        .await;
}
