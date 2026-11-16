// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Processor {
    pub(crate) group: u16,
    pub(crate) number: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Access {
    Allowed,
    Barred,
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Topology(Vec<Cpu>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Class {
    pub(crate) class: u8,
    pub(crate) cores: usize,
    pub(crate) logical: usize,
}

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

    fn consistent(&self) -> bool {
        let mut seen = BTreeSet::new();
        let mut core_class = BTreeMap::new();
        self.0.iter().all(|cpu| {
            seen.insert(cpu.processor)
                && *core_class.entry(cpu.core_key()).or_insert(cpu.class) == cpu.class
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Plan {

    Seats(Vec<Processor>),
    LeftToOs(Left),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Left {

    OneClass,

    Inconsistent,

    NothingUsable,
}

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
    let mut leads: BTreeMap<(u16, u32), Processor> = BTreeMap::new();
    for cpu in topology.usable().filter(|cpu| cpu.class == fastest) {
        let lead = leads.entry(cpu.core_key()).or_insert(cpu.processor);
        *lead = (*lead).min(cpu.processor);
    }
    let mut seats: Vec<Processor> = leads.into_values().collect();
    seats.sort_unstable();
    Plan::Seats(seats)
}

#[cfg(test)]
mod tests;
