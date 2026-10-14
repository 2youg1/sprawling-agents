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
/// minute (`crates/sprawling/Spec.lean` §8-62, §8-129-2).
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

/// How long the stand-in endpoint stays silent when nobody releases it:
/// long enough that an accounting thread waiting on it is seen waiting,
/// short enough that a red run ends on its assertion.
const SILENCE: std::time::Duration = std::time::Duration::from_secs(5);

/// A probe waits on its endpoint off the accounting thread: while the
/// endpoint is silent, the next command is still answered, and the
/// probe's reading is written once the endpoint answers.
#[test]
fn a_probe_held_by_a_silent_provider_leaves_the_next_command_answered() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    let (release, held) = std::sync::mpsc::channel::<()>();
    let (said, heard) = std::sync::mpsc::channel();
    // The first request the endpoint reads waits to be released; the
    // ones after it pass, so a released probe finishes at once.
    let silence = std::sync::Mutex::new(Some(held));
    let endpoint_said = said.clone();
    let pace: Pace = std::sync::Arc::new(move |_: &str| {
        if let Some(held) = silence.lock().unwrap().take() {
            drop(held.recv_timeout(SILENCE));
            endpoint_said.send("provider answered").unwrap();
        }
    });
    let (base_url, _provider) = fake_openai_paced(&["m-small"], Vec::new(), Vec::new(), pace);
    let mut worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();

    worker.serve_one(Posted {
        command: wire::Command::ProbeEndpoint {
            name: wire::ProviderName::parse("house").unwrap(),
            base_url,
            dialect: kernel::DialectKind::OpenAi,
            secret: None,
            auth_header: None,
            tuning: wire::EndpointTuning::default(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"probe"),
        },
        reply: wire::Reply::nowhere(),
    });
    worker.serve_one(Posted {
        command: wire::Command::CloseCity(wire::CityClosing {
            mode: wire::CloseMode::Drain,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"close"),
        }),
        reply: wire::Reply::to(move |_| {
            said.send("command answered").unwrap();
            wire::Delivered::ToThePeer
        }),
    });
    // A provider that already stopped waiting has dropped its end.
    drop(release.send(()));
    worker.land_the_rest().unwrap();
    let heard_in_order: Vec<&str> = (0..2)
        .map(|_| heard.recv_timeout(SILENCE).unwrap())
        .collect();

    assert_eq!(
        heard_in_order,
        vec!["command answered", "provider answered"]
    );
    assert_eq!(
        ledger_text(&report.ledger_dir)
            .lines()
            .filter(|line| line.contains("endpoint_probed"))
            .count(),
        1
    );
}
