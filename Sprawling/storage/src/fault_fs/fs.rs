// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::vfs::Vfs;

use super::plan::{FaultPlan, FileState, State, TornTail};

#[derive(Clone)]
pub struct FaultFs(Arc<Mutex<State>>);

impl FaultFs {
    pub fn new(plan: FaultPlan) -> Self {
        FaultFs(Arc::new(Mutex::new(State {
            files: BTreeMap::new(),
            dirs: BTreeSet::new(),
            op: 0,
            bytes_read: 0,
            plan,
        })))
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn power_cut(&self) {
        let mut state = self.state();
        cut(&mut state);
    }

    pub fn op_count(&self) -> u64 {
        self.state().op
    }

    pub fn bytes_read(&self) -> u64 {
        self.state().bytes_read
    }

    fn charge_write(&self, bytes: &[u8], op_name: &'static str) -> io::Result<()> {
        {
            let mut state = self.state();
            if let Some(needle) = state.plan.cut_on_write
                && contains(bytes, needle.as_bytes())
            {
                state.plan.cut_on_write = None;
                state.op = state.op.saturating_add(1);
                cut(&mut state);
                return Err(io::Error::other(format!(
                    "power lost during {op_name} carrying `{needle}` (op {})",
                    state.op
                )));
            }
        }
        self.charge(op_name)
    }

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

        let common = file
            .durable
            .iter()
            .zip(&file.live)
            .take_while(|(durable, live)| durable == live)
            .count();
        let delta = file.live.get(common..).unwrap_or(&[]);
        let keep = match torn {
            TornTail::None => 0,
            TornTail::KeepBytes(k) => usize::try_from(k).unwrap_or(usize::MAX).min(delta.len()),
        };
        let reached = common.saturating_add(keep);
        let mut settled = file.live.get(..reached).unwrap_or(&[]).to_vec();
        settled.extend_from_slice(file.durable.get(reached..).unwrap_or(&[]));
        file.durable = settled.clone();
        file.live = settled;
    }
}

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
        let mut state = self.state();
        let bytes = state
            .files
            .get(path)
            .map(|f| f.live.clone())
            .ok_or_else(|| not_found(path))?;
        let moved = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        state.bytes_read = state.bytes_read.saturating_add(moved);
        Ok(bytes)
    }

    fn size(&self, path: &Path) -> io::Result<u64> {
        FaultFs::charge(self, "size")?;
        let state = self.state();
        let file = state.files.get(path).ok_or_else(|| not_found(path))?;
        Ok(u64::try_from(file.live.len()).unwrap_or(u64::MAX))
    }

    fn read_at(&self, path: &Path, offset: u64, len: u64) -> io::Result<Vec<u8>> {
        FaultFs::charge(self, "read_at")?;
        let mut state = self.state();
        let file = state.files.get(path).ok_or_else(|| not_found(path))?;
        let at = usize::try_from(offset).unwrap_or(usize::MAX);
        let take = usize::try_from(len).unwrap_or(usize::MAX);
        let tail = file.live.get(at..).unwrap_or(&[]);
        let bytes = tail.get(..take.min(tail.len())).unwrap_or(&[]).to_vec();
        let moved = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        state.bytes_read = state.bytes_read.saturating_add(moved);
        Ok(bytes)
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
        self.charge_write(bytes, "append")
    }

    fn write_at(&mut self, path: &Path, offset: u64, bytes: &[u8]) -> io::Result<()> {
        {
            let mut state = self.state();
            let file = state.files.entry(path.to_path_buf()).or_insert(FileState {
                durable: Vec::new(),
                live: Vec::new(),
                durable_entry: false,
            });
            let from = usize::try_from(offset).map_err(io::Error::other)?;
            let end = from
                .checked_add(bytes.len())
                .ok_or_else(|| io::Error::other("a write past the address space"))?;
            if file.live.len() < end {
                file.live.resize(end, 0);
            }
            if let Some(slot) = file.live.get_mut(from..end) {
                slot.copy_from_slice(bytes);
            }
        }
        self.charge_write(bytes, "write_at")
    }

    fn truncate(&mut self, path: &Path, len: u64) -> io::Result<()> {
        {
            let mut state = self.state();
            let file = state.files.get_mut(path).ok_or_else(|| not_found(path))?;
            let len = usize::try_from(len).map_err(io::Error::other)?;
            file.live.resize(len, 0);
        }
        self.charge("truncate")
    }

    fn rename(&mut self, from: &Path, to: &Path) -> io::Result<()> {
        {
            let mut state = self.state();
            let Some(mut file) = state.files.remove(from) else {
                return Err(not_found(from));
            };

            file.durable_entry = false;
            state.files.insert(to.to_path_buf(), file);
        }
        self.charge("rename")
    }

    fn sync_data(&mut self, path: &Path) -> io::Result<()> {

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

    fn copy_permissions(&mut self, from: &Path, to: &Path) -> io::Result<()> {
        for path in [from, to] {
            if !self.exists(path) {
                return Err(not_found(path));
            }
        }
        self.charge("copy_permissions")
    }

    fn make_readonly(&mut self, path: &Path) -> io::Result<()> {
        if !self.exists(path) {
            return Err(not_found(path));
        }
        self.charge("make_readonly")
    }

    fn exists(&self, path: &Path) -> bool {
        self.state().files.contains_key(path)
    }
}

#[cfg(test)]
mod tests;
