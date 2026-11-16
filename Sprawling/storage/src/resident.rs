// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;

pub const RESIDENT_TOTAL_BYTES: usize = 24 * 1024 * 1024;

pub struct Resident<R, K> {
    budget: usize,
    held: usize,
    order: VecDeque<(R, K)>,
    bytes: BTreeMap<(R, K), Arc<[u8]>>,
    frozen: BTreeSet<R>,
}

impl<R: Ord + Clone, K: Ord + Clone> Resident<R, K> {
    pub fn new(budget_bytes: usize) -> Self {
        Self {
            budget: budget_bytes,
            held: 0,
            order: VecDeque::new(),
            bytes: BTreeMap::new(),
            frozen: BTreeSet::new(),
        }
    }

    pub fn insert<E>(
        &mut self,
        owner: &R,
        key: &K,
        load: impl FnOnce(&K) -> Result<Vec<u8>, E>,
    ) -> Result<(), E> {
        let bytes = Arc::<[u8]>::from(load(key)?);
        self.place(owner, key, bytes);
        Ok(())
    }

    pub fn read<E>(
        &mut self,
        owner: &R,
        key: &K,
        load: impl FnOnce(&K) -> Result<Vec<u8>, E>,
    ) -> Result<Arc<[u8]>, E> {
        let name = (owner.clone(), key.clone());
        if let Some(bytes) = self.bytes.get(&name) {
            return Ok(Arc::clone(bytes));
        }
        let bytes = Arc::<[u8]>::from(load(key)?);
        self.place(owner, key, Arc::clone(&bytes));
        Ok(bytes)
    }

    pub fn evict_oldest(&mut self) {
        if let Some(name) = self.order.pop_front() {
            self.forget(&name);
        }
    }

    pub fn freeze(&mut self, owner: &R) {
        let (gone, kept): (VecDeque<_>, VecDeque<_>) =
            self.order.drain(..).partition(|(run, _)| run == owner);
        self.order = kept;
        gone.iter().for_each(|name| self.forget(name));
        self.frozen.insert(owner.clone());
    }

    pub fn resident_bytes(&self) -> usize {
        self.held
    }

    fn place(&mut self, owner: &R, key: &K, bytes: Arc<[u8]>) {
        if self.frozen.contains(owner) {
            return;
        }
        let name = (owner.clone(), key.clone());
        if self.bytes.contains_key(&name) {
            self.order.retain(|placed| placed != &name);
            self.forget(&name);
        }
        self.held = self.held.saturating_add(bytes.len());
        self.bytes.insert(name.clone(), bytes);
        self.order.push_back(name);
        self.trim();
    }

    fn trim(&mut self) {
        while self.held > self.budget && !self.order.is_empty() {
            self.evict_oldest();
        }
    }

    fn forget(&mut self, name: &(R, K)) {
        if let Some(bytes) = self.bytes.remove(name) {
            self.held = self.held.saturating_sub(bytes.len());
        }
    }
}

#[cfg(test)]
#[path = "resident/tests.rs"]
mod tests;
