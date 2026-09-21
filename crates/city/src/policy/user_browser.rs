// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The address a person's own browser answers on.

use kernel::{AxCode, AxError};

/// What a building's `usersbrowser` key says.
///
/// The value is both the switch and the address: a person who wrote the
/// line has answered both questions at once, and two lines would be two
/// answers that can disagree. `true` alone is a person who has enabled
/// the tool and not yet said where their browser answers; every attach
/// then asks them, and asks with the step they should take.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserBrowser {
    /// Enabled, and no address declared yet.
    Waiting,
    /// Enabled at the address the person declared.
    At(UserBrowserEndpoint),
}

/// The address a person's own browser answers on, read once.
///
/// The whole URL is what an attach connects to, and the host is what the
/// attach door judges; both come out of one parse, because two fields
/// derived twice drift. Where the host may point is the door's rule, not
/// this parser's: a grammar that also decided policy would be a second
/// authority for the same question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserBrowserEndpoint {
    url: String,
    host: String,
}

impl UserBrowserEndpoint {
    /// Reads the value of the `usersbrowser` key.
    ///
    /// # Errors
    /// Refuses anything but a `ws://` address carrying a host. The
    /// person's own browser speaks the unencrypted WebSocket locally;
    /// an address over TLS is a different setting, and accepting one
    /// silently would attach somewhere the person did not describe.
    pub fn parse(raw: &str) -> Result<UserBrowserEndpoint, AxError> {
        let trimmed = raw.trim();
        let bad = || {
            AxError::failure(
                AxCode::ConfigInvalid,
                "read a building's rules",
                format!("`usersbrowser = {raw}` is not a browser address"),
            )
            .with_recovery(
                "write `usersbrowser = \"ws://127.0.0.1:<port>/session\"` — the address the browser \
                 prints when it is started with remote debugging — or `usersbrowser = true` to \
                 enable the tool and have it ask for the address, or `usersbrowser = false` to \
                 switch it off",
            )
        };
        if trimmed == "true" {
            return Err(bad());
        }
        if !trimmed.starts_with("ws://") {
            return Err(bad());
        }
        let host = kernel::gate::host_of(trimmed)
            .ok()
            .flatten()
            .ok_or_else(bad)?;
        Ok(UserBrowserEndpoint {
            url: trimmed.to_owned(),
            host,
        })
    }

    /// What the attach connects to.
    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    /// What the attach door classifies.
    #[must_use]
    pub fn host(&self) -> &str {
        &self.host
    }
}
