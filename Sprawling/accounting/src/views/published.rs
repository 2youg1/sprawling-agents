// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::{Arc, Mutex, PoisonError};

use kernel::AxError;

use super::holding::Views;

pub struct Published {
    current: Mutex<Arc<Views>>,
}

impl Published {
    pub fn new(views: Views) -> Published {
        Published {
            current: Mutex::new(Arc::new(views)),
        }
    }

    pub fn snapshot(&self) -> Arc<Views> {
        Arc::clone(&self.current.lock().unwrap_or_else(PoisonError::into_inner))
    }

    pub fn replace(&self, latest: Arc<Views>) -> Arc<Views> {
        std::mem::replace(
            &mut *self.current.lock().unwrap_or_else(PoisonError::into_inner),
            latest,
        )
    }
}

pub fn answer_outside_the_lock(
    views: &Published,
    query: &wire::Query,
) -> (kernel::Seq, Result<wire::Answer, AxError>) {
    let snapshot = views.snapshot();
    let as_of = snapshot.next_unfolded();
    let prepared = snapshot.prepare(query);
    drop(snapshot);
    (as_of, prepared.answer())
}
