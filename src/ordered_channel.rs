use std::sync::{Condvar, Mutex};

use crate::HashMap;

pub struct OrderedChannel<T> {
    inner: Mutex<HashMap<usize, T>>,
    cond: Condvar,
}

pub struct OrderedChannelIterator<'a, T> {
    channel: &'a OrderedChannel<T>,
    iter_counter: usize,
    total: usize,
}


impl<T> OrderedChannel<T> {
    pub fn new() -> OrderedChannel<T> {
        OrderedChannel {
            inner: Mutex::new(HashMap::default()),
            cond: Condvar::new(),
        }
    }

    pub fn put(&self, i: usize, object: T) {
        let mut inner = self.inner.lock().unwrap();
        inner.insert(i, object);
        self.cond.notify_all();
    }

    pub fn get(&self, i: usize) -> T {
        let mut inner = self.inner.lock().unwrap();
        loop {
            match inner.remove(&i) {
                Some(item) => return item,
                None => inner = self.cond.wait(inner).unwrap(),
            }
        }
    }

    pub fn iter(&self, total: usize) -> OrderedChannelIterator<'_, T> {
        OrderedChannelIterator { channel: self, iter_counter: 0, total }
    }
}

impl<T> Iterator for OrderedChannelIterator<'_, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.iter_counter == self.total {
            return None;
        }

        self.iter_counter += 1;
        Some(self.channel.get(self.iter_counter - 1))
    }
}
