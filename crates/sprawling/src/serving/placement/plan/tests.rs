// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The properties of `crates/sprawling/spec/Serving/Placement/Plan.lean`
//! checked on generated topologies, then the plan for described processor
//! families. Each family is built from the vendor's published core,
//! thread and cache counts, numbered the way Windows numbers a hybrid
//! part (the faster cores' threads first, a core's SMT siblings
//! adjacent); none of them was read from the machine itself.

#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use std::collections::BTreeSet;

use proptest::prelude::*;

use super::{Access, Cpu, Left, Plan, Processor, Topology, plan};

/// One generated physical core: its class, its threads, its cache, and
/// which of its threads the process may use.
#[derive(Debug, Clone)]
struct Drawn {
    class: u8,
    threads: u32,
    cache: u32,
    barred: Vec<bool>,
}

fn drawn() -> impl Strategy<Value = Drawn> {
    (0u8..4, 1u32..3, 0u32..3, proptest::collection::vec(any::<bool>(), 2))
        .prop_map(|(class, threads, cache, barred)| Drawn { class, threads, cache, barred })
}

/// A topology of up to ten cores in up to two groups, each core's threads
/// numbered next to each other, sometimes spoiled by a repeated processor
/// or a core reported in a second class, then shuffled.
fn topology() -> impl Strategy<Value = Vec<Cpu>> {
    (proptest::collection::vec(drawn(), 0..10), 0u8..8)
        .prop_map(|(cores, spoil)| {
            let mut cpus = Vec::new();
            for (core, d) in (0u32..).zip(&cores) {
                for thread in 0..d.threads {
                    let index = usize::try_from(thread).unwrap();
                    cpus.push(Cpu {
                        processor: Processor {
                            group: u16::from(core >= 6),
                            number: core * 2 + thread,
                        },
                        class: d.class,
                        core,
                        cache: d.cache,
                        access: if d.barred[index] { Access::Barred } else { Access::Allowed },
                    });
                }
            }
            match (spoil, cpus.first().copied()) {
                (0, Some(first)) => cpus.push(first),
                (1, Some(first)) => cpus.push(Cpu {
                    processor: Processor { group: first.processor.group, number: 999 },
                    class: first.class.wrapping_add(1),
                    ..first
                }),
                _ => {}
            }
            cpus
        })
        .prop_shuffle()
}

fn seats(plan: &Plan) -> &[Processor] {
    match plan {
        Plan::Seats(seats) => seats,
        Plan::LeftToOs(_) => &[],
    }
}

fn usable(cpus: &[Cpu]) -> impl Iterator<Item = &Cpu> {
    cpus.iter().filter(|cpu| cpu.access == Access::Allowed)
}

fn consistent(cpus: &[Cpu]) -> bool {
    let ids: BTreeSet<Processor> = cpus.iter().map(|cpu| cpu.processor).collect();
    ids.len() == cpus.len()
        && cpus.iter().all(|a| {
            cpus.iter()
                .all(|b| (a.processor.group, a.core) != (b.processor.group, b.core) || a.class == b.class)
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2_000))]

    #[test]
    fn the_plan_keeps_every_property_of_the_model(cpus in topology()) {
        let got = plan(&Topology::new(cpus.clone()));
        let seats = seats(&got);
        let classes: BTreeSet<u8> = usable(&cpus).map(|cpu| cpu.class).collect();
        let seated: Vec<&Cpu> = seats
            .iter()
            .map(|seat| cpus.iter().find(|cpu| cpu.processor == *seat).unwrap())
            .collect();

        // a_planned_cpu_is_one_the_process_may_use
        prop_assert!(seated.iter().all(|cpu| cpu.access == Access::Allowed));
        // an_inconsistent_topology_gives_no_plan
        if !consistent(&cpus) {
            prop_assert_eq!(&got, &Plan::LeftToOs(Left::Inconsistent));
        }
        // one_class_gives_no_plan
        if classes.len() < 2 {
            prop_assert!(seats.is_empty());
        }
        // one_seat_per_physical_core
        let cores: BTreeSet<(u16, u32)> =
            seated.iter().map(|cpu| (cpu.processor.group, cpu.core)).collect();
        prop_assert_eq!(cores.len(), seats.len());
        // a_planned_cpu_is_never_of_the_slowest_class
        if let Some(slowest) = classes.first() {
            prop_assert!(seated.iter().all(|cpu| cpu.class != *slowest));
        }
        // The plan seats every fastest-class core once, so seats equal
        // the fastest class's physical cores rather than only bounding them.
        if consistent(&cpus) && classes.len() >= 2 {
            let fastest = *classes.last().unwrap();
            let top: BTreeSet<(u16, u32)> = usable(&cpus)
                .filter(|cpu| cpu.class == fastest)
                .map(|cpu| (cpu.processor.group, cpu.core))
                .collect();
            prop_assert_eq!(seats.len(), top.len());
        }
        // Seats arrive sorted, and Seats is never empty.
        prop_assert!(seats.windows(2).all(|pair| pair[0] < pair[1]));
        prop_assert!(!matches!(&got, Plan::Seats(seats) if seats.is_empty()));
    }

    /// the_plan_ignores_the_order_it_is_read_in and the_plan_never_reads_the_cache.
    #[test]
    fn the_plan_ignores_order_and_caches(cpus in topology(), cache in 0u32..5) {
        let got = plan(&Topology::new(cpus.clone()));
        let mut reversed = cpus.clone();
        reversed.reverse();
        prop_assert_eq!(&plan(&Topology::new(reversed)), &got);
        let recached: Vec<Cpu> = cpus.iter().map(|cpu| Cpu { cache, ..*cpu }).collect();
        prop_assert_eq!(&plan(&Topology::new(recached)), &got);
    }
}

/// A run of physical cores of one class: `cores` cores of `threads`
/// threads each, sharing the last-level cache `cache`.
struct Run {
    class: u8,
    cores: u32,
    threads: u32,
    cache: u32,
}

/// A part as Windows numbers it in one group: the runs in the order
/// given, each core's threads adjacent.
fn part(runs: &[Run]) -> Vec<Cpu> {
    let mut cpus = Vec::new();
    let mut number = 0u32;
    let mut core = 0u32;
    for run in runs {
        for _ in 0..run.cores {
            for _ in 0..run.threads {
                cpus.push(Cpu {
                    processor: Processor { group: 0, number },
                    class: run.class,
                    core,
                    cache: run.cache,
                    access: Access::Allowed,
                });
                number += 1;
            }
            core += run.threads;
        }
    }
    cpus
}

fn numbers(numbers: &[u32]) -> Plan {
    Plan::Seats(numbers.iter().map(|&number| Processor { group: 0, number }).collect())
}

fn even(below: u32) -> Vec<u32> {
    (0..below).step_by(2).collect()
}

/// i5-1340P (Raptor Lake-P): 4 P-cores with SMT, 8 E-cores, one L3.
fn i5_1340p() -> Vec<Cpu> {
    part(&[
        Run { class: 1, cores: 4, threads: 2, cache: 0 },
        Run { class: 0, cores: 8, threads: 1, cache: 0 },
    ])
}

#[test]
fn a_raptor_lake_laptop_seats_one_thread_per_performance_core() {
    assert_eq!(plan(&Topology::new(i5_1340p())), numbers(&even(8)));
}

#[test]
fn a_raptor_lake_desktop_seats_its_eight_performance_cores() {
    // i9-13900K: 8 P-cores with SMT, 16 E-cores.
    let cpus = part(&[
        Run { class: 1, cores: 8, threads: 2, cache: 0 },
        Run { class: 0, cores: 16, threads: 1, cache: 0 },
    ]);
    assert_eq!(plan(&Topology::new(cpus)), numbers(&even(16)));
}

#[test]
fn a_meteor_lake_laptop_seats_performance_cores_and_never_the_low_power_tier() {
    // Core Ultra 7 155H: 6 P-cores with SMT, 8 E-cores, 2 low-power E-cores
    // on the SoC tile with a cache of their own: three classes.
    let cpus = part(&[
        Run { class: 2, cores: 6, threads: 2, cache: 0 },
        Run { class: 1, cores: 8, threads: 1, cache: 0 },
        Run { class: 0, cores: 2, threads: 1, cache: 1 },
    ]);
    assert_eq!(plan(&Topology::new(cpus)), numbers(&even(12)));
}

#[test]
fn a_meteor_lake_container_without_performance_cores_prefers_the_e_cores_not_the_low_power_ones() {
    let cpus: Vec<Cpu> = part(&[
        Run { class: 2, cores: 6, threads: 2, cache: 0 },
        Run { class: 1, cores: 8, threads: 1, cache: 0 },
        Run { class: 0, cores: 2, threads: 1, cache: 1 },
    ])
    .into_iter()
    .map(|cpu| Cpu {
        access: if cpu.class == 2 { Access::Barred } else { Access::Allowed },
        ..cpu
    })
    .collect();
    assert_eq!(plan(&Topology::new(cpus)), numbers(&(12..20).collect::<Vec<_>>()));
}

#[test]
fn a_lunar_lake_laptop_seats_its_four_performance_cores() {
    // Core Ultra 7 258V: 4 P-cores without SMT, 4 low-power E-cores.
    let cpus = part(&[
        Run { class: 1, cores: 4, threads: 1, cache: 0 },
        Run { class: 0, cores: 4, threads: 1, cache: 1 },
    ]);
    assert_eq!(plan(&Topology::new(cpus)), numbers(&[0, 1, 2, 3]));
}

#[test]
fn a_single_ccd_x3d_is_left_to_the_operating_system() {
    // Ryzen 7 7800X3D: 8 cores with SMT, one CCD, one L3.
    let cpus = part(&[Run { class: 0, cores: 8, threads: 2, cache: 0 }]);
    assert_eq!(plan(&Topology::new(cpus)), Plan::LeftToOs(Left::OneClass));
}

#[test]
fn a_dual_ccd_x3d_is_left_to_the_operating_system_and_its_cache_steering() {
    // Ryzen 9 7950X3D and 9950X3D: 16 cores with SMT, two CCDs, one class,
    // two L3 sizes.
    let topology = Topology::new(part(&[
        Run { class: 0, cores: 8, threads: 2, cache: 0 },
        Run { class: 0, cores: 8, threads: 2, cache: 1 },
    ]));
    assert_eq!(plan(&topology), Plan::LeftToOs(Left::OneClass));
    assert_eq!(topology.shape().caches, 2);
}

#[test]
fn a_zen_5_and_zen_5c_laptop_seats_its_four_zen_5_cores() {
    // Ryzen AI 9 HX 370: 4 Zen 5 and 8 Zen 5c cores, all with SMT, each
    // kind on a complex with its own L3.
    let cpus = part(&[
        Run { class: 1, cores: 4, threads: 2, cache: 0 },
        Run { class: 0, cores: 8, threads: 2, cache: 1 },
    ]);
    assert_eq!(plan(&Topology::new(cpus)), numbers(&even(8)));
}

#[test]
fn a_threadripper_in_two_groups_is_left_to_the_operating_system() {
    // Threadripper 7980X: 64 cores with SMT, 128 logical processors in two
    // groups of 64; one class. The reading thread may use group 0 only.
    let cpus: Vec<Cpu> = part(&[Run { class: 0, cores: 64, threads: 2, cache: 0 }])
        .into_iter()
        .map(|cpu| {
            let group = u16::from(cpu.processor.number >= 64);
            Cpu {
                processor: Processor { group, number: cpu.processor.number % 64 },
                core: cpu.core % 64,
                cache: cpu.processor.number / 16,
                access: if group == 0 { Access::Allowed } else { Access::Barred },
                ..cpu
            }
        })
        .collect();
    let topology = Topology::new(cpus);
    assert_eq!(plan(&topology), Plan::LeftToOs(Left::OneClass));
    assert_eq!((topology.shape().usable, topology.shape().logical), (64, 128));
}

#[test]
fn a_snapdragon_x_with_one_class_is_left_to_the_operating_system() {
    // Snapdragon X Elite X1E-78-100: 12 Oryon cores in three clusters.
    let cpus = part(&[
        Run { class: 0, cores: 4, threads: 1, cache: 0 },
        Run { class: 0, cores: 4, threads: 1, cache: 1 },
        Run { class: 0, cores: 4, threads: 1, cache: 2 },
    ]);
    assert_eq!(plan(&Topology::new(cpus)), Plan::LeftToOs(Left::OneClass));
}

#[test]
fn a_four_vcpu_virtual_machine_is_left_to_the_operating_system() {
    let cpus = part(&[Run { class: 0, cores: 4, threads: 1, cache: 0 }]);
    assert_eq!(plan(&Topology::new(cpus)), Plan::LeftToOs(Left::OneClass));
}

#[test]
fn a_virtual_machine_that_reports_one_core_in_two_classes_is_left_to_the_operating_system() {
    let mut cpus = part(&[
        Run { class: 1, cores: 2, threads: 1, cache: 0 },
        Run { class: 0, cores: 2, threads: 1, cache: 0 },
    ]);
    cpus[3].core = cpus[0].core;
    assert_eq!(plan(&Topology::new(cpus)), Plan::LeftToOs(Left::Inconsistent));
}

#[test]
fn a_container_limited_to_two_e_cores_is_left_to_the_operating_system() {
    let cpus: Vec<Cpu> = i5_1340p()
        .into_iter()
        .map(|cpu| Cpu {
            access: if matches!(cpu.processor.number, 8 | 9) { Access::Allowed } else { Access::Barred },
            ..cpu
        })
        .collect();
    assert_eq!(plan(&Topology::new(cpus)), Plan::LeftToOs(Left::OneClass));
}

#[test]
fn a_container_limited_to_one_p_core_and_one_e_core_seats_the_p_core_it_may_use() {
    // Allowed: the second thread of P-core 1 and E-core 8; the plan names
    // the thread it may use, not the core's first.
    let cpus: Vec<Cpu> = i5_1340p()
        .into_iter()
        .map(|cpu| Cpu {
            access: if matches!(cpu.processor.number, 3 | 8) { Access::Allowed } else { Access::Barred },
            ..cpu
        })
        .collect();
    assert_eq!(plan(&Topology::new(cpus)), numbers(&[3]));
}

#[test]
fn a_process_that_may_use_nothing_is_left_to_the_operating_system() {
    let cpus: Vec<Cpu> = i5_1340p()
        .into_iter()
        .map(|cpu| Cpu { access: Access::Barred, ..cpu })
        .collect();
    assert_eq!(plan(&Topology::new(cpus)), Plan::LeftToOs(Left::NothingUsable));
}
