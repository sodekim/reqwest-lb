use async_stream::stream;
use dashmap::DashSet;
use futures::stream::BoxStream;
use nacos_sdk::api::naming::{
    NamingChangeEvent, NamingEventListener, NamingServiceBuilder, ServiceInstance,
};
use nacos_sdk::api::props::ClientProps;
use reqwest::{Client, Url};
use reqwest_lb::discovery::Change;
use reqwest_lb::supplier::{DiscoverySupplier, LoadBalancer};
use reqwest_lb::{LoadBalancerMiddleware, LoadBalancerPolicy, LoadBalancerRegistry};
use reqwest_middleware::ClientBuilder;
use std::collections::HashSet;
use std::convert::Infallible;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::time::sleep;
use tracing::{info, Level};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    // create load balancer factory
    let mut registry = LoadBalancerRegistry::default();
    let discovery = discovery();
    let load_balancer = LoadBalancer::new(
        DiscoverySupplier::new(discovery),
        LoadBalancerPolicy::RoundRobin,
    );
    registry.add("app", load_balancer);

    // create reqwest client
    let middleware = LoadBalancerMiddleware::new(registry);
    let client = ClientBuilder::new(Client::builder().no_proxy().build().unwrap())
        .with(middleware)
        .build();
    loop {
        let text = client
            .get("lb://app/index")
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        println!("{}", text);
        sleep(Duration::from_secs(5)).await;
    }
}

#[derive(Debug, Clone)]
struct Instance {
    instance: ServiceInstance,
}

impl Instance {
    pub fn new(instance: ServiceInstance) -> Self {
        Self { instance }
    }
}

fn hash(instance: &ServiceInstance) -> u64 {
    let mut hasher = DefaultHasher::new();
    instance.ip.hash(&mut hasher);
    instance.port.hash(&mut hasher);
    hasher.finish()
}

impl TryInto<Url> for Instance {
    type Error = url::ParseError;

    fn try_into(self) -> Result<Url, Self::Error> {
        Url::parse(&format!(
            "http://{}:{}",
            self.instance.ip, self.instance.port
        ))
    }
}

fn discovery() -> BoxStream<'static, Result<Change<u64, Instance>, Infallible>> {
    let naming = NamingServiceBuilder::new(
        ClientProps::new()
            .server_addr("127.0.0.1:8848")
            .namespace("dev")
            .auth_username("sodax")
            .auth_password("123456"),
    )
    .enable_auth_plugin_http()
    .build()
    .unwrap();

    let stream = stream! {

        let keys = DashSet::new();

        // get initialize instances from nacos
        for instance in naming
            .get_all_instances("app".to_string(), Some("default".to_string()), vec![], false)
            .await
            .unwrap() {
            let key = hash(&instance);
            keys.insert(key);
            yield Ok(Change::Insert(key, Instance::new(instance)));
        }

         // NOTICE: the initialized event notify load balancer all elements already insert
        yield Ok(Change::Initialized);

        let (tx,mut rx) = unbounded_channel();
        let listener = ServiceInstanceEventListener::new(keys,tx);
        naming.subscribe("app".to_string(), Some("default".to_string()), vec![], Arc::new(listener)).await.unwrap();

        while let Some(change) = rx.recv().await {
            yield Ok(change);
        }

    };
    Box::pin(stream)
}

struct ServiceInstanceEventListener {
    keys: DashSet<u64>,
    tx: UnboundedSender<Change<u64, Instance>>,
}

impl ServiceInstanceEventListener {
    pub fn new(keys: DashSet<u64>, tx: UnboundedSender<Change<u64, Instance>>) -> Self {
        Self { keys, tx }
    }
}

impl NamingEventListener for ServiceInstanceEventListener {
    fn event(&self, event: Arc<NamingChangeEvent>) {
        info!("naming event: {:?}", event);
        if let Some(instances) = event.instances.as_deref() {
            let mut alive_keys = HashSet::new();
            instances.into_iter().for_each(|instance| {
                let key = hash(&instance);
                if !self.keys.contains(&key) {
                    self.keys.insert(key);
                    self.tx
                        .send(Change::Insert(key, Instance::new(instance.clone())))
                        .unwrap();
                }
                alive_keys.insert(key);
            });

            self.keys
                .iter()
                .filter(|key| !alive_keys.contains(key))
                .for_each(|key| self.tx.send(Change::Remove(*key)).unwrap());
            self.keys.retain(|k| alive_keys.contains(k));
        }
    }
}
