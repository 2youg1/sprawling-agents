// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One socket's place at this machine's door for a browser
//! (`crates/wire/spec/Server/Sessions.lean` §8-93s): the credential its hello
//! showed, the session token it holds while it lives, and the signal
//! that the door has since ended a token.

use kernel::B3Hash;
use tokio::sync::watch;

use super::door::LocalDoor;

/// A welcomed socket's seat, taken by [`LocalDoor::seat`]. Dropping it
/// leaves the seat, which starts the token's idle allowance once no
/// socket holds it.
pub(crate) struct Seat {
    pub(super) door: LocalDoor,
    pub(super) presented: Option<String>,
    /// The session token this seat holds; `None` when the hello showed
    /// the native key, which belongs to no device.
    pub(super) token: Option<B3Hash>,
    pub(super) ended: watch::Receiver<()>,
}

impl Seat {
    /// The credential the socket's hello showed.
    pub(crate) fn presented(&self) -> Option<&str> {
        self.presented.as_deref()
    }

    /// Resolves when the door may have ended this socket's token; the
    /// socket then judges [`Seat::presented`] again.
    pub(crate) async fn ended(&mut self) {
        // The sender lives in the door this seat holds, so it cannot be
        // dropped first; a closed channel would mean nothing can end.
        if self.ended.changed().await.is_err() {
            std::future::pending::<()>().await;
        }
    }
}

impl Drop for Seat {
    fn drop(&mut self) {
        if let Some(token) = self.token {
            self.door.unseat(&token);
        }
    }
}
