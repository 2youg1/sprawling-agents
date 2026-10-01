// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! How long a probe took is read off the monotonic clock, whatever the
//! city's clock does meanwhile.

use super::endpoints::ledger_text;
use crate::worker::fixture::*;
use crate::worker::*;

/// How long a probe took is a span, read off the monotonic clock: a
/// city clock that leaps a minute at every read leaves it well under a
/// minute (sprawling-SPEC.md 8-62, 8-129-2).
#[test]
fn a_probe_times_itself_on_the_monotonic_clock() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    let (base_url, _provider) = fake_openai(&["m-small"], Vec::new());
    let mut worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        Hands {
            clock: std::sync::Arc::new(LeapingClock::from_now()),
            ..crate::worker::fixture::hands()
        },
    )
    .unwrap();
    worker
        .handle(wire::Command::ProbeEndpoint {
            name: wire::ProviderName::parse("house").unwrap(),
            base_url,
            dialect: kernel::DialectKind::OpenAi,
            secret: None,
            auth_header: None,
            tuning: wire::EndpointTuning::default(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"probe"),
        })
        .unwrap();
    let probed: Vec<u64> = ledger_text(&report.ledger_dir)
        .lines()
        .filter(|line| line.contains("endpoint_probed"))
        .filter_map(|line| serde_json::from_str(line).ok())
        .filter_map(|record: serde_json::Value| elapsed_ms(&record))
        .collect();
    assert_eq!(
        probed
            .iter()
            .map(|spent| *spent < LeapingClock::LEAP_MS)
            .collect::<Vec<_>>(),
        vec![true],
        "{probed:?}"
    );
}

/// The first `elapsed_ms` anywhere in a record.
fn elapsed_ms(value: &serde_json::Value) -> Option<u64> {
    match value {
        serde_json::Value::Object(map) => map
            .get("elapsed_ms")
            .and_then(serde_json::Value::as_u64)
            .or_else(|| map.values().find_map(elapsed_ms)),
        serde_json::Value::Array(items) => items.iter().find_map(elapsed_ms),
        serde_json::Value::Null
        | serde_json::Value::Bool(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::String(_) => None,
    }
}
