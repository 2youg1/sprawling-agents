// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What goes back to the one who asked: where a refusal is delivered,
//! where it ended up, and what an outside editor is told about the
//! request it made.
//!
//! Vocabulary, not transport. The city's one writer names these types
//! and never listens on a socket, so they sit outside the `server`
//! feature, which carries only the listener (wire-SPEC.md 12.2).

use std::sync::Arc;

use kernel::AxError;
use serde::Serialize;

/// Where a refusal ended up.
///
/// Three states rather than a `Result`, because "nobody asked" and "the
/// one who asked has gone" are different facts: the first is the
/// schedule working normally, and the second is worth a diagnostic line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub enum Delivered {
    ToThePeer,
    NobodyAsked,
    PeerGone,
}

/// Where a refusal goes back to.
///
/// A command travels socket to desk to worker thread, and only events
/// come back, so a refusal made minutes later has no way home. Making
/// it an event instead would tell everyone watching about one person's
/// mistyped URL; the city's history is not anybody's error log. So the
/// command carries the address of whoever sent it.
///
/// Holds a function rather than a channel so that this crate's public
/// signature does not name a transport the assembly layer would then
/// have to name too.
#[derive(Clone)]
pub struct Reply(Option<Arc<dyn Fn(AxError) -> Delivered + Send + Sync>>);

impl Reply {
    /// A reply address that reaches the peer that sent the command.
    pub fn to(sink: impl Fn(AxError) -> Delivered + Send + Sync + 'static) -> Reply {
        Reply(Some(Arc::new(sink)))
    }

    /// No peer asked. The schedule starts work by itself, and so does
    /// the startup scan; a refusal there has nobody to be handed to.
    pub fn nowhere() -> Reply {
        Reply(None)
    }

    /// Hands the refusal back, and says where it ended up.
    pub fn refuse(&self, error: AxError) -> Delivered {
        match &self.0 {
            Some(sink) => sink(error),
            None => Delivered::NobodyAsked,
        }
    }
}

impl std::fmt::Debug for Reply {
    /// Says whether there is somebody to answer, and never what was
    /// said: the payload is an `AxError` on its way to one peer.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let face = if self.0.is_some() {
            "Reply(a peer)"
        } else {
            "Reply(nowhere)"
        };
        f.write_str(face)
    }
}

/// What an accepted request gets back: the run it became, and nothing
/// else. Progress is what an editor may see; the city's history is not
/// published through this door.
#[derive(Debug, Serialize)]
pub struct AcpProgress {
    pub run: String,
    pub turns: u32,
    pub finished: bool,
}
