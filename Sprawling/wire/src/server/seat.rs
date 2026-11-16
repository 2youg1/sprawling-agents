// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::B3Hash;
use tokio::sync::watch;

use super::door::LocalDoor;

pub(crate) struct Seat {
    pub(super) door: LocalDoor,
    pub(super) presented: Option<String>,

    pub(super) token: Option<B3Hash>,
    pub(super) ended: watch::Receiver<()>,
}

impl Seat {
    pub(crate) fn presented(&self) -> Option<&str> {
        self.presented.as_deref()
    }

    pub(crate) async fn ended(&mut self) {
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
