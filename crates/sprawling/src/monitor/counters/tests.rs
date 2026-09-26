// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A reading of this very process is not empty.

use super::Counters;

#[test]
fn a_reading_of_this_process_has_memory_and_free_space() {
    let mut counters = Counters::open(std::env::temp_dir());
    counters.read(std::time::Duration::ZERO);
    let second = counters.read(std::time::Duration::from_secs(1));

    let nonzero = |value: u64| value > 0;
    assert_eq!(
        (
            nonzero(second.core_working_set_bytes),
            nonzero(second.core_private_bytes),
            nonzero(second.machine_available_bytes),
            nonzero(second.volume_free_bytes),
            second.machine_cpu_permille <= 1000,
            second.core_cpu_permille <= 1000,
        ),
        (true, true, true, true, true, true)
    );
}
