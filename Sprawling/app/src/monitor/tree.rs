// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

use super::counters::Reads;

pub const MINIMUM_BEAT_MS: u64 = 250;

pub const MINIMUM_BEAT: Duration = Duration::from_millis(MINIMUM_BEAT_MS);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TreeReading {
    pub processes: u64,

    pub cpu_permille: Option<u64>,
    pub private_bytes: u64,
    pub working_set_bytes: u64,
    pub read_bytes: u64,
    pub written_bytes: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Seen {
    pub cpu_ms: u64,
    pub read_bytes: u64,
    pub written_bytes: u64,
    pub peak_private_bytes: u64,
    pub peak_working_set_bytes: u64,
}

pub struct Tree {
    system: System,
    root: Pid,

    last: BTreeMap<u32, Last>,
    peak_private_bytes: u64,
    peak_working_set_bytes: u64,
    beats: u64,
    table_reads: u64,
}

#[derive(Debug, Clone, Copy)]
struct Last {
    cpu_ms: u64,
    read_bytes: u64,
    written_bytes: u64,
}

struct Member {
    pid: u32,
    last: Last,
    private_bytes: u64,
    working_set_bytes: u64,
}

impl Tree {
    pub fn open(root: u32) -> Result<Tree, sysinfo::Error> {
        Ok(Tree {
            system: System::new()?,
            root: Pid::from_u32(root),
            last: BTreeMap::new(),
            peak_private_bytes: 0,
            peak_working_set_bytes: 0,
            beats: 0,
            table_reads: 0,
        })
    }

    pub fn read(&mut self, elapsed: Duration) -> Option<TreeReading> {
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing()
                .with_cpu()
                .with_memory()
                .with_disk_usage(),
        );
        self.table_reads = self.table_reads.saturating_add(1);
        let members = members(&self.system, self.root)?;
        let mut reading = TreeReading::default();
        let mut spent_ms = 0_u64;
        for member in members {
            let before = self.last.insert(member.pid, member.last);
            spent_ms = spent_ms.saturating_add(
                member
                    .last
                    .cpu_ms
                    .saturating_sub(before.map_or(0, |last| last.cpu_ms)),
            );
            reading.processes = reading.processes.saturating_add(1);
            reading.private_bytes = reading.private_bytes.saturating_add(member.private_bytes);
            reading.working_set_bytes = reading
                .working_set_bytes
                .saturating_add(member.working_set_bytes);
            reading.read_bytes = reading.read_bytes.saturating_add(member.last.read_bytes);
            reading.written_bytes = reading
                .written_bytes
                .saturating_add(member.last.written_bytes);
        }
        reading.cpu_permille = match self.beats {
            0 => None,
            _ => Some(permille(spent_ms, elapsed)),
        };
        self.beats = self.beats.saturating_add(1);
        self.peak_private_bytes = self.peak_private_bytes.max(reading.private_bytes);
        self.peak_working_set_bytes = self.peak_working_set_bytes.max(reading.working_set_bytes);
        Some(reading)
    }

    #[must_use]
    pub fn seen(&self) -> Option<Seen> {
        if self.beats == 0 {
            return None;
        }
        let sum = |figure: fn(&Last) -> u64| {
            self.last
                .values()
                .map(figure)
                .fold(0_u64, u64::saturating_add)
        };
        Some(Seen {
            cpu_ms: sum(|last| last.cpu_ms),
            read_bytes: sum(|last| last.read_bytes),
            written_bytes: sum(|last| last.written_bytes),
            peak_private_bytes: self.peak_private_bytes,
            peak_working_set_bytes: self.peak_working_set_bytes,
        })
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the count gate's tests read it (`Sprawling/app/spec/Main.lean` §8-129-3); a production reading only counts"
        )
    )]
    pub(crate) fn reads(&self) -> Reads {
        Reads {
            own: 0,
            machine: 0,
            table: self.table_reads,
        }
    }
}

fn members(system: &System, root: Pid) -> Option<Vec<Member>> {
    system.process(root)?;
    let mut children: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for (pid, process) in system.processes() {
        if let Some(parent) = process.parent() {
            children
                .entry(parent.as_u32())
                .or_default()
                .push(pid.as_u32());
        }
    }
    let mut visited = BTreeSet::new();
    let mut waiting = vec![root.as_u32()];
    let mut found = Vec::new();
    while let Some(pid) = waiting.pop() {
        if !visited.insert(pid) {
            continue;
        }
        if let Some(process) = system.process(Pid::from_u32(pid)) {
            let disk = process.disk_usage();
            found.push(Member {
                pid,
                last: Last {
                    cpu_ms: process.accumulated_cpu_time(),
                    read_bytes: disk.total_read_bytes,
                    written_bytes: disk.total_written_bytes,
                },
                private_bytes: process.virtual_memory(),
                working_set_bytes: process.memory(),
            });
        }
        waiting.extend(children.get(&pid).into_iter().flatten().copied());
    }
    Some(found)
}

fn permille(spent_ms: u64, elapsed: Duration) -> u64 {
    let cores = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
    let capacity = elapsed
        .as_nanos()
        .saturating_mul(u128::try_from(cores).unwrap_or(u128::MAX));
    u128::from(spent_ms)
        .saturating_mul(1_000_000_000)
        .checked_div(capacity)
        .map_or(0, |share| u64::try_from(share.min(1000)).unwrap_or(1000))
}

#[cfg(test)]
#[path = "tree/tests.rs"]
mod tests;
