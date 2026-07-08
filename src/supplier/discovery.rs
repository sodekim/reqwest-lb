use crate::discovery::{Change, Discovery};
use crate::runtime::Runtime;
use crate::supplier::Supplier;
use crate::with::With;
use futures::future::BoxFuture;
use std::collections::HashMap;
use std::convert::Infallible;
use std::fmt::Debug;
use std::future::poll_fn;
use std::hash::Hash;
use std::pin::pin;
use std::sync::Arc;
use tokio::sync::{watch, RwLock};
use tracing::{error, info};

struct Shared<D: Discovery> {
    elements: RwLock<HashMap<D::Key, D::Element>>,
    initialized: watch::Sender<bool>,
}

impl<D: Discovery> Shared<D> {
    fn new() -> (Shared<D>, watch::Receiver<bool>) {
        let (initialized, rx) = watch::channel(false);
        (
            Self {
                elements: RwLock::new(HashMap::new()),
                initialized,
            },
            rx,
        )
    }
}

pub struct DiscoverySupplier<D: Discovery> {
    shared: Arc<Shared<D>>,
    initialized: watch::Receiver<bool>,
}

impl<D: Discovery> Clone for DiscoverySupplier<D> {
    fn clone(&self) -> Self {
        Self {
            shared: self.shared.clone(),
            initialized: self.initialized.clone(),
        }
    }
}

impl<D> DiscoverySupplier<D>
where
    D: Discovery + Send + 'static,
    D::Key: Eq + Hash + Send + Sync + 'static,
    D::Element: Send + Sync + 'static,
    D::Error: Debug + Send,
{
    pub fn new<R: Runtime>(discovery: D) -> Self {
        let (shared, initialized) = Shared::new();
        let shared = Arc::new(shared);
        Self::collect::<R>(shared.clone(), discovery);
        Self {
            shared,
            initialized,
        }
    }

    fn collect<R: Runtime>(shared: Arc<Shared<D>>, discovery: D) {
        R::spawn(async move {
            let mut discovery = pin!(discovery);
            while let Some(change) = poll_fn(|cx| discovery.as_mut().poll_change(cx)).await {
                match change {
                    Ok(change) => match change {
                        Change::Insert(k, v) => {
                            info!("Collector receive insert change: key={:?}", k);
                            let mut items = shared.elements.write().await;
                            items.insert(k, v);
                        }
                        Change::Remove(k) => {
                            info!("Collector receive remove change: key={:?}", k);
                            let mut items = shared.elements.write().await;
                            items.remove(&k);
                        }
                        Change::Initialized => {
                            let _ = shared.initialized.send(true);
                        }
                    },
                    Err(e) => error!("Poll discovery change error: {:?}", e),
                }
            }
        });
    }
}

impl<D> Supplier for DiscoverySupplier<D>
where
    D: Discovery + 'static,
    D::Key: Ord + Clone + Send + Sync + 'static,
    D::Element: Clone + Send + Sync + 'static,
{
    type Element = D::Element;
    type Error = Infallible;
    type Future = BoxFuture<'static, Result<Vec<Self::Element>, Self::Error>>;

    fn get(&self) -> Self::Future {
        let shared = self.shared.clone();
        let mut initialized = self.initialized.clone();
        Box::pin(async move {
            loop {
                if *initialized.borrow() {
                    break;
                }
                if initialized.changed().await.is_err() {
                    break;
                }
            }
            let elements = shared
                .elements
                .read()
                .await
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect::<Vec<_>>()
                .with(|v| v.sort_by(|(k1, _), (k2, _)| k1.cmp(k2)));
            Ok(elements.into_iter().map(|(_, v)| v).collect())
        })
    }
}
