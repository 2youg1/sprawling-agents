// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A reading of this very process is not empty, and a beat reads the
//! platform exactly as often as its watchers ask.

use super::{Counters, Reads, Watched};
use crate::monitor::Monitor;

#[test]
fn a_reading_of_this_process_has_memory_and_free_space() {
    let mut counters = Counters::open(std::env::temp_dir());
    counters.read(Watched::Everything, std::time::Duration::ZERO);
    let second = counters.read(Watched::Everything, std::time::Duration::from_secs(1));

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

/// A city served from a relative path lives on the disk that holds its
/// absolute spelling; no mount point is a prefix of `.` itself.
#[test]
fn a_city_opened_by_a_relative_path_reads_its_free_space() {
    let mut counters = Counters::open(std::path::PathBuf::from("."));
    let sample = counters.read(Watched::Everything, std::time::Duration::ZERO);
    assert!(sample.volume_free_bytes > 0, "{sample:?}");
}

/// The regression gate of sprawling-SPEC.md 8-129-3: a count, not a
/// time, so it holds on a loaded runner as on a quiet desk. Nobody
/// watching reads nothing; a summary reads this process once a beat; a
/// whole page reads this process and the machine once a beat each; the
/// city's sampler never reads the process table.
#[test]
fn a_beat_reads_the_platform_exactly_as_often_as_its_watchers_ask() {
    const BEATS: u64 = 3;
    let reads_after = |watched: Option<Watched>| {
        let mut monitor = Monitor::new();
        let watch = watched.map(|watched| monitor.watch(watched));
        let mut counters = Counters::open(std::env::temp_dir());
        for _ in 0..BEATS {
            monitor.tick(|watched| counters.read(watched, std::time::Duration::from_secs(1)));
        }
        drop(watch);
        counters.reads()
    };
    let reads = |own, machine| Reads {
        own,
        machine,
        table: 0,
    };
    assert_eq!(
        [None, Some(Watched::Summary), Some(Watched::Everything)].map(reads_after),
        [reads(0, 0), reads(BEATS, 0), reads(BEATS, BEATS)]
    );
}
