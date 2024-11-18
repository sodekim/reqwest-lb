use reqwest::{Client, Url};
use reqwest_lb::{
    LoadBalancerFactory, LoadBalancerMiddleware, LoadBalancerPolicy, SimpleLoadBalancer,
};
use reqwest_middleware::ClientBuilder;
use reqwest_retry::policies::ExponentialBackoff;
use reqwest_retry::RetryTransientMiddleware;

#[tokio::test]
async fn with_retry() {
    // load balancer middleware
    let mut factory = LoadBalancerFactory::default();
    let urls = vec![
        // error
        Url::parse("https://www.rust-lang-error.org").unwrap(),
        // ok
        Url::parse("https://www.rust-lang.org").unwrap(),
    ];
    let load_balancer = SimpleLoadBalancer::new(urls, LoadBalancerPolicy::RoundRobin);
    factory.add("rust-server", load_balancer);
    let load_balancer_middleware = LoadBalancerMiddleware::new(factory);

    // retry middleware
    let retry_middleware = RetryTransientMiddleware::new_with_policy(
        ExponentialBackoff::builder().build_with_max_retries(1),
    );

    // reqwest client
    // https://www.rust-lang-error.org => retry error (choose next) => https://www.rust-lang.org
    let client = ClientBuilder::new(Client::default())
        .with(retry_middleware)
        .with(load_balancer_middleware)
        .build();

    let response = client.get("lb://rust-server").send().await;
    assert!(response.is_ok());
}
