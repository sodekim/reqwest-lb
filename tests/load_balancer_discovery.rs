use http::Extensions;
use reqwest::Url;
use reqwest_lb::discovery::Change;
use reqwest_lb::supplier::discovery::DiscoverySupplier;
use reqwest_lb::supplier::Supplier;
use reqwest_lb::{LoadBalancer, LoadBalancerPolicy, SimpleLoadBalancer};

#[tokio::test]
async fn laod_balancer_discovery() {
    let ports = (0..10).map(|offset| 3000 + offset).collect::<Vec<_>>();
    let mut events = ports
        .iter()
        .map(|port| {
            Url::parse(&format!("http://127.0.0.1:{}", port)).map(|url| Change::Insert(*port, url))
        })
        .collect::<Vec<_>>();
    events.push(Ok(Change::Initialized));

    let discovery = futures::stream::iter(events);
    let supplier = DiscoverySupplier::new(discovery);
    let elements = supplier.get().await.unwrap();
    println!("{:?}", elements);
    let load_balancer = SimpleLoadBalancer::new(supplier, LoadBalancerPolicy::RoundRobin);
    let mut extensions = Extensions::new();
    for port in ports {
        let selected = load_balancer.choose(&mut extensions).await;
        assert_eq!(
            selected,
            Ok(Some(
                Url::parse(&format!("http://127.0.0.1:{}", port)).unwrap()
            ))
        );
    }
}
