// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The listening end, and the humble half of it (ARCHITECTURE section
//! 9). Every branch here is a send, a receive, or the end of a session;
//! the judgements it applies are `channels::reception`'s and the bytes
//! it serves are `channels::assets`'.
//!
//! Five jobs and no policy: serve the client bundle, upgrade a
//! WebSocket, accept an upload, take a credential from a caller on this
//! machine, and let an outside editor drive the city.
//!
//! A refusal made minutes later has no way home, which is why a command
//! carries the [`Reply`] address of whoever sent it.

//! Replies: deliveries and refusals.

use std::sync::Arc;

use kernel::AxError;

pub(crate) fn refusal_text(err: &AxError) -> String {
    format!("{}: {}", err.action(), err.recovery())
}

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
