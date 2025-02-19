#![allow(unused)]

use reqwest::{Client, Url};
use reqwest_lb::discovery::Change;
use reqwest_lb::supplier::DiscoverySupplier;
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
    let mut events = ports
        .iter()
        .map(|port| {
            Url::parse(&format!("http://127.0.0.1:{}", port)).map(|url| Change::Insert(*port, url))
        })
        .collect::<Vec<_>>();

    // NOTICE: the initialized event notify load balancer all elements already insert
    events.push(Ok(Change::Initialized));

    let supplier = DiscoverySupplier::new(futures::stream::iter(events));
    let load_balancer = LoadBalancer::new(supplier, policy);

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
