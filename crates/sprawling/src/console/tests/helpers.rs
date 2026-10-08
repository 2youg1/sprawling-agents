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
    clippy::wildcard_enum_match_arm,
    clippy::let_underscore_must_use,
    clippy::let_underscore_untyped,
    reason = "test code"
)]

use super::super::terminal::{Inside, drive};
use super::super::*;
use kernel::Address;
use std::sync::Arc;
pub(super) fn key() -> kernel::IdemKey {
    kernel::IdemKey::derive(&kernel::RunId::CITY, kernel::Seq::FIRST, b"a typed line")
}
pub(super) fn room() -> Address {
    Address::parse("lab/room1").unwrap()
}

/// A terminal whose facts are synthetic on purpose: nothing here may
/// name a real machine's directories, which `xtask release` refuses.
pub(super) fn terminal(bind: &str, pairing: Option<&str>) -> Terminal {
    Terminal {
        url: "http://127.0.0.1:8787".to_owned(),
        pairing: pairing.map(str::to_owned),
        city: "/tmp/a-city".to_owned(),
        client: "embedded, 3 file(s), 558419 gzipped byte(s)".to_owned(),
        bind: bind.parse().unwrap(),
        surface: Surface::Headless,
    }
}
pub(super) fn vitals() -> wire::MetricsAnswer {
    wire::MetricsAnswer {
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
    Arc::new(|query: wire::Query| {
        let answer = match query {
            wire::Query::Metrics => Ok(wire::Answer::Metrics(Box::new(vitals()))),
            other => Err(kernel::AxError::failure(
                kernel::AxCode::ConfigInvalid,
                "answer a question",
                format!("{} is not scripted here", other.name()),
            )
            .with_recovery("this test answers Metrics and nothing else")),
        };
        (kernel::Seq::FIRST, answer)
    })
}

/// Runs the console loop over a scripted script and returns what a
/// person would have seen.
pub(super) fn typed(script: &str, terminal: &Terminal) -> String {
    driven(script, terminal, &inside())
}

/// The line console over a script, with what it said collected in order.
pub(super) fn driven(script: &str, terminal: &Terminal, inside: &Inside) -> String {
    let said = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let into = Arc::clone(&said);
    let say: super::super::cli::Say = Arc::new(move |line: String| {
        into.lock().unwrap().push(line);
    });
    drive(
        terminal,
        inside,
        &mut std::io::Cursor::new(script.as_bytes().to_vec()),
        &say,
    );
    said.lock().unwrap().join("\n")
}

/// The console's reach into a city with no remote door.
pub(super) fn inside() -> Inside {
    Inside {
        lifecycle: tokio::sync::mpsc::channel(8).0,
        desk: Arc::new(accounting::worker::CommandDesk::default()),
        answering: answering(),
        remote: Err(kernel::AxError::failure(
            kernel::AxCode::ConfigInvalid,
            "keep the remote door",
            "this test keeps none",
        )
        .with_recovery("nothing to do")),
    }
}
