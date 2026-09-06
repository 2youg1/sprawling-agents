// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::float_arithmetic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]

use super::super::terminal::drive;
use super::super::*;
use kernel::Address;
use std::sync::Arc;
pub(super) fn room() -> Address {
    Address::parse("lab/room1").unwrap()
}

/// A terminal whose facts are synthetic on purpose: nothing here may
/// name a real machine's directories, which `xtask release` refuses.
pub(super) fn terminal(bind: &str, token: Option<&str>) -> Terminal {
    Terminal {
        url: "http://127.0.0.1:8787".to_owned(),
        token: token.map(str::to_owned),
        city: "/tmp/a-city".to_owned(),
        client: "embedded, 3 file(s), 558419 gzipped byte(s)".to_owned(),
        bind: bind.parse().unwrap(),
    }
}
pub(super) fn vitals() -> channels::MetricsAnswer {
    channels::MetricsAnswer {
        events: 12_043,
        runs_active: 2,
        runs_frozen: 41,
        buildings: 7,
        approvals_waiting: 3,
        signals_waiting: 0,
        discards_outstanding: 1,
    }
}

/// The one function the socket calls, standing in for the views.
pub(super) fn answering() -> Answering {
    Arc::new(|query: channels::Query| match query {
        channels::Query::Metrics => Ok(channels::Answer::Metrics(Box::new(vitals()))),
        other => Err(kernel::AxError::failure(
            kernel::AxCode::ConfigInvalid,
            "answer a question",
            format!("{} is not scripted here", other.name()),
        )),
    })
}

/// Runs the console loop over a scripted script and returns what a
/// person would have seen.
pub(super) fn typed(script: &str, terminal: &Terminal) -> String {
    let desk = crate::serving::CommandDesk::new();
    let mut out: Vec<u8> = Vec::new();
    drive(
        terminal,
        &desk,
        &answering(),
        &mut std::io::Cursor::new(script.as_bytes().to_vec()),
        &mut out,
    );
    String::from_utf8(out).unwrap()
}
