use reqwest::{Client, Url};
use reqwest_lb::supplier::LoadBalancer;
use reqwest_lb::{LoadBalancerMiddleware, LoadBalancerPolicy, LoadBalancerRegistry};
use reqwest_middleware::ClientBuilder;
use reqwest_retry::policies::ExponentialBackoff;
use reqwest_retry::{Jitter, RetryTransientMiddleware};
use std::time::Duration;

#[tokio::test]
async fn with_retry() {
    // load balancer middleware
    let mut factory = LoadBalancerRegistry::default();
    let urls = vec![
        // error
        Url::parse("https://www.rust-lang-error.org").unwrap(),
        // ok
        Url::parse("https://www.rust-lang.org").unwrap(),
    ];
    let load_balancer = LoadBalancer::new(urls, LoadBalancerPolicy::RoundRobin);
    factory.add("rust-server", load_balancer);
    let load_balancer_middleware = LoadBalancerMiddleware::new(factory);

    // We create a ExponentialBackoff retry policy which implements `RetryPolicy`.
    let retry_policy = ExponentialBackoff::builder()
        .retry_bounds(Duration::from_secs(1), Duration::from_secs(60))
        .jitter(Jitter::Bounded)
        .base(2)
        .build_with_total_retry_duration(Duration::from_secs(24 * 60 * 60));

    let retry_transient_middleware = RetryTransientMiddleware::new_with_policy(retry_policy);
    let client = ClientBuilder::new(Client::new())
        .with(retry_transient_middleware)
        .with(load_balancer_middleware)
        .build();

    let response = client.get("lb://rust-server").send().await;
    assert!(response.is_ok());
}
