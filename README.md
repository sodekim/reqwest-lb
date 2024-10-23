# reqwest-lb

[![Crates.io][crates-badge]][crates-url]
[![Apache licensed][apache-badge]][apache-url]
[![Build Status][actions-badge]][actions-url]

[crates-badge]: https://img.shields.io/crates/v/reqwest-lb.svg
[crates-url]: https://crates.io/crates/reqwest-lb
[apache-badge]: https://img.shields.io/badge/license-Aapche-blue.svg
[apache-url]: LICENSE
[actions-badge]: https://github.com/w-sodalite/reqwest-lb/workflows/CI/badge.svg
[actions-url]: https://github.com/w-sodalite/reqwest-lb/actions?query=workflow%3ACI

A crate for [reqwest](https://crates.io/crates/reqwest) support load balancer, use
the [reqwest-middleware](https://crates.io/crates/reqwest-middleware).

## Overview

This crate provide a middleware `LoadBalancerMiddleware`, it implement `reqwest-middleware::Middleware`, then use the `lb://` instead `http://` or `https://`。

- ### dependencies

    ```toml
    [dependencies]
    reqwest = "0.12"
    reqwest-middleware = "0.3"
    reqwest-lb = "0.1"
    ```

- ### example

    ```rust
    use reqwest::{Client, Url};
    use reqwest_lb::LoadBalancerMiddleware;
    use reqwest_lb::SimpleLoadBalancer;
    use reqwest_lb::{LoadBalancerFactory, LoadBalancerPolicy};
    use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
  
    pub fn create<const N: usize>(
        ports: [u16; N],
        policy: LoadBalancerPolicy<Url>,
    ) -> ClientWithMiddleware {
        // create load balancer factory
        let mut factory = LoadBalancerFactory::default();
  
        // create url list
        let mut urls = Vec::with_capacity(N);
        for port in ports {
            urls.push(Url::parse(format!("http://127.0.0.1:{}", port).as_str()).unwrap());
        }
  
        // create load balancer
        let load_balancer = SimpleLoadBalancer::new(urls, policy);
  
        // register load balancer for the host: example-server
        factory.add("example-server", load_balancer);
  
        // create reqwest client
        let middleware = LoadBalancerMiddleware::new(factory);
        ClientBuilder::new(Client::builder().no_proxy().build().unwrap())
            .with(middleware)
            .build()
    }
  
    pub async fn run() {
        // use round robin policy
        let client = create([3001, 3002], LoadBalancerPolicy::RoundRobin);
        // http://127.0.0.1:3001
        let response = client.get("lb://example-server/").send().await.unwrap();
        // http://127.0.0.1:3002
        let response = client.get("lb://example-server/").send().await.unwrap();
    }
  
    ```

- ### load balancer policy

  - RoundRobin (default)
  - Random
  - First
  - Last
  - Weight

- ### discovery

  use `discovery` dynamic control the supplier elements, can send the elements change event `insert`,`remove` or
  `initialized` to the stream.

  ```rust
  
  use http::Extensions;
  use reqwest::Url;
  use reqwest_lb::discovery::Change;
  use reqwest_lb::supplier::discovery::DiscoverySupplier;
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
  
  ```  

## License

This project is licensed under the [Apache 2.0](./LICENSE)
