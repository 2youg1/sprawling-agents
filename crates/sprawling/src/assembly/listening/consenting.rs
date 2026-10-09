// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! `AddAgent` taken at the served city's listener
//! (`crates/sprawling/spec/Serving.lean`): the offers the views answered
//! and this machine's search path are both here, and neither is in the
//! run worker.

use std::path::PathBuf;
use std::sync::Arc;

use accounting::offered::Offered;

use super::doorstep::Commands;

/// The command sink with `AddAgent` carried out here, against the offers
/// in `offered`, its program found on the search path the harness page
/// reads; every other command goes on to `rest`.
pub(super) fn taking(city_root: PathBuf, offered: Offered, rest: Commands) -> Commands {
    Arc::new(move |command, reply| {
        let wire::Command::AddAgent(adding) = &command else {
            return rest(command, reply);
        };
        offered.consent(&city_root, adding, crate::doctor::host::find_program)
    })
}
