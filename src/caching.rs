use std::collections::HashMap;
use std::sync::RwLock;
use std::hash::Hash;


pub struct Cache<K, V: Clone>(RwLock<Option<HashMap<K, V>>>);

impl<K: Hash + Eq, V: Clone> Cache<K, V> {
    pub const fn new() -> Self {
        Cache(RwLock::new(None))
    }

    pub fn lookup(&self, key: &K) -> Option<V> {
        self.0.read().unwrap().as_ref()
            .and_then(|cache| cache.get(key).cloned())
    }

    pub fn insert(&self, key: K, value: V) {
        let mut cache_opt = self.0.write().unwrap();

        if let Some(cache) = cache_opt.as_mut() {
            cache.insert(key, value);
        } else {
            let mut cache = HashMap::new();
            cache.insert(key, value);
            *cache_opt = Some(cache);
        }

    }
}
