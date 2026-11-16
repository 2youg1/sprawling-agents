// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The working set read back from disk: a cache with a byte budget,
//! trimmed from its oldest entry, that holds nothing for a frozen run
//! (`crates/storage/Spec.lean` §8-40).
//!
//! The properties it keeps are proved in
//! `crates/sprawling/spec/Serving/Memory.lean` §8-173: the resident bytes
//! never exceed the budget, a frozen run holds nothing, and every read
//! answers the bytes on disk, an evicted entry's included. The budget
//! counts bytes rather than entries because entries differ in size by
//! orders of magnitude (sprawling D42).

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;

/// The whole read-back budget: 12 MiB for the views and the side index
/// and 12 MiB of read-back, the figure Memory.lean's budget table
/// proposes until a reading replaces it.
pub const RESIDENT_TOTAL_BYTES: usize = 24 * 1024 * 1024;

/// Entries in the order they were placed, each owned by one run and named
/// by its address on disk.
///
/// A hit does not reorder: the model trims first in, first out, and only
/// placing an address again moves it to the newest end.
pub struct Resident<R, K> {
    budget: usize,
    held: usize,
    order: VecDeque<(R, K)>,
    bytes: BTreeMap<(R, K), Arc<[u8]>>,
    frozen: BTreeSet<R>,
}

impl<R: Ord + Clone, K: Ord + Clone> Resident<R, K> {
    /// An empty cache that never holds more than `budget_bytes`.
    pub fn new(budget_bytes: usize) -> Self {
        Self {
            budget: budget_bytes,
            held: 0,
            order: VecDeque::new(),
            bytes: BTreeMap::new(),
            frozen: BTreeSet::new(),
        }
    }

    /// Reads `key` from disk through `load` and places it, then drops the
    /// oldest entries until the cache is within its budget. A frozen run
    /// is not placed; an entry larger than the budget is not kept. A
    /// failed `load` changes nothing and its error is returned.
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

    /// Answers the cached bytes on a hit; on a miss reads them through
    /// `load`, places them as [`Self::insert`] does, and answers them.
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

    /// Drops the oldest entry, when the system asks for memory back or a
    /// beat evicts.
    pub fn evict_oldest(&mut self) {
        if let Some(name) = self.order.pop_front() {
            self.forget(&name);
        }
    }

    /// Drops every entry of `owner` and places nothing for it afterwards.
    pub fn freeze(&mut self, owner: &R) {
        let (gone, kept): (VecDeque<_>, VecDeque<_>) =
            self.order.drain(..).partition(|(run, _)| run == owner);
        self.order = kept;
        gone.iter().for_each(|name| self.forget(name));
        self.frozen.insert(owner.clone());
    }

    /// The bytes the cache holds now.
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

    /// Drops from the oldest end while the cache is over its budget, the
    /// newest entry included when it alone is larger than the budget.
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
