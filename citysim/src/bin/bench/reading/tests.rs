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
         samples=100 p50_us=10 p95_us=10 p99_us=10"
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
