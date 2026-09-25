// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The reading line's stable format, and the machine-class field every
//! reading carries.

use super::*;

fn hundred(at: u64) -> Vec<Duration> {
    vec![Duration::from_micros(at); 100]
}

#[test]
fn a_reading_line_is_stable_and_carries_its_machine_class() {
    let reading = Reading::of(
        Load::MultiRunParallel,
        SubMetric::Harness,
        MachineClass::General,
        hundred(10),
    )
    .unwrap();
    assert_eq!(
        reading.line(),
        "perf load=multi_run_parallel sub=harness machine_class=general \
         samples=100 floor_us=10 p50_us=10 p95_us=10 p99_us=10"
    );
}

/// The floor is what the path costs on a quiet machine and the middle
/// is what it cost beside everything else the machine did, so a line
/// that drops the first cannot tell a slower design from a busier day.
#[test]
fn a_reading_line_carries_its_floor_beside_the_middle() {
    let reading = Reading::of(
        Load::LongSessionForwarding,
        SubMetric::Harness,
        MachineClass::General,
        (1..=100).rev().map(Duration::from_micros).collect(),
    )
    .unwrap();
    assert_eq!(
        reading.line(),
        "perf load=long_session_forwarding sub=harness machine_class=general \
         samples=100 floor_us=1 p50_us=51 p95_us=96 p99_us=100"
    );
}

#[test]
fn a_reading_with_no_samples_is_refused_rather_than_printed_as_zero() {
    let refused = Reading::of(
        Load::LargeLedgerFold,
        SubMetric::Harness,
        MachineClass::General,
        Vec::new(),
    )
    .unwrap_err();
    assert!(refused.contains("sampled nothing"), "{refused}");
}
