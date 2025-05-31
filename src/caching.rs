use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, RwLock};
use std::hash::Hash;

use rustc_hash::FxHashMap as HashMap;


const CACHE_SIZE_LIMIT: usize = 500000;

pub struct Cache<K, V: Clone> {
    store: RwLock<Option<HashMap<K, (V, u64)>>>,
    time_counter: AtomicU64,
}

impl<K: Hash + Eq, V: Clone> Cache<K, V> {
    pub const fn new() -> Self {
        Cache { store: RwLock::new(None), time_counter: AtomicU64::new(0) }
    }

    pub fn lookup(&self, key: &K) -> Option<V> {
        self.store.read().unwrap().as_ref()
            .and_then(|cache| cache.get(key).cloned())
            .map(|v| v.0)
    }

    pub fn insert(&self, key: K, value: V) {
        let mut cache_opt = self.store.write().unwrap();

        if let Some(cache) = cache_opt.as_mut() {
            if cache.len() > CACHE_SIZE_LIMIT {
                let total_age: u64 = cache.values()
                    .map(|v| v.1)
                    .sum();
                let avg_age = total_age / (cache.len() as u64);
                cache.retain(|_, v| v.1 < avg_age);
            }
            cache.insert(key, (value, self.time_counter.fetch_add(1, Ordering::SeqCst)));
        } else {
            let mut cache = HashMap::default();
            cache.insert(key, (value, 0));
            *cache_opt = Some(cache);
            self.time_counter.store(0, Ordering::SeqCst);
        }

    }
}
