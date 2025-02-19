use http::Extensions;
use reqwest::Url;
use reqwest_lb::discovery::Change;
use reqwest_lb::supplier::{DiscoverySupplier, LoadBalancer};
use reqwest_lb::{LoadBalancerPolicy, LoadBalancerTrait};

#[tokio::test]
async fn load_balancer_discovery() {
    let ports = (0..10).map(|offset| 3000 + offset).collect::<Vec<_>>();
    let mut events = ports
        .iter()
        .map(|port| {
            Url::parse(&format!("http://127.0.0.1:{}", port)).map(|url| Change::Insert(*port, url))
        })
        .collect::<Vec<_>>();

    // NOTICE: the initialized event notify load balancer all elements already insert
    events.push(Ok(Change::Initialized));

    let supplier = DiscoverySupplier::new(futures::stream::iter(events));
    let load_balancer = LoadBalancer::new(supplier, LoadBalancerPolicy::RoundRobin);
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
