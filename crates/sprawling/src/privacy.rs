// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Local privacy history modules (`crates/sprawling/spec/Privacy.lean`).

pub mod cli;
mod controls;
#[cfg_attr(
    not(any(test, windows)),
    expect(
        dead_code,
        reason = "the privacy write verbs run the coordinator on Windows only"
    )
)]
mod coordinator;
#[cfg(windows)]
mod elevation;
#[cfg_attr(
    not(any(test, windows)),
    expect(
        dead_code,
        reason = "the privacy write verbs run the coordinator on Windows only"
    )
)]
mod fault;
mod identity;
mod journal;
mod originals;
#[cfg_attr(
    not(any(test, windows)),
    expect(
        dead_code,
        reason = "the privacy write verbs run the coordinator on Windows only"
    )
)]
mod plan;
mod state;
mod target;
#[cfg(windows)]
pub(crate) mod windows;
