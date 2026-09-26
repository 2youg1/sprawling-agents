// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One injected reading per degradation: each test starts from a calm
//! sample, pushes exactly one resource past its line, and expects exactly
//! that degradation with its readings and recovery.

use super::*;

const GIB: u64 = 1024 * 1024 * 1024;

fn calm() -> ResourceReadings {
    ResourceReadings {
        durable_lag: Duration::from_micros(300),
        commit_floor: Duration::from_micros(200),
        volume: VolumeSpace {
            free_bytes: 400 * GIB,
            total_bytes: 1000 * GIB,
        },
        queued_runs: 0,
        schedule_delay: Duration::from_micros(40),
    }
}

fn verdict(readings: &ResourceReadings) -> Vec<(Degradation, Recovery)> {
    assess(readings).map(|d| (d, d.recovery())).collect()
}

#[test]
fn calm_readings_show_no_degradation_and_admit_work() {
    assert_eq!(verdict(&calm()), vec![]);
    assert_eq!(admit_work(calm().volume), Ok(()));
}

#[test]
fn a_disk_below_its_floor_is_low_and_stops_new_work() {
    let volume = VolumeSpace {
        free_bytes: GIB,
        total_bytes: 1000 * GIB,
    };
    let readings = ResourceReadings { volume, ..calm() };
    assert_eq!(
        verdict(&readings),
        vec![(
            Degradation::DiskLow {
                free_bytes: GIB,
                floor_bytes: 4 * GIB,
            },
            Recovery::FreeDiskSpace {
                at_least_bytes: 3 * GIB,
            },
        )]
    );
    assert_eq!(
        admit_work(volume),
        Err(Degradation::DiskLow {
            free_bytes: GIB,
            floor_bytes: 4 * GIB,
        })
    );
}

#[test]
fn a_watermark_lagging_many_commit_floors_is_a_slow_disk_that_still_admits() {
    let readings = ResourceReadings {
        durable_lag: Duration::from_millis(40),
        ..calm()
    };
    assert_eq!(
        verdict(&readings),
        vec![(
            Degradation::DiskSlow {
                durable_lag: Duration::from_millis(40),
                commit_floor: Duration::from_micros(200),
            },
            Recovery::ReduceDiskLoad,
        )]
    );
    assert_eq!(admit_work(readings.volume), Ok(()));
}

#[test]
fn a_spinning_disk_whose_fsync_is_slow_is_not_degraded() {
    let readings = ResourceReadings {
        durable_lag: Duration::from_millis(40),
        commit_floor: Duration::from_millis(12),
        ..calm()
    };
    assert_eq!(verdict(&readings), vec![]);
}

#[test]
fn a_queued_run_is_tight_memory_that_still_admits() {
    let readings = ResourceReadings {
        queued_runs: 2,
        ..calm()
    };
    assert_eq!(
        verdict(&readings),
        vec![(
            Degradation::MemoryTight { queued_runs: 2 },
            Recovery::FreeMemory,
        )]
    );
    assert_eq!(admit_work(readings.volume), Ok(()));
}

#[test]
fn a_scheduling_delay_past_one_frame_is_a_saturated_cpu_that_still_admits() {
    let readings = ResourceReadings {
        schedule_delay: Duration::from_millis(30),
        ..calm()
    };
    assert_eq!(
        verdict(&readings),
        vec![(
            Degradation::CpuSaturated {
                schedule_delay: Duration::from_millis(30),
            },
            Recovery::ReduceCpuLoad,
        )]
    );
    assert_eq!(admit_work(readings.volume), Ok(()));
}

#[test]
fn every_bottleneck_at_once_is_reported_in_declaration_order() {
    let readings = ResourceReadings {
        durable_lag: Duration::from_secs(1),
        volume: VolumeSpace {
            free_bytes: 0,
            total_bytes: 0,
        },
        queued_runs: 1,
        schedule_delay: Duration::from_secs(1),
        ..calm()
    };
    let kinds: Vec<_> = assess(&readings)
        .map(|d| std::mem::discriminant(&d))
        .collect();
    let expected: Vec<_> = [
        Degradation::DiskSlow {
            durable_lag: Duration::ZERO,
            commit_floor: Duration::ZERO,
        },
        Degradation::DiskLow {
            free_bytes: 0,
            floor_bytes: 0,
        },
        Degradation::MemoryTight { queued_runs: 0 },
        Degradation::CpuSaturated {
            schedule_delay: Duration::ZERO,
        },
    ]
    .iter()
    .map(std::mem::discriminant)
    .collect();
    assert_eq!(kinds, expected);
}

#[test]
fn the_floor_is_a_share_of_the_volume_clamped_at_both_ends() {
    assert_eq!(free_space_floor(0), FREE_SPACE_FLOOR_MIN);
    assert_eq!(free_space_floor(100 * GIB), 2 * GIB);
    assert_eq!(free_space_floor(u64::MAX), FREE_SPACE_FLOOR_MAX);
}
