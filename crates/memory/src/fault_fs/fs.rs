// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! FaultFs: the second Vfs adapter — a deterministic power-loss model
//!. Compiled for tests and for citysim under the
//! `fault` feature; never in a production build.
//!
//! The model is stricter than any real platform so the write discipline
//! it enforces holds on every platform (memory-SPEC 8-2):
//! - every file has two planes: `durable` (survives power loss) and
//!   `live` (what the running process observes). `sync_data` promotes
//!   live to durable; a power cut drops the unsynced delta, except a
//!   `TornTail` prefix that models a torn write reaching the platter.
//! - a created file's directory entry survives only after `sync_dir` on
//!   its parent — even when its bytes were synced (stricter than POSIX).
//! - removals are instantly durable (simplification: the only remover is
//!   tail recovery, and a resurrected empty segment is harmless — reopen
//!   tolerates it).
//! - the op hitting `cut_at_op` fails with "power lost"; the plan is then
//!   consumed, so the same instance serves the powered reopen. Appends
//!   land on `live` before the cut check so the tear can bite the very
//!   write that died.
//!
//! Everything is explicit in [`FaultPlan`]; there is no randomness.

//! The fault filesystem: power cuts on demand.

use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::vfs::Vfs;

use super::plan::{FaultPlan, FileState, State, TornTail};
/// Shared-state handle: clone it, hand one clone to the ledger, keep one
/// to cut power and to reopen after the crash. `Arc<Mutex<_>>` rather
/// than `Rc<RefCell<_>>` because the inner seam it implements is `Send`
/// (sprawling-SPEC 8-44): a store that holds one has to cross a thread.
#[derive(Clone)]
pub struct FaultFs(Arc<Mutex<State>>);

impl FaultFs {
    pub fn new(plan: FaultPlan) -> Self {
        FaultFs(Arc::new(Mutex::new(State {
            files: BTreeMap::new(),
            dirs: BTreeSet::new(),
            op: 0,
            plan,
        })))
    }

    /// The state, whatever a thread that held it did before it died: a
    /// power-loss model has no invariant a panic could leave half-kept,
    /// so a poisoned lock is taken as it stands.
    fn state(&self) -> MutexGuard<'_, State> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Power loss now: see the module doc for the exact semantics.
    pub fn power_cut(&self) {
        let mut state = self.state();
        cut(&mut state);
    }

    pub fn op_count(&self) -> u64 {
        self.state().op
    }

    /// Counts the op; when it hits the plan, power dies: the cut applies
    /// and the op itself fails.
    fn charge(&self, op_name: &'static str) -> io::Result<()> {
        let mut state = self.state();
        state.op = state.op.saturating_add(1);
        if state.plan.cut_at_op == Some(state.op) {
            state.plan.cut_at_op = None;
            cut(&mut state);
            return Err(io::Error::other(format!(
                "power lost during {op_name} (op {})",
                state.op
            )));
        }
        Ok(())
    }
}

fn cut(state: &mut State) {
    let torn = state.plan.torn_tail;
    state.files.retain(|_, file| file.durable_entry);
    for file in state.files.values_mut() {
        let delta = file.live.get(file.durable.len()..).unwrap_or(&[]);
        let keep = match torn {
            TornTail::None => 0,
            TornTail::KeepBytes(k) => usize::try_from(k).unwrap_or(usize::MAX).min(delta.len()),
        };
        let mut settled = file.durable.clone();
        settled.extend_from_slice(delta.get(..keep).unwrap_or(&[]));
        file.durable = settled.clone();
        file.live = settled;
    }
}

/// Substring search over raw bytes: the ledger line is UTF-8 JSON, but a
/// `Vfs` sees bytes and must not assume otherwise.
fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    haystack.windows(needle.len()).any(|w| w == needle)
}

fn not_found(path: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::NotFound,
        format!("no such file: {}", path.display()),
    )
}

impl Vfs for FaultFs {
    fn create_dir_all(&mut self, dir: &Path) -> io::Result<()> {
        self.state().dirs.insert(dir.to_path_buf());
        self.charge("create_dir_all")
    }

    fn list(&self, dir: &Path) -> io::Result<Vec<PathBuf>> {
        // A read op still charges: the process can die mid-read too.
        FaultFs::charge(self, "list")?;
        let state = self.state();
        let mut files: Vec<PathBuf> = state
            .files
            .keys()
            .filter(|p| p.parent() == Some(dir))
            .cloned()
            .collect();
        files.sort();
        Ok(files)
    }

    fn list_dirs(&self, dir: &Path) -> io::Result<Vec<PathBuf>> {
        FaultFs::charge(self, "list_dirs")?;
        let state = self.state();
        let mut dirs: Vec<PathBuf> = state
            .files
            .keys()
            .filter_map(|path| path.parent())
            .filter(|parent| parent.parent() == Some(dir))
            .map(Path::to_path_buf)
            .collect();
        dirs.sort();
        dirs.dedup();
        Ok(dirs)
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        FaultFs::charge(self, "read")?;
        let state = self.state();
        state
            .files
            .get(path)
            .map(|f| f.live.clone())
            .ok_or_else(|| not_found(path))
    }

    fn append(&mut self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        {
            let mut state = self.state();
            let file = state.files.entry(path.to_path_buf()).or_insert(FileState {
                durable: Vec::new(),
                live: Vec::new(),
                durable_entry: false,
            });
            file.live.extend_from_slice(bytes);
        }
        // Bytes are on the live plane before either check: the tear model
        // can bite exactly this write.
        {
            let mut state = self.state();
            if let Some(needle) = state.plan.cut_on_write
                && contains(bytes, needle.as_bytes())
            {
                state.plan.cut_on_write = None;
                state.op = state.op.saturating_add(1);
                cut(&mut state);
                return Err(io::Error::other(format!(
                    "power lost during append carrying `{needle}` (op {})",
                    state.op
                )));
            }
        }
        self.charge("append")
    }

    fn truncate(&mut self, path: &Path, len: u64) -> io::Result<()> {
        {
            let mut state = self.state();
            let file = state.files.get_mut(path).ok_or_else(|| not_found(path))?;
            let len = usize::try_from(len).unwrap_or(usize::MAX);
            file.live.truncate(len);
        }
        self.charge("truncate")
    }

    fn rename(&mut self, from: &Path, to: &Path) -> io::Result<()> {
        {
            let mut state = self.state();
            let Some(mut file) = state.files.remove(from) else {
                return Err(not_found(from));
            };
            // The target's dir entry is new: it survives only after a
            // sync_dir. Stricter than reality — a cut here loses the
            // object entirely, but its put never returned Ok, so no
            // acknowledged effect is lost (memory-SPEC 8-3).
            file.durable_entry = false;
            state.files.insert(to.to_path_buf(), file);
        }
        self.charge("rename")
    }

    fn sync_data(&mut self, path: &Path) -> io::Result<()> {
        // Charge first: when power dies during the barrier, the barrier
        // never happened.
        self.charge("sync_data")?;
        let mut state = self.state();
        let file = state.files.get_mut(path).ok_or_else(|| not_found(path))?;
        file.durable = file.live.clone();
        Ok(())
    }

    fn sync_dir(&mut self, dir: &Path) -> io::Result<()> {
        self.charge("sync_dir")?;
        let mut state = self.state();
        for (path, file) in state.files.iter_mut() {
            if path.parent() == Some(dir) {
                file.durable_entry = true;
            }
        }
        Ok(())
    }

    fn remove_file(&mut self, path: &Path) -> io::Result<()> {
        {
            let mut state = self.state();
            if state.files.remove(path).is_none() {
                return Err(not_found(path));
            }
        }
        self.charge("remove_file")
    }

    fn exists(&self, path: &Path) -> bool {
        self.state().files.contains_key(path)
    }
}

#[cfg(test)]
mod tests;
