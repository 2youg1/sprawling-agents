// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The placement plan: a pure function from the processor topology the
//! platform reported to the CPUs the hot threads may prefer
//! (`crates/sprawling/spec/Serving/Placement/Plan.lean`, decision D45 in
//! `crates/sprawling/spec/Serving/Placement.lean`). It calls no platform,
//! so every rule is checked here on described topologies of processors
//! this machine is not.

use std::collections::{BTreeMap, BTreeSet};

/// A logical processor as the platform numbers it: a processor group
/// (always 0 off Windows) and the number inside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Processor {
    pub(crate) group: u16,
    pub(crate) number: u32,
}

/// Whether this process may run on a processor: its affinity, a Job
/// Object, a container's cpuset or a virtual machine decides.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Access {
    Allowed,
    Barred,
}

/// One logical processor as read. `class` grows with speed; `core` and
/// `cache` are numbered inside the processor's group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Cpu {
    pub(crate) processor: Processor,
    pub(crate) class: u8,
    pub(crate) core: u32,
    pub(crate) cache: u32,
    pub(crate) access: Access,
}

impl Cpu {
    fn core_key(&self) -> (u16, u32) {
        (self.processor.group, self.core)
    }

    fn usable(&self) -> bool {
        self.access == Access::Allowed
    }
}

/// The topology as the platform reported it, in the platform's order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Topology(Vec<Cpu>);

/// One efficiency class among the usable processors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Class {
    pub(crate) class: u8,
    pub(crate) cores: usize,
    pub(crate) logical: usize,
}

/// What the doctor says about a topology: the usable classes, fastest
/// first, the cache groups they span, and how many processors the
/// process may use out of how many there are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Shape {
    pub(crate) classes: Vec<Class>,
    pub(crate) caches: usize,
    pub(crate) usable: usize,
    pub(crate) logical: usize,
}

impl Topology {
    pub(crate) fn new(cpus: Vec<Cpu>) -> Self {
        Self(cpus)
    }

    pub(crate) fn shape(&self) -> Shape {
        let usable: Vec<&Cpu> = self.usable().collect();
        let mut classes: BTreeMap<u8, (BTreeSet<(u16, u32)>, usize)> = BTreeMap::new();
        for cpu in &usable {
            let (cores, logical) = classes.entry(cpu.class).or_default();
            cores.insert(cpu.core_key());
            *logical = logical.saturating_add(1);
        }
        Shape {
            classes: classes
                .into_iter()
                .rev()
                .map(|(class, (cores, logical))| Class {
                    class,
                    cores: cores.len(),
                    logical,
                })
                .collect(),
            caches: usable
                .iter()
                .map(|cpu| (cpu.processor.group, cpu.cache))
                .collect::<BTreeSet<_>>()
                .len(),
            usable: usable.len(),
            logical: self.0.len(),
        }
    }

    fn usable(&self) -> impl Iterator<Item = &Cpu> {
        self.0.iter().filter(|cpu| cpu.usable())
    }

    /// No processor reported twice, and no physical core in two classes.
    fn consistent(&self) -> bool {
        let mut seen = BTreeSet::new();
        let mut core_class = BTreeMap::new();
        self.0.iter().all(|cpu| {
            seen.insert(cpu.processor)
                && *core_class.entry(cpu.core_key()).or_insert(cpu.class) == cpu.class
        })
    }
}

/// The CPUs the hot threads prefer, or why the operating system alone
/// places them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Plan {
    /// Never empty, sorted by processor.
    Seats(Vec<Processor>),
    LeftToOs(Left),
}

/// Why a topology gets no plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Left {
    /// The usable processors are one efficiency class, whatever their caches.
    OneClass,
    /// A processor reported twice, or a physical core in two classes.
    Inconsistent,
    /// The process may use none of the processors reported.
    NothingUsable,
}

/// The plan for `topology`: the fastest usable class, the lowest-numbered
/// usable processor of each of its physical cores, sorted by processor.
pub(crate) fn plan(topology: &Topology) -> Plan {
    if !topology.consistent() {
        return Plan::LeftToOs(Left::Inconsistent);
    }
    let classes: BTreeSet<u8> = topology.usable().map(|cpu| cpu.class).collect();
    let (Some(&slowest), Some(&fastest)) = (classes.first(), classes.last()) else {
        return Plan::LeftToOs(Left::NothingUsable);
    };
    if slowest == fastest {
        return Plan::LeftToOs(Left::OneClass);
    }
    let mut seats: Vec<Processor> = topology
        .usable()
        .filter(|cpu| cpu.class == fastest)
        .map(|cpu| cpu.processor)
        .collect();
    seats.sort_unstable();
    Plan::Seats(seats)
}

#[cfg(test)]
mod tests;
