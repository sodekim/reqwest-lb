use crate::discovery::{Change, Discovery};
use crate::supplier::Supplier;
use crate::with::With;
use futures::future::BoxFuture;
use std::collections::HashMap;
use std::convert::Infallible;
use std::fmt::Debug;
use std::future::poll_fn;
use std::hash::Hash;
use std::pin::pin;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use tokio::spawn;
use tokio::sync::{Notify, RwLock};
use tracing::{error, info};

///
/// State: new
///
const STATE_NEW: u8 = 0;

///
/// State: initializing
///
const STATE_INITIALIZING: u8 = 1;

///
/// State: initialized
///
const STATE_INITIALIZED: u8 = 2;

pub struct DiscoverySupplier<D: Discovery> {
    state: Arc<AtomicU8>,
    elements: Arc<RwLock<HashMap<D::Key, D::Element>>>,
    notify: Arc<Notify>,
}

impl<D: Discovery> Clone for DiscoverySupplier<D> {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            elements: self.elements.clone(),
            notify: self.notify.clone(),
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
    pub fn new(discovery: D) -> Self {
        let state = Arc::new(AtomicU8::new(STATE_NEW));
        let elements = Arc::new(RwLock::new(HashMap::new()));
        let notify = Arc::new(Notify::new());
        Self::collect(state.clone(), discovery, elements.clone(), notify.clone());
        Self {
            state,
            elements,
            notify,
        }
    }

    fn try_upgrade_state(state: &AtomicU8, old_state: u8, new_state: u8) -> bool {
        state
            .compare_exchange(old_state, new_state, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    fn collect(
        state: Arc<AtomicU8>,
        discovery: D,
        elements: Arc<RwLock<HashMap<D::Key, D::Element>>>,
        notify: Arc<Notify>,
    ) {
        if Self::try_upgrade_state(&state, STATE_NEW, STATE_INITIALIZING) {
            spawn(async move {
                let mut discovery = pin!(discovery);
                while let Some(change) = poll_fn(|cx| discovery.as_mut().poll_change(cx)).await {
                    match change {
                        Ok(change) => match change {
                            Change::Insert(k, v) => {
                                info!("Collector receive insert change: key={:?}", k);
                                let mut items = elements.write().await;
                                items.insert(k, v);
                            }
                            Change::Remove(k) => {
                                info!("Collector receive remove change: key={:?}", k);
                                let mut items = elements.write().await;
                                items.remove(&k);
                            }
                            Change::Initialized => {
                                if Self::try_upgrade_state(
                                    &state,
                                    STATE_INITIALIZING,
                                    STATE_INITIALIZED,
                                ) {
                                    notify.notify_waiters()
                                }
                            }
                        },
                        Err(e) => error!("Poll discovery change error: {:?}", e),
                    }
                }
            });
        }
    }
}

impl<D> Supplier for DiscoverySupplier<D>
where
    D: Discovery,
    D::Key: Ord + Clone + Send + Sync + 'static,
    D::Element: Clone + Send + Sync + 'static,
{
    type Element = D::Element;
    type Error = Infallible;
    type Future = BoxFuture<'static, Result<Vec<Self::Element>, Self::Error>>;

    fn get(&self) -> Self::Future {
        let state = self.state.clone();
        let notify = self.notify.clone();
        let elements = self.elements.clone();
        Box::pin(async move {
            if state.load(Ordering::SeqCst) != STATE_INITIALIZED {
                notify.notified().await;
            }
            let elements = elements
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
