// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The one file a reading pass is in the middle of
//! (`crates/runtime/spec/Tools/Read.lean` §8-29-8, D14).
//!
//! A truncated answer says the file goes on, and the model almost always
//! reads the rest of it: keeping the bytes that window already cost turns
//! the continuation into a hot read. Everything else releases them. One
//! slot, replaced by the next file a read opens, dropped when the file
//! is read to its end — so a run holds at most one file's bytes, and a
//! run that abandons a read holds them until the next read rather than
//! for ever piling more up.
//!
//! The slot answers no judgement. A caller has already opened the file
//! and found it to be the one judged before it asks here, so a stale
//! slot can only miss, never redirect a read.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

/// The bytes one window read left, and the file they belong to.
struct Held {
    real: PathBuf,
    size: u64,
    modified: SystemTime,
    /// The byte the held bytes start at.
    base: u64,
    bytes: Vec<u8>,
}

/// The slot a run keeps its one in-progress file in.
#[derive(Default)]
pub(super) struct Hot {
    held: Mutex<Option<Held>>,
}

/// The bytes a caller may use instead of reading: where they start, and
/// the bytes themselves.
pub(super) struct Covered {
    pub base: u64,
    pub bytes: Vec<u8>,
}

impl Hot {
    /// The held bytes when they cover `[start, start + want)` of the file
    /// the caller has just opened: same real location, same size, same
    /// stamp. A slot for another file is released here, which is what
    /// keeps a run to one held file.
    pub(super) fn covered(
        &self,
        real: &Path,
        size: u64,
        modified: SystemTime,
        start: u64,
        want: usize,
    ) -> Option<Covered> {
        // What this call releases leaves the slot inside the lock and is
        // freed outside it (D15): a read never waits behind the
        // dropping of the file the pass has finished with.
        let released = {
            let mut held = self.held.lock().ok()?;
            let found = held.as_ref()?;
            let same = found.real == real && found.size == size && found.modified == modified;
            if same {
                let end = start.saturating_add(u64::try_from(want).ok()?);
                let held_end = found
                    .base
                    .saturating_add(u64::try_from(found.bytes.len()).ok()?);
                if start < found.base || end > held_end {
                    return None;
                }
                let from = usize::try_from(start.saturating_sub(found.base)).ok()?;
                let to = usize::try_from(end.saturating_sub(found.base)).ok()?;
                return Some(Covered {
                    base: found.base,
                    bytes: found.bytes.get(from..to)?.to_vec(),
                });
            }
            held.take()
        };
        drop(released);
        None
    }

    /// Whether anything is held at all. A read that finds the slot empty
    /// asks the platform for facts and bytes in one open, rather than
    /// paying a second open to learn what nothing was waiting on.
    pub(super) fn is_empty(&self) -> bool {
        self.held.lock().is_ok_and(|held| held.is_none())
    }

    /// Holds the bytes of a file whose answer was truncated: the pass is
    /// still going, so the continuation starts warm.
    pub(super) fn keep(
        &self,
        real: &Path,
        size: u64,
        modified: SystemTime,
        base: u64,
        bytes: &[u8],
    ) {
        if let Ok(mut held) = self.held.lock() {
            *held = Some(Held {
                real: real.to_path_buf(),
                size,
                modified,
                base,
                bytes: bytes.to_vec(),
            });
        }
    }

    /// Releases what is held: the file was read to its end, or the read
    /// that would have continued it went elsewhere.
    pub(super) fn release(&self) {
        // Taken under the lock, freed after it, for the reason
        // `covered` gives.
        let released = self.held.lock().ok().and_then(|mut held| held.take());
        drop(released);
    }
}
