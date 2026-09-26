// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Connecting an outside application: the command behind the page's
//! button. What the page and this command both need to know about the
//! broker is `crate::toolkit_broker`'s.

use channels::ToolkitSlug;
use kernel::event::record::ToolkitLinkOpened;
use kernel::{AxCode, AxError, EventKind, Payload};

use super::RunWorker;
use crate::toolkit_broker::broker_for;

impl RunWorker {
    /// Opens a consent session for one outside application.
    ///
    /// **What is recorded is the request, never the standing and never
    /// the consent url.** Where an application stands is a fact about
    /// now and belongs to the broker (channels-SPEC.md section 8-31); a
    /// consent url is a capability, and anybody replaying this log would
    /// be holding one. The page reads both back from the broker with
    /// `Query::Toolkits` immediately afterwards.
    ///
    /// # Errors
    /// A city with no project key enrolled, a vault that cannot answer,
    /// a broker that refuses, and a payload the ledger will not take.
    pub(in crate::assembly) fn connect_toolkit(
        &mut self,
        toolkit: &ToolkitSlug,
    ) -> Result<(), AxError> {
        let city = kernel::layout::CityLayout::new(&self.city_root).city_address();
        let held = broker_for(Some(&self.credentials.vault), city.as_ref())?.ok_or_else(|| {
            AxError::failure(
                AxCode::CredentialMissing,
                "connect an outside application",
                "this city holds no project key for the broker",
            )
            .with_recovery("store the broker's project key on the MCP page, then try again")
        })?;
        let (broker, user) = held;
        broker.connect(toolkit.as_str(), &user)?;
        // The slug and nothing else: which application a person asked for
        // is the fact worth keeping, and the consent URL is a capability.
        self.record(
            EventKind::ToolkitLinkOpened,
            Payload::of(&ToolkitLinkOpened {
                toolkit: toolkit.as_str().to_owned(),
            })?,
        )
    }
}
