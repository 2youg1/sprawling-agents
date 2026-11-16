// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::rc::Rc;

use kernel::{AxCode, TimeMs};

use super::super::state::fixtures::{dword, owner};
use super::super::target::{RawValue, TaskState};
use super::*;

pub(super) const CONTROLS: [PrivacyControl; 4] = [
    PrivacyControl::FeedbackNotifications,
    PrivacyControl::StartLaunchTracking,
    PrivacyControl::PowershellTelemetryOptout,
    PrivacyControl::DeviceCensusTask,
];

pub(super) fn absent() -> Snapshot {
    Snapshot::Registry(RawValue::Absent)
}

pub(super) fn task(state: fn([u8; 32]) -> TaskState, digest: u8) -> Snapshot {
    Snapshot::Task(state([digest; 32]))
}

pub(super) fn enabled(definition_sha256: [u8; 32]) -> TaskState {
    TaskState::Enabled { definition_sha256 }
}

pub(super) fn disabled(definition_sha256: [u8; 32]) -> TaskState {
    TaskState::Disabled { definition_sha256 }
}

/// A value of the same kind that nobody asked for.
pub(super) fn third(value: &Snapshot) -> Snapshot {
    match value {
        Snapshot::Registry(_) => dword(77),
        Snapshot::Task(_) => task(disabled, 9),
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum WriteEffect {
    Lands,
    LandsReportingFailure,
    LandsThird,
    Refused(Refusal),
}

#[derive(Debug, Clone, Copy)]
pub(super) enum Refusal {
    Denied,
    Declined,
    Failed,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum ReadEffect {
    Reads,
    Denied,
    Fails,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum AppendEffect {
    Durable,
    Fails,
}

/// What the host and the history saw, in order.
#[derive(Debug, Clone)]
pub(super) enum Seen {
    Run,
    Read,
    Wrote {
        control: PrivacyControl,
        value: Snapshot,
    },
    Appended {
        line: Line,
        values: BTreeMap<PrivacyControl, Snapshot>,
    },
}

/// The host's true values, the history's lines, the faults the next
/// calls meet, and the log of everything seen.
pub(super) struct World {
    pub(super) now: u64,
    pub(super) foreign: bool,
    pub(super) values: BTreeMap<PrivacyControl, Snapshot>,
    pub(super) lines: Vec<Line>,
    pub(super) reads: VecDeque<ReadEffect>,
    pub(super) writes: VecDeque<WriteEffect>,
    pub(super) appends: VecDeque<AppendEffect>,
    pub(super) lapse_on_prepared: bool,
    pub(super) log: Vec<Seen>,
}

pub(super) type Shared = Rc<RefCell<World>>;

pub(super) fn world() -> Shared {
    Rc::new(RefCell::new(World {
        now: 1_000,
        foreign: false,
        values: BTreeMap::from([
            (CONTROLS[0], absent()),
            (CONTROLS[1], dword(1)),
            (CONTROLS[2], absent()),
            (CONTROLS[3], task(enabled, 1)),
        ]),
        lines: Vec::new(),
        reads: VecDeque::new(),
        writes: VecDeque::new(),
        appends: VecDeque::new(),
        lapse_on_prepared: false,
        log: Vec::new(),
    }))
}

pub(super) struct FaultHost(pub(super) Shared);

impl accounting::Clock for FaultHost {
    fn now(&self) -> Result<TimeMs, AxError> {
        Ok(TimeMs::new(self.0.borrow().now))
    }
}

pub(super) fn failed(what: &str) -> AxError {
    AxError::failure(AxCode::ToolUnavailable, "fixture host", what).with_recovery("fixture fault")
}

impl Host for FaultHost {
    type Identity = ();

    fn identity(&mut self) -> Result<(), AxError> {
        Ok(())
    }

    fn owner(&mut self, recorded: Option<&SecretRef>, (): &()) -> Result<SecretRef, AxError> {
        match (recorded, self.0.borrow().foreign) {
            (None, false) => Ok(owner()),
            (Some(recorded), false) if *recorded == owner() => Ok(recorded.clone()),
            (None | Some(_), true | false) => Err(failed("another identity")),
        }
    }

    fn read(&mut self, control: PrivacyControl) -> Result<Reading, ReadFault> {
        let mut world = self.0.borrow_mut();
        world.log.push(Seen::Read);
        match world.reads.pop_front().unwrap_or(ReadEffect::Reads) {
            ReadEffect::Reads => Ok(Reading {
                value: world.values[&control].clone(),
                key_existed: true,
            }),
            ReadEffect::Denied => Err(ReadFault::AccessDenied),
            ReadEffect::Fails => Err(ReadFault::Failed(failed("read"))),
        }
    }

    fn write(&mut self, control: PrivacyControl, value: &Snapshot) -> Result<(), WriteFault> {
        let mut world = self.0.borrow_mut();
        world.log.push(Seen::Wrote {
            control,
            value: value.clone(),
        });
        let effect = world.writes.pop_front().unwrap_or(WriteEffect::Lands);
        let landed = match effect {
            WriteEffect::Lands | WriteEffect::LandsReportingFailure => value.clone(),
            WriteEffect::LandsThird => third(value),
            WriteEffect::Refused(_) => world.values[&control].clone(),
        };
        world.values.insert(control, landed);
        match effect {
            WriteEffect::Lands | WriteEffect::LandsThird => Ok(()),
            WriteEffect::LandsReportingFailure | WriteEffect::Refused(Refusal::Failed) => {
                Err(WriteFault::Failed(failed("write")))
            }
            WriteEffect::Refused(Refusal::Denied) => Err(WriteFault::AccessDenied),
            WriteEffect::Refused(Refusal::Declined) => Err(WriteFault::Declined),
        }
    }
}

/// The history kept in memory and judged by the same fold a file is.
pub(super) struct MemoryJournal {
    world: Shared,
    history: History,
}

impl MemoryJournal {
    pub(super) fn open(world: &Shared) -> Result<Self, HistoryFault> {
        let history = History::fold(world.borrow().lines.clone())?;
        Ok(Self {
            world: Rc::clone(world),
            history,
        })
    }
}

impl Journal for MemoryJournal {
    fn history(&self) -> &History {
        &self.history
    }

    fn append_durable(&mut self, line: &Line) -> Result<(), HistoryFault> {
        let mut world = self.world.borrow_mut();
        if let Some(AppendEffect::Fails) = world.appends.pop_front() {
            return Err(HistoryFault::Io(std::io::Error::other("fixture sync")));
        }
        let mut lines = world.lines.clone();
        lines.push(line.clone());
        self.history = History::fold(lines.clone())?;
        world.lines = lines;
        if world.lapse_on_prepared && matches!(line.event, Event::Prepared { .. }) {
            world.now += TTL_MS;
        }
        let values = world.values.clone();
        world.log.push(Seen::Appended {
            line: line.clone(),
            values,
        });
        Ok(())
    }
}
