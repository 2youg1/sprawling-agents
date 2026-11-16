// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::event::record::ToolkitLinkOpened;
use kernel::{AxCode, AxError, EventKind, Payload};
use wire::ToolkitSlug;

use super::RunWorker;
use crate::toolkit_broker::broker_for;

impl RunWorker {
    pub(in crate::worker) fn connect_toolkit(
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

        self.record(
            EventKind::ToolkitLinkOpened,
            Payload::of(&ToolkitLinkOpened {
                toolkit: toolkit.as_str().to_owned(),
            })?,
        )
    }
}
