// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A second reading after busy work shows the CPU it spent.

use std::time::{Duration, Instant};

use super::OwnProcess;

#[test]
fn a_busy_process_reads_its_own_cpu_and_memory() {
    let mut own = OwnProcess::new();
    let first = own.read();
    let start = Instant::now();
    let mut spin = 0_u64;
    while start.elapsed() < Duration::from_millis(60) {
        spin = std::hint::black_box(spin.wrapping_add(1));
    }
    let second = own.read();

    assert_eq!(
        (
            first.cpu_permille,
            second.cpu_permille > 0,
            second.private_bytes > 0,
            second.working_set_bytes > 0,
        ),
        (0, true, true, true)
    );
}
