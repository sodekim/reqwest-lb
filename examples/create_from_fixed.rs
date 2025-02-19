#![allow(unused)]

use reqwest::{Client, Url};
use reqwest_lb::supplier::LoadBalancer;
use reqwest_lb::LoadBalancerMiddleware;
use reqwest_lb::LoadBalancerPolicy;
use reqwest_lb::LoadBalancerRegistry;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};

pub fn create<const N: usize>(
    ports: [u16; N],
    policy: LoadBalancerPolicy<Url>,
) -> ClientWithMiddleware {
    // create load balancer factory
    let mut registry = LoadBalancerRegistry::default();

    // create url list
    let mut urls = Vec::with_capacity(N);
    for port in ports {
        urls.push(Url::parse(format!("http://127.0.0.1:{}", port).as_str()).unwrap());
    }

    // create load balancer
    let load_balancer = LoadBalancer::new(urls, policy);

    // register load balancer for the host: example-server
    registry.add("example-server", load_balancer);

    // create reqwest client
    let middleware = LoadBalancerMiddleware::new(registry);
    ClientBuilder::new(Client::builder().no_proxy().build().unwrap())
        .with(middleware)
        .build()
}

#[tokio::main]
async fn main() {
    // use round robin policy
    let client = create([3001, 3002], LoadBalancerPolicy::RoundRobin);
    // http://127.0.0.1:3001
    let response = client.get("lb://example-server/").send().await.unwrap();
    // http://127.0.0.1:3002
    let response = client.get("lb://example-server/").send().await.unwrap();
}


