// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::Arc;

use kernel::{AxError, IdemKey};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub enum Delivered {
    ToThePeer,
    NobodyAsked,
    PeerGone,
}

#[derive(Clone)]
pub struct Reply {
    sink: Option<Arc<dyn Fn(Outcome) -> Delivered + Send + Sync>>,
    request: Option<IdemKey>,
}

pub enum Outcome {
    Accepted(IdemKey),
    Refused(AxError),
}

impl Reply {
    pub fn to(sink: impl Fn(AxError) -> Delivered + Send + Sync + 'static) -> Reply {
        Self::outcomes(move |outcome| match outcome {
            Outcome::Refused(error) => sink(error),
            Outcome::Accepted(_) => Delivered::NobodyAsked,
        })
    }

    pub fn outcomes(sink: impl Fn(Outcome) -> Delivered + Send + Sync + 'static) -> Reply {
        Reply {
            sink: Some(Arc::new(sink)),
            request: None,
        }
    }

    pub fn nowhere() -> Reply {
        Reply {
            sink: None,
            request: None,
        }
    }

    #[must_use]
    pub fn for_request(mut self, request: Option<IdemKey>) -> Self {
        self.request = request;
        self
    }

    pub fn request(&self) -> Option<IdemKey> {
        self.request
    }

    pub fn refuse(&self, error: AxError) -> Delivered {
        self.send(Outcome::Refused(error))
    }

    pub fn accept(&self, key: IdemKey) -> Delivered {
        self.send(Outcome::Accepted(key))
    }

    fn send(&self, outcome: Outcome) -> Delivered {
        match &self.sink {
            Some(sink) => sink(outcome),
            None => Delivered::NobodyAsked,
        }
    }
}

impl std::fmt::Debug for Reply {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let face = if self.sink.is_some() {
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
