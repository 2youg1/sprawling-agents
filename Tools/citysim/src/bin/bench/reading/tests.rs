// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

fn hundred(at: u64) -> Vec<Duration> {
    vec![Duration::from_micros(at); 100]
}

fn taken() -> Taken {
    Taken {
        machine: MachineClass::General,
        fixture: B3Hash::digest(b"fixture"),
    }
}

fn label() -> String {
    taken().fixture.to_string().chars().take(16).collect()
}

#[test]
fn a_reading_line_is_stable_and_carries_its_machine_class() {
    let reading = Reading::of(
        Load::LargeLedgerFold,
        SubMetric::Harness,
        taken(),
        hundred(10),
    )
    .unwrap();
    assert_eq!(
        reading.line(),
        format!(
            "perf load=large_ledger_fold sub=harness machine_class=general fixture={} \
             samples=100 floor_us=10 p50_us=10 p95_us=10 p99_us=10",
            label()
        )
    );
}

#[test]
fn a_reading_line_carries_its_floor_beside_the_middle() {
    let reading = Reading::of(
        Load::LongSessionForwarding,
        SubMetric::Harness,
        taken(),
        (1..=100).rev().map(Duration::from_micros).collect(),
    )
    .unwrap();
    assert_eq!(
        reading.line(),
        format!(
            "perf load=long_session_forwarding sub=harness machine_class=general fixture={} \
             samples=100 floor_us=1 p50_us=50 p95_us=95 p99_us=99",
            label()
        )
    );
}

#[test]
fn a_reading_with_no_samples_is_refused_rather_than_printed_as_zero() {
    let refused = Reading::of(
        Load::LargeLedgerFold,
        SubMetric::Harness,
        taken(),
        Vec::new(),
    )
    .unwrap_err();
    assert!(refused.contains("sampled nothing"), "{refused}");
}
