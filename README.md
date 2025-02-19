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

    ```

- ### discovery

  use `discovery` dynamic control the supplier elements, can send the elements change event `insert`,`remove` or
  `initialized` to the stream.

    ```rust

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

    ```

- ### load balancer policy

  - RoundRobin (default)
  - Random
  - First
  - Last
  - Weight

## License

This project is licensed under the [Apache 2.0](./LICENSE)
