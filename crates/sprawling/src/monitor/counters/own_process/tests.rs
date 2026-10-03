// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A second reading after busy work shows the CPU it spent.

use std::time::Duration;

use cpu_time::ProcessTime;

use super::{OwnProcess, kib_sum};

#[test]
fn a_busy_process_reads_its_own_cpu_and_memory() {
    let mut own = OwnProcess::new();
    let first = own.read(Duration::ZERO);
    let start = ProcessTime::now();
    let mut spin = 0_u64;
    while start.elapsed() < Duration::from_millis(60) {
        spin = std::hint::black_box(spin.wrapping_add(1));
    }
    let second = own.read(Duration::from_secs(1));

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

/// The rollup's two private lines are summed in bytes; a status file
/// without them reads as absent, so the fallback is taken.
#[test]
fn private_lines_sum_in_bytes_and_absent_lines_read_as_absent() {
    let rollup = "Rss:                3000 kB
Private_Clean:        12 kB
Private_Dirty:       100 kB
Swap:   0 kB
";
    let status = "Name:	sprawling
RssAnon:	    2048 kB
";
    assert_eq!(
        (
            kib_sum(rollup, &["Private_Clean:", "Private_Dirty:"]),
            kib_sum(status, &["RssAnon:"]),
            kib_sum(status, &["Private_Clean:", "Private_Dirty:"]),
        ),
        (Some(112 * 1024), Some(2048 * 1024), None)
    );
}
