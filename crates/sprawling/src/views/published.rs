// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The views the fold last finished, published to every reader, and a
//! query answered from a snapshot of them with every slow read done
//! after the snapshot is let go.

use std::sync::{Arc, Mutex, PoisonError};

use kernel::AxError;

use super::holding::Views;

/// The views the fold last finished, handed to every reader
/// (sprawling-SPEC.md 8-93).
///
/// The lock covers one `Arc` copy or swap and nothing that can panic,
/// so even a poisoned lock holds a whole `Arc`, and it is read as one.
pub(crate) struct Published {
    current: Mutex<Arc<Views>>,
}

impl Published {
    pub(crate) fn new(views: Views) -> Published {
        Published {
            current: Mutex::new(Arc::new(views)),
        }
    }

    /// The views as the fold last published them. Held only while a
    /// query copies out what it needs, because the fold takes a retired
    /// copy back only once no reader holds it.
    pub(crate) fn snapshot(&self) -> Arc<Views> {
        Arc::clone(&self.current.lock().unwrap_or_else(PoisonError::into_inner))
    }

    /// Publishes `latest` and hands back the copy it replaces, which is
    /// dropped or reclaimed outside the lock.
    pub(crate) fn replace(&self, latest: Arc<Views>) -> Arc<Views> {
        std::mem::replace(
            &mut *self.current.lock().unwrap_or_else(PoisonError::into_inner),
            latest,
        )
    }
}

/// Answers one query from a snapshot of the views, holding it only while
/// [`Views::prepare`] copies out what the query needs: the disk, git or
/// network read in [`Prepared::finish`] runs after the snapshot is let
/// go, so neither the fold nor another reader waits on it.
///
/// The answer is dated by the snapshot it is prepared from, so the date
/// is exactly the first record the answer does not reflect.
///
/// The `Result` is the shape the console's `Answering` takes; this path
/// refuses nothing itself, because a snapshot is an `Arc` taken whole
/// and has no poisoned state.
pub(crate) fn answer_outside_the_lock(
    views: &Published,
    query: &channels::Query,
) -> (kernel::Seq, Result<channels::Answer, AxError>) {
    let snapshot = views.snapshot();
    let as_of = snapshot.next_unfolded();
    let prepared = snapshot.prepare(query);
    drop(snapshot);
    (as_of, Ok(prepared.finish()))
}
