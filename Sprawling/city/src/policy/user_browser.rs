// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserBrowser {
    Waiting,

    At(UserBrowserEndpoint),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserBrowserEndpoint {
    url: String,
    host: String,
}

impl UserBrowserEndpoint {
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

    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    #[must_use]
    pub fn host(&self) -> &str {
        &self.host
    }
}
