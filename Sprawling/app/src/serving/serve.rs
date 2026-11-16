// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::net::SocketAddr;

use kernel::Payload;

pub struct Serving {
    pub city_root: std::path::PathBuf,
    pub addr: SocketAddr,

    pub token: String,
    pub client: wire::ClientAssets,
    pub vault: gateway::Custodian,
    pub vault_notice: Option<Payload>,
    pub log: runtime::diagnostics::Diagnostics,

    pub journal: crate::serving::Journal,

    pub core: accounting::person::CorePriority,
}
