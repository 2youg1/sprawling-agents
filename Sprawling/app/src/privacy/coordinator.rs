// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::num::NonZeroU64;

use kernel::{AxError, SecretRef};
use wire::{PrivacyControl, PrivacySettlement};

use super::fault::{HistoryFault, PrivacyFault, ReadFault, Unconfirmed, WriteFault};
use super::journal::LockedJournal;
use super::plan::{self, ApplyPlan, ReconcilePlan, Request, RestorePlan, RollbackEnd, Verdict};
use super::state::{Event, History, Intent, Line, Outcome, SCHEMA};
use super::target::{Reading, Snapshot};

const TTL_MS: u64 = 60_000;

pub(crate) trait Host: accounting::Clock {
    type Identity;

    fn identity(&mut self) -> Result<Self::Identity, AxError>;

    fn owner(
        &mut self,
        recorded: Option<&SecretRef>,
        identity: &Self::Identity,
    ) -> Result<SecretRef, AxError>;

    fn read(&mut self, control: PrivacyControl) -> Result<Reading, ReadFault>;

    fn write(&mut self, control: PrivacyControl, value: &Snapshot) -> Result<(), WriteFault>;
}

pub(super) trait Journal {
    fn history(&self) -> &History;

    fn append_durable(&mut self, line: &Line) -> Result<(), HistoryFault>;
}

impl Journal for LockedJournal {
    fn history(&self) -> &History {
        Self::history(self)
    }

    fn append_durable(&mut self, line: &Line) -> Result<(), HistoryFault> {
        Self::append_durable(self, line)
    }
}

pub(super) enum Command {
    Apply {
        control: PrivacyControl,
        expected: Snapshot,
    },
    Restore {
        control: PrivacyControl,
        expected: Snapshot,
    },
    Reconcile {
        expected: Snapshot,
    },
}

#[derive(Debug, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "done", rename_all = "snake_case")]
pub(super) enum Done {
    Applied {
        operation: NonZeroU64,
    },
    Restored {
        operation: NonZeroU64,
    },
    AlreadyWritten,
    Reconciled {
        operation: NonZeroU64,
        settlement: PrivacySettlement,
    },
}

pub(super) fn run<H: Host, J: Journal>(
    host: &mut H,
    open: impl FnOnce() -> Result<J, HistoryFault>,
    command: &Command,
) -> Result<Done, PrivacyFault> {
    let accepted = host.now().map_err(PrivacyFault::Clock)?;
    let identity = host.identity().map_err(PrivacyFault::Identity)?;
    let mut session = Session {
        journal: open().map_err(PrivacyFault::History)?,
        host,
        identity,
        deadline: accepted.value().saturating_add(TTL_MS),
    };
    match command {
        Command::Apply { control, expected } => session.change(Action::Apply, *control, expected),
        Command::Restore { control, expected } => {
            session.change(Action::Restore, *control, expected)
        }
        Command::Reconcile { expected } => session.reconcile(expected),
    }
}

#[derive(Clone, Copy)]
enum Action {
    Apply,
    Restore,
}

struct Session<'h, H: Host, J> {
    host: &'h mut H,
    journal: J,
    identity: H::Identity,
    deadline: u64,
}

impl<H: Host, J: Journal> Session<'_, H, J> {
    fn change(
        &mut self,
        action: Action,
        control: PrivacyControl,
        expected: &Snapshot,
    ) -> Result<Done, PrivacyFault> {
        let (host, identity) = (&mut *self.host, &self.identity);
        let holdings = self
            .journal
            .history()
            .holdings(|recorded| host.owner(recorded, identity))
            .map_err(PrivacyFault::Identity)?;
        let request = Request {
            control,
            operation: holdings.next_operation().ok_or(PrivacyFault::HistoryFull)?,
            expected,
        };
        let fresh = host
            .read(control)
            .map_err(|fault| PrivacyFault::Unreadable { control, fault })?;
        let intent = match action {
            Action::Apply => match plan::plan_apply(&holdings, &request, &fresh) {
                ApplyPlan::Write(intent) => intent,
                ApplyPlan::Unresolved => return Err(PrivacyFault::Unresolved),
                ApplyPlan::Changed => return Err(PrivacyFault::Changed { control }),
                ApplyPlan::TargetAbsent => return Err(PrivacyFault::TargetAbsent { control }),
                ApplyPlan::AlreadyWritten => return Ok(Done::AlreadyWritten),
            },
            Action::Restore => match plan::plan_restore(&holdings, &request, &fresh) {
                RestorePlan::Write(intent) => intent,
                RestorePlan::Unresolved => return Err(PrivacyFault::Unresolved),
                RestorePlan::NothingOwned => return Err(PrivacyFault::NothingOwned { control }),
                RestorePlan::Changed => return Err(PrivacyFault::Changed { control }),
                RestorePlan::Conflict => return Err(PrivacyFault::Conflict { control }),
            },
        };
        self.carry_out(&intent)
    }

    fn carry_out(&mut self, intent: &Intent) -> Result<Done, PrivacyFault> {
        let (control, operation) = (intent.control, intent.operation);
        self.unexpired(control, None)?;
        self.journal
            .append_durable(&line(Event::Prepared {
                intent: intent.clone(),
            }))
            .map_err(PrivacyFault::History)?;
        if let Err(fault) = self.unexpired(control, Some(operation)) {
            self.record(intent, Outcome::NotApplied)?;
            return Err(fault);
        }
        let written = self.host.write(control, &intent.modified).err();
        let readback = match self.host.read(control) {
            Ok(reading) => reading.value,
            Err(fault) => {
                return self.lose(intent, Unconfirmed::ReadbackUnreadable(fault));
            }
        };
        match plan::settle(intent, plan::judge_readback(intent, &readback)) {
            Verdict::Applied => self
                .record(intent, Outcome::Applied)
                .map(|()| Done::Applied { operation }),
            Verdict::Restored => self
                .record(intent, Outcome::Restored)
                .map(|()| Done::Restored { operation }),
            Verdict::NotApplied => {
                self.record(intent, Outcome::NotApplied)?;
                Err(PrivacyFault::NotApplied {
                    control,
                    operation,
                    cause: written,
                })
            }
            Verdict::RollBack => self.roll_back(intent),
        }
    }

    fn roll_back(&mut self, intent: &Intent) -> Result<Done, PrivacyFault> {
        let (control, operation) = (intent.control, intent.operation);
        let written = self.host.write(control, &intent.original).err();
        let end = match self.host.read(control) {
            Ok(reading) => plan::settle_rollback(intent, &reading.value),
            Err(fault) => {
                return self.lose(intent, Unconfirmed::RollbackUnreadable(fault));
            }
        };
        match end {
            RollbackEnd::RolledBack => {
                self.record(intent, Outcome::RolledBack)?;
                Err(PrivacyFault::RolledBack { control, operation })
            }
            RollbackEnd::Lost => self.lose(intent, Unconfirmed::RollbackMissed(written)),
        }
    }

    fn lose(&mut self, intent: &Intent, why: Unconfirmed) -> Result<Done, PrivacyFault> {
        self.record(intent, Outcome::Unknown)?;
        Err(PrivacyFault::Unknown {
            control: intent.control,
            operation: intent.operation,
            why,
        })
    }

    fn reconcile(&mut self, expected: &Snapshot) -> Result<Done, PrivacyFault> {
        let (host, identity) = (&mut *self.host, &self.identity);
        let unresolved = self
            .journal
            .history()
            .holdings(|recorded| host.owner(recorded, identity))
            .map_err(PrivacyFault::Identity)?
            .unresolved()
            .cloned()
            .ok_or(PrivacyFault::NothingUnresolved)?;
        let control = unresolved.control;
        let fresh = host
            .read(control)
            .map_err(|fault| PrivacyFault::Unreadable { control, fault })?;
        let settlement = match plan::plan_reconcile(&unresolved, expected, &fresh.value) {
            ReconcilePlan::Settle(settlement) => settlement,
            ReconcilePlan::Changed => return Err(PrivacyFault::Changed { control }),
        };
        let operation = unresolved.operation;
        self.journal
            .append_durable(&line(Event::Reconciled {
                operation,
                settlement,
            }))
            .map_err(PrivacyFault::History)?;
        Ok(Done::Reconciled {
            operation,
            settlement,
        })
    }

    fn unexpired(
        &self,
        control: PrivacyControl,
        operation: Option<NonZeroU64>,
    ) -> Result<(), PrivacyFault> {
        let now = self.host.now().map_err(PrivacyFault::Clock)?;
        if now.value() < self.deadline {
            Ok(())
        } else {
            Err(PrivacyFault::Expired { control, operation })
        }
    }

    fn record(&mut self, intent: &Intent, outcome: Outcome) -> Result<(), PrivacyFault> {
        self.journal
            .append_durable(&line(Event::Finished {
                operation: intent.operation,
                outcome,
            }))
            .map_err(|fault| PrivacyFault::ReceiptLost {
                control: intent.control,
                operation: intent.operation,
                fault,
            })
    }
}

fn line(event: Event) -> Line {
    Line {
        schema: SCHEMA,
        event,
    }
}

impl Command {
    pub(super) const fn action(&self) -> &'static str {
        match self {
            Self::Apply { .. } => "apply privacy control",
            Self::Restore { .. } => "restore privacy control",
            Self::Reconcile { .. } => "reconcile privacy operation",
        }
    }
}

#[cfg(test)]
mod support;
#[cfg(test)]
mod tests;
