// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::Arc;

use kernel::AxError;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub enum Delivered {
    ToThePeer,
    NobodyAsked,
    PeerGone,
}

#[derive(Clone)]
pub struct Reply(Option<Arc<dyn Fn(AxError) -> Delivered + Send + Sync>>);

impl Reply {

    pub fn to(sink: impl Fn(AxError) -> Delivered + Send + Sync + 'static) -> Reply {
        Reply(Some(Arc::new(sink)))
    }

    pub fn nowhere() -> Reply {
        Reply(None)
    }

    pub fn refuse(&self, error: AxError) -> Delivered {
        match &self.0 {
            Some(sink) => sink(error),
            None => Delivered::NobodyAsked,
        }
    }
}

impl std::fmt::Debug for Reply {

    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let face = if self.0.is_some() {
            "Reply(a peer)"
        } else {
            "Reply(nowhere)"
        };
        f.write_str(face)
    }
}

#[derive(Debug, Serialize)]
pub struct AcpProgress {
    pub run: String,
    pub turns: u32,
    pub finished: bool,
}
