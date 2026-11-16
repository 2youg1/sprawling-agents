// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The processor topology as each platform reports it, never assumed
//! (`crates/sprawling/spec/Serving/Placement.lean` D45): CPU sets and the
//! reading thread's group on Windows, `/proc` and sysfs on Linux, `sysctl`
//! on macOS. The translation from each platform's words to a [`Topology`]
//! is a pure function beside its reader, so a reading captured on one
//! machine is checked on every platform.

use std::collections::{BTreeMap, BTreeSet};

use super::plan::{Access, Cpu, Processor, Topology};

/// Why no topology was read, in words for the doctor and standard error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Unread(pub(crate) String);

/// This machine's topology.
#[cfg(windows)]
pub(crate) fn read() -> Result<Topology, Unread> {
    let sets = desktop_ffi::cpu::sets()
        .map_err(|err| Unread(format!("Windows did not list the CPU sets ({err:?})")))?;
    let group = desktop_ffi::cpu::thread_group().map_err(|err| {
        Unread(format!(
            "Windows did not name this thread's processor group ({err:?})"
        ))
    })?;
    Ok(from_cpu_sets(&sets, group.group, group.mask))
}

/// This machine's topology.
#[cfg(target_os = "linux")]
pub(crate) fn read() -> Result<Topology, Unread> {
    linux::read()
}

/// This machine's topology.
#[cfg(target_os = "macos")]
pub(crate) fn read() -> Result<Topology, Unread> {
    macos::read()
}

/// This machine's topology.
#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
pub(crate) fn read() -> Result<Topology, Unread> {
    Err(Unread(
        "this platform reports no processor topology".to_owned(),
    ))
}

/// Windows' CPU sets as a topology: the process may use a processor of
/// the reading thread's `group` whose bit is in `mask`, unless another
/// process holds it for its exclusive use. An ideal processor is named
/// inside the thread's group, so the other groups are barred.
#[cfg_attr(
    not(any(windows, test)),
    expect(dead_code, reason = "read on Windows, tested everywhere")
)]
pub(crate) fn from_cpu_sets(
    sets: &[desktop_ffi::cpu_set::CpuSet],
    group: u16,
    mask: u64,
) -> Topology {
    Topology::new(
        sets.iter()
            .map(|set| {
                let in_mask = 1u64
                    .checked_shl(u32::from(set.logical))
                    .is_some_and(|bit| mask & bit != 0);
                let held_elsewhere =
                    set.flags.allocated() && !set.flags.allocated_to_this_process();
                Cpu {
                    processor: Processor {
                        group: set.group,
                        number: u32::from(set.logical),
                    },
                    class: set.class,
                    core: u32::from(set.core),
                    cache: u32::from(set.cache),
                    access: if set.group == group && in_mask && !held_elsewhere {
                        Access::Allowed
                    } else {
                        Access::Barred
                    },
                }
            })
            .collect(),
    )
}

/// A Linux CPU list such as `0-3,8,10-11`, or `None` when it does not parse.
#[cfg_attr(
    not(any(target_os = "linux", test)),
    expect(dead_code, reason = "read on Linux, tested everywhere")
)]
pub(crate) fn cpu_list(text: &str) -> Option<BTreeSet<u32>> {
    let mut cpus = BTreeSet::new();
    for part in text.trim().split(',').filter(|part| !part.is_empty()) {
        let (first, last): (u32, u32) = match part.split_once('-') {
            Some((first, last)) => (first.parse().ok()?, last.parse().ok()?),
            None => {
                let only = part.parse().ok()?;
                (only, only)
            }
        };
        if first > last {
            return None;
        }
        cpus.extend(first..=last);
    }
    Some(cpus)
}

/// What sysfs says of one Linux CPU.
#[cfg_attr(
    not(any(target_os = "linux", test)),
    expect(dead_code, reason = "read on Linux, tested everywhere")
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LinuxCpu {
    pub(crate) number: u32,
    pub(crate) package: u32,
    pub(crate) core: u32,
    pub(crate) cache: u32,
    pub(crate) capacity: Option<u32>,
}

/// Linux's words as a topology. `hybrid` is Intel's split into
/// `cpu_core` and `cpu_atom` when the kernel offers it; otherwise each
/// distinct `cpu_capacity` is a class, and with neither there is one.
#[cfg_attr(
    not(any(target_os = "linux", test)),
    expect(dead_code, reason = "read on Linux, tested everywhere")
)]
pub(crate) fn from_sysfs(
    cpus: &[LinuxCpu],
    allowed: &BTreeSet<u32>,
    hybrid: Option<&BTreeSet<u32>>,
) -> Option<Topology> {
    let capacities: BTreeSet<u32> = cpus.iter().filter_map(|cpu| cpu.capacity).collect();
    let rank: BTreeMap<u32, u8> = capacities
        .iter()
        .zip(0u8..)
        .map(|(&capacity, class)| (capacity, class))
        .collect();
    let mut read = Vec::with_capacity(cpus.len());
    for cpu in cpus {
        let class = match (hybrid, cpu.capacity) {
            (Some(big), _) => u8::from(big.contains(&cpu.number)),
            (None, Some(capacity)) => *rank.get(&capacity)?,
            (None, None) => 0,
        };
        read.push(Cpu {
            processor: Processor {
                group: 0,
                number: cpu.number,
            },
            class,
            core: cpu.package.checked_mul(1 << 16)?.checked_add(cpu.core)?,
            cache: cpu.cache,
            access: if allowed.contains(&cpu.number) {
                Access::Allowed
            } else {
                Access::Barred
            },
        });
    }
    Some(Topology::new(read))
}

/// One macOS performance level: `perflevel0` is the fastest.
#[cfg_attr(
    not(any(target_os = "macos", test)),
    expect(dead_code, reason = "read on macOS, tested everywhere")
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PerfLevel {
    pub(crate) physical: u32,
    pub(crate) logical: u32,
}

/// macOS's performance levels as a topology, fastest level first, each
/// core's threads numbered next to each other.
#[cfg_attr(
    not(any(target_os = "macos", test)),
    expect(dead_code, reason = "read on macOS, tested everywhere")
)]
pub(crate) fn from_perf_levels(levels: &[PerfLevel]) -> Option<Topology> {
    let count = u8::try_from(levels.len()).ok()?;
    let mut cpus = Vec::new();
    let mut number = 0u32;
    let mut core = 0u32;
    for (level, index) in levels.iter().zip(0u8..) {
        let threads = level.logical.checked_div(level.physical)?.max(1);
        for _ in 0..level.physical {
            for _ in 0..threads {
                cpus.push(Cpu {
                    processor: Processor { group: 0, number },
                    class: count.checked_sub(index)?.checked_sub(1)?,
                    core,
                    cache: u32::from(index),
                    access: Access::Allowed,
                });
                number = number.checked_add(1)?;
            }
            core = core.checked_add(1)?;
        }
    }
    Some(Topology::new(cpus))
}

#[cfg(target_os = "linux")]
mod linux {
    use std::collections::BTreeSet;
    use std::path::Path;

    use super::{LinuxCpu, Topology, Unread, cpu_list, from_sysfs};

    const CPUS: &str = "/sys/devices/system/cpu";

    pub(super) fn read() -> Result<Topology, Unread> {
        let allowed = allowed()?;
        let present = text(&format!("{CPUS}/present"))
            .and_then(|list| cpu_list(&list))
            .ok_or_else(|| Unread(format!("{CPUS}/present is unreadable")))?;
        let cpus: Vec<LinuxCpu> = present.iter().map(|&number| one(number)).collect();
        let hybrid = text("/sys/devices/cpu_core/cpus")
            .filter(|_| Path::new("/sys/devices/cpu_atom/cpus").exists())
            .and_then(|list| cpu_list(&list));
        from_sysfs(&cpus, &allowed, hybrid.as_ref())
            .ok_or_else(|| Unread("sysfs describes a topology that does not add up".to_owned()))
    }

    /// The CPUs this process may run on: `sched_getaffinity`'s answer, as
    /// the kernel prints it for this process.
    fn allowed() -> Result<BTreeSet<u32>, Unread> {
        text("/proc/self/status")
            .and_then(|status| {
                status
                    .lines()
                    .find_map(|line| line.strip_prefix("Cpus_allowed_list:"))
                    .and_then(cpu_list)
            })
            .ok_or_else(|| Unread("/proc/self/status names no Cpus_allowed_list".to_owned()))
    }

    fn one(number: u32) -> LinuxCpu {
        let at = |leaf: &str| number_at(&format!("{CPUS}/cpu{number}/{leaf}"));
        LinuxCpu {
            number,
            package: at("topology/physical_package_id").unwrap_or(0),
            core: at("topology/core_id").unwrap_or(number),
            cache: at("cache/index3/id")
                .or_else(|| at("cache/index2/id"))
                .unwrap_or(0),
            capacity: at("cpu_capacity"),
        }
    }

    fn number_at(path: &str) -> Option<u32> {
        text(path)?.trim().parse().ok()
    }

    /// A sysfs file's text, or `None` where the kernel does not offer it:
    /// an absent field is the topology's to report, not an error.
    fn text(path: &str) -> Option<String> {
        std::fs::read_to_string(path).ok()
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use super::{PerfLevel, Topology, Unread, from_perf_levels};

    pub(super) fn read() -> Result<Topology, Unread> {
        let levels = match sysctl("hw.nperflevels") {
            Ok(count) => (0..count)
                .map(|level| {
                    Ok(PerfLevel {
                        physical: sysctl(&format!("hw.perflevel{level}.physicalcpu"))?,
                        logical: sysctl(&format!("hw.perflevel{level}.logicalcpu"))?,
                    })
                })
                .collect::<Result<Vec<_>, Unread>>()?,
            // An Intel Mac has no performance levels: one class.
            Err(_) => vec![PerfLevel {
                physical: sysctl("hw.physicalcpu")?,
                logical: sysctl("hw.logicalcpu")?,
            }],
        };
        from_perf_levels(&levels)
            .ok_or_else(|| Unread("sysctl describes a topology that does not add up".to_owned()))
    }

    fn sysctl(name: &str) -> Result<u32, Unread> {
        let out = child::command("/usr/sbin/sysctl")
            .args(["-n", name])
            .output()
            .map_err(|err| Unread(format!("sysctl did not start: {err}")))?;
        if !out.status.success() {
            return Err(Unread(format!("sysctl has no {name}")));
        }
        String::from_utf8_lossy(&out.stdout)
            .trim()
            .parse()
            .map_err(|err| Unread(format!("sysctl {name} is not a number: {err}")))
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests {
    use super::*;
    use crate::serving::placement::plan::{Left, Plan, plan};

    /// This machine's own `GetSystemCpuSetInformation` answer, captured
    /// once: an i5-1340P, 4 P-cores with SMT reporting efficiency class 1
    /// and 8 E-cores reporting class 0, one last-level cache.
    const I5_1340P: &[u8] = include_bytes!("reading/i5-1340p.cpusets");

    fn seats(numbers: &[u32]) -> Plan {
        Plan::Seats(
            numbers
                .iter()
                .map(|&number| Processor { group: 0, number })
                .collect(),
        )
    }

    #[test]
    fn this_machines_reading_seats_its_four_performance_cores() {
        let sets = desktop_ffi::cpu_set::parse(I5_1340P).unwrap();
        assert_eq!(sets.len(), 16);
        let topology = from_cpu_sets(&sets, 0, 0xFFFF);
        assert_eq!(plan(&topology), seats(&[0, 2, 4, 6]));
        let shape = topology.shape();
        let classes: Vec<(u8, usize, usize)> = shape
            .classes
            .iter()
            .map(|c| (c.class, c.cores, c.logical))
            .collect();
        assert_eq!(classes, vec![(1, 4, 8), (0, 8, 8)]);
    }

    #[test]
    fn a_job_that_narrows_the_mask_narrows_the_plan() {
        let sets = desktop_ffi::cpu_set::parse(I5_1340P).unwrap();
        // Processors 3 and 8..15: one P-core thread and the E-cores.
        assert_eq!(plan(&from_cpu_sets(&sets, 0, 0xFF08)), seats(&[3]));
        // The E-cores alone are one class.
        assert_eq!(
            plan(&from_cpu_sets(&sets, 0, 0xFF00)),
            Plan::LeftToOs(Left::OneClass)
        );
        // A thread in another group may name none of these.
        assert_eq!(
            plan(&from_cpu_sets(&sets, 1, 0xFFFF)),
            Plan::LeftToOs(Left::NothingUsable)
        );
    }

    #[test]
    fn a_linux_cpu_list_reads_ranges_and_refuses_what_is_not_one() {
        assert_eq!(
            cpu_list("0-3,8,10-11\n"),
            Some([0, 1, 2, 3, 8, 10, 11].into())
        );
        assert_eq!(cpu_list(""), Some(BTreeSet::new()));
        assert_eq!(cpu_list("3-1"), None);
        assert_eq!(cpu_list("a"), None);
    }

    fn linux(number: u32, core: u32, capacity: Option<u32>) -> LinuxCpu {
        LinuxCpu {
            number,
            package: 0,
            core,
            cache: 0,
            capacity,
        }
    }

    #[test]
    fn linux_on_intel_hybrid_reads_cpu_core_as_the_fast_class() {
        // i5-1340P under Linux: cpu0-7 are cpu_core (SMT pairs), 8-15 cpu_atom.
        let cpus: Vec<LinuxCpu> = (0..16)
            .map(|n| linux(n, if n < 8 { n / 2 } else { n }, None))
            .collect();
        let allowed: BTreeSet<u32> = (0..16).collect();
        let big: BTreeSet<u32> = (0..8).collect();
        let topology = from_sysfs(&cpus, &allowed, Some(&big)).unwrap();
        assert_eq!(plan(&topology), seats(&[0, 2, 4, 6]));
    }

    #[test]
    fn linux_on_arm_reads_three_capacities_as_three_classes() {
        // A prime core, three big cores and four little ones.
        let capacity = |n: u32| match n {
            0 => 1024,
            1..=3 => 870,
            _ => 380,
        };
        let cpus: Vec<LinuxCpu> = (0..8).map(|n| linux(n, n, Some(capacity(n)))).collect();
        let allowed: BTreeSet<u32> = (0..8).collect();
        assert_eq!(
            plan(&from_sysfs(&cpus, &allowed, None).unwrap()),
            seats(&[0])
        );
        // A container given two little cores.
        let two: BTreeSet<u32> = [6, 7].into();
        assert_eq!(
            plan(&from_sysfs(&cpus, &two, None).unwrap()),
            Plan::LeftToOs(Left::OneClass)
        );
    }

    #[test]
    fn an_apple_silicon_mac_reads_its_performance_levels_as_classes() {
        // M3 Pro: perflevel0 has 6 performance cores, perflevel1 6 efficiency cores.
        let topology = from_perf_levels(&[
            PerfLevel {
                physical: 6,
                logical: 6,
            },
            PerfLevel {
                physical: 6,
                logical: 6,
            },
        ])
        .unwrap();
        assert_eq!(plan(&topology), seats(&[0, 1, 2, 3, 4, 5]));
        // An Intel Mac reports no levels and is read as one.
        let intel = from_perf_levels(&[PerfLevel {
            physical: 8,
            logical: 16,
        }])
        .unwrap();
        assert_eq!(plan(&intel), Plan::LeftToOs(Left::OneClass));
    }
}
