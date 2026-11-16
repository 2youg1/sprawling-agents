// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::{BTreeSet, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use kernel::{AxCode, AxError, IdemKey};
use wire::{
    PrivacyAction, PrivacyAnswer, PrivacyHistory, PrivacyHost, PrivacyOutcome, PrivacyResult,
};

use super::coordinator::{self, Command, Done, Host};
use super::journal::{self, LockedJournal};
use super::target::Snapshot;

const KEPT: usize = 64;

pub(crate) trait Machine: Host + Send {
    fn facts(&mut self) -> PrivacyHost;
}

pub(crate) struct Service<M> {
    machine: Box<dyn Fn() -> M + Send + Sync>,
    history: PathBuf,
    kept: Mutex<Kept>,

    turn: Mutex<()>,
}

#[derive(Default)]
struct Kept {
    outcomes: VecDeque<PrivacyOutcome>,
    accepted: BTreeSet<IdemKey>,
}

pub(super) struct Operation {
    idem: IdemKey,
    command: Command,
}

impl Service<super::system::System> {
    pub(crate) fn of_this_machine() -> Result<Self, AxError> {
        Ok(Self::new(
            super::system::this_machine,
            accounting::home::Home::detect()?.privacy_history(),
        ))
    }
}

impl<M: Machine + 'static> Service<M> {
    pub(super) fn new(machine: impl Fn() -> M + Send + Sync + 'static, history: PathBuf) -> Self {
        Self {
            machine: Box::new(machine),
            history,
            kept: Mutex::default(),
            turn: Mutex::default(),
        }
    }

    pub(crate) fn answer(&self) -> PrivacyAnswer {
        let outcomes = self.kept().outcomes.iter().cloned().collect();
        let mut machine = (self.machine)();
        let host = machine.facts();
        let history = self.disclosed(&mut machine);
        let windows = !matches!(host, PrivacyHost::NotWindows);
        super::answer::answer(
            host,
            |control| windows.then(|| machine.read(control).map(|reading| reading.value)),
            history,
            outcomes,
        )
    }

    pub(crate) fn carry_out(
        self: &Arc<Self>,
        action: PrivacyAction,
        idem: IdemKey,
    ) -> Result<(), AxError> {
        let Some(operation) = self.accept(action, idem)? else {
            return Ok(());
        };
        let service = Arc::clone(self);
        std::thread::Builder::new()
            .name("sprawling-privacy".to_owned())
            .spawn(move || service.perform(operation))
            .map(drop)
            .map_err(|source| {
                self.kept().forget(idem);
                AxError::failure(
                    AxCode::ToolUnavailable,
                    "carry out a privacy operation",
                    format!("no thread could be started for it: {source}"),
                )
                .with_recovery("nothing was written; send the operation again")
            })
    }

    pub(super) fn accept(
        &self,
        action: PrivacyAction,
        idem: IdemKey,
    ) -> Result<Option<Operation>, AxError> {
        let command = command(&action)?;
        let mut kept = self.kept();
        if !kept.accepted.insert(idem) {
            return if kept.outcomes.iter().any(|outcome| outcome.idem == idem) {
                Ok(None)
            } else {
                Err(AxError::failure(
                    AxCode::InvalidArgs,
                    command.action(),
                    "this operation was carried out before and its result is no longer kept",
                )
                .with_recovery(
                    "read the privacy page again; send a new operation to change anything",
                ))
            };
        }
        kept.record(PrivacyOutcome {
            idem,
            action,
            result: PrivacyResult::Running,
        });
        Ok(Some(Operation { idem, command }))
    }

    pub(super) fn perform(&self, operation: Operation) {
        let result = {
            let _turn = self.turn.lock().unwrap_or_else(PoisonError::into_inner);
            let mut machine = (self.machine)();
            match coordinator::run(
                &mut machine,
                || LockedJournal::open(&self.history),
                &operation.command,
            ) {
                Ok(done) => result(done),
                Err(fault) => PrivacyResult::Refused {
                    code: fault.code(),
                    error: fault.into_ax(operation.command.action()),
                },
            }
        };
        self.kept().settle(operation.idem, result);
    }

    fn disclosed(&self, machine: &mut M) -> PrivacyHistory {
        let history = match journal::read(&self.history) {
            Ok(history) => history,
            Err(fault) => {
                return PrivacyHistory::Unreadable {
                    error: fault.into_ax("read local privacy history"),
                };
            }
        };

        let released = history.holdings(|recorded| match recorded {
            None => Ok(()),
            Some(owner) => {
                let identity = machine.identity()?;
                machine.owner(Some(owner), &identity).map(drop)
            }
        });
        match released {
            Ok(holdings) => super::answer::disclosed(&holdings),
            Err(error) => PrivacyHistory::Withheld { error },
        }
    }

    fn kept(&self) -> MutexGuard<'_, Kept> {
        self.kept.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Kept {
    fn record(&mut self, outcome: PrivacyOutcome) {
        self.outcomes.push_back(outcome);
        while self.outcomes.len() > KEPT {
            self.outcomes.pop_front();
        }
    }

    fn settle(&mut self, idem: IdemKey, result: PrivacyResult) {
        if let Some(outcome) = self
            .outcomes
            .iter_mut()
            .find(|outcome| outcome.idem == idem)
        {
            outcome.result = result;
        }
    }

    fn forget(&mut self, idem: IdemKey) {
        self.accepted.remove(&idem);
        self.outcomes.retain(|outcome| outcome.idem != idem);
    }
}

fn command(action: &PrivacyAction) -> Result<Command, AxError> {
    Ok(match action {
        PrivacyAction::Apply { control, expected } => Command::Apply {
            control: *control,
            expected: Snapshot::try_from(expected)?,
        },
        PrivacyAction::Restore { control, expected } => Command::Restore {
            control: *control,
            expected: Snapshot::try_from(expected)?,
        },
        PrivacyAction::Reconcile { expected } => Command::Reconcile {
            expected: Snapshot::try_from(expected)?,
        },
    })
}

fn result(done: Done) -> PrivacyResult {
    match done {
        Done::Applied { operation } => PrivacyResult::Applied {
            operation: operation.get(),
        },
        Done::Restored { operation } => PrivacyResult::Restored {
            operation: operation.get(),
        },
        Done::AlreadyWritten => PrivacyResult::AlreadyWritten,
        Done::Reconciled {
            operation,
            settlement,
        } => PrivacyResult::Reconciled {
            operation: operation.get(),
            settlement,
        },
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests;
