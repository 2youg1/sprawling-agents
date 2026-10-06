// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The work still running while a run goes on, and the one place it is
//! stopped.
//!
//! `runtime::turn` consumes an interruption at a phase boundary. A child
//! process waited on inside a system call is not at a phase boundary, so
//! a command blocked there cannot be stopped. Every command therefore
//! enters this table before it is waited on: the wait
//! is a bounded number of short polls against a member somebody else can
//! reach, rather than one blocking call nobody can interrupt.
//!
//! Two facts about the wait are deliberate. It is counted rather than
//! timed, because the clock is sampled at one point in this city and a
//! module that sampled it to wait ten seconds would be the second one.
//! And a child's output goes to files rather than to pipes, because a
//! pipe buffer that fills stops the child inside a write — which is the
//! same hang this module exists to remove, wearing another name.
//!
//! How many polls that wait is, and how an ending is read, both belong
//! to [`waiting`]: the window is carried by the table rather than read
//! from a constant here, so a caller can choose one short enough to
//! observe both answers.
//!
//! The table has two kinds of member: a background command, which a
//! halt kills, and a run a resident handed down, which a halt marks and
//! which stops itself at its next safe point by asking [`Backlog::stopping`]
//! (`crates/runtime/spec/Backlog.lean` §8-28-2). One table, one `halt`, so a stopped scope has
//! nothing left going in it under either name.

use std::collections::BTreeMap;

use kernel::{Address, AxCode, AxError, RunId};

/// One member of the table, for as long as this process lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BacklogId(u64);

impl std::fmt::Display for BacklogId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "bg-{}", self.0)
    }
}

/// The table itself. Cloning gives another handle onto the same table,
/// which is how the assembly point can stop what a tool started without
/// either of them knowing about the other. A clone carries `scratch`
/// too, so it keeps naming that backlog's directories.
#[derive(Clone)]
pub struct Backlog {
    table: std::sync::Arc<std::sync::Mutex<Table>>,
    scratch: Scratch,
    window: PollBudget,
    sink: Option<Sink>,
    shares: Shares,
    affinity: RunAffinity,
}

impl Default for Backlog {
    fn default() -> Backlog {
        Backlog::with_window(PollBudget::DEFAULT)
    }
}

#[derive(Default)]
struct Table {
    next: u64,
    members: BTreeMap<BacklogId, Member>,
    jobs: jobs::Jobs,
}

impl Backlog {
    /// The same table, asking these shares for every run whose command
    /// it starts (`crates/runtime/spec/Tools/Exec.lean` D29).
    #[must_use]
    pub fn with_shares(self, shares: Shares) -> Backlog {
        Backlog { shares, ..self }
    }

    /// The shares this table asks for each run.
    #[must_use]
    pub fn shares(&self) -> Shares {
        self.shares
    }

    /// Enters a run a resident handed down, so a halt on its scope can
    /// reach it. The assembly layer calls this before the drive and
    /// [`Backlog::leave`] after it.
    ///
    /// # Errors
    /// `E_STORAGE_FATAL` when the table cannot be reached.
    pub fn enrol_run(&self, scope: &Address, what: String) -> Result<BacklogId, AxError> {
        let id = self.mint()?;
        self.enrol(
            id,
            Member {
                scope: scope.clone(),
                what,
                body: Body::Run(RunState::Going),
            },
        )?;
        Ok(id)
    }

    /// Whether a halt has reached this member. A run asks at each safe
    /// point and answers `true` with `Interrupt::Cancel`; a member that
    /// has already left the table is not stopping, it is gone.
    ///
    /// # Errors
    /// `E_STORAGE_FATAL` when the table cannot be reached.
    pub fn stopping(&self, id: BacklogId) -> Result<bool, AxError> {
        let table = self.hold()?;
        Ok(table
            .members
            .get(&id)
            .is_some_and(|member| matches!(member.body, Body::Run(RunState::Stopping))))
    }

    /// Takes a run member out once it has frozen. A run that stayed
    /// would be reported standing after its own `run_frozen` line.
    ///
    /// # Errors
    /// `E_STORAGE_FATAL` when the table cannot be reached.
    pub fn leave(&self, id: BacklogId) -> Result<(), AxError> {
        let mut table = self.hold()?;
        table.members.remove(&id);
        Ok(())
    }

    /// Stops every member inside a scope, or every member in the city
    /// when no scope is named. Returns how many were reached.
    ///
    /// A member is stopped where it stands, and the run that
    /// started it comes back to its next boundary rather than staying
    /// inside a system call. A run member is marked rather than killed:
    /// it comes back to its own next safe point and cancels there.
    ///
    /// # Errors
    /// `E_STORAGE_FATAL` when the table cannot be reached, which means
    /// a thread died holding it.
    pub fn halt(&self, scope: Option<&Address>) -> Result<usize, AxError> {
        let mut table = self.hold()?;
        let mut reached = 0usize;
        let mut failure = None;
        for member in table.members.values_mut() {
            let covered = scope.is_none_or(|within| member.scope.is_within(within));
            if !covered {
                continue;
            }
            let stopped = match &mut member.body {
                Body::Command { child, .. } => match child.stop() {
                    Ok(()) => true,
                    Err(err) => {
                        failure = Some(err);
                        false
                    }
                },
                Body::Run(state) => {
                    *state = RunState::Stopping;
                    true
                }
            };
            if stopped {
                reached = reached.saturating_add(1);
            }
        }
        match failure {
            Some(err) => Err(err),
            None => Ok(reached),
        }
    }

    /// Takes every background member of `owner` that has stopped, in
    /// table order, and lets go of every stopped member whose run has
    /// ended.
    ///
    /// A member is reported once and then forgotten: this is what a
    /// caller appends to the tail of its next tool result, and a result
    /// delivered twice would read as the command having run twice. A
    /// member another run started is never reported here, because that
    /// tail goes into this run's model and onto its lines of the ledger.
    ///
    /// # Errors
    /// `E_STORAGE_FATAL` when the table cannot be reached.
    pub fn harvest(&self, owner: RunId) -> Result<Vec<Finished>, AxError> {
        let mut table = self.hold()?;
        let mut done = Vec::new();
        let mut spent = Vec::new();
        let mut live = Vec::new();
        let Table { members, jobs, .. } = &mut *table;
        for (id, member) in members.iter_mut() {
            let Body::Command {
                child,
                dir,
                claim,
                tail,
                ceiling,
            } = &mut member.body
            else {
                continue;
            };
            match claim {
                Claim::Run(run) if *run == owner => {}
                Claim::Nobody => {}
                Claim::Window(_) | Claim::Run(_) => continue,
            }
            let Some(exit) = child.poll()? else {
                if let (Claim::Run(_), Some(_)) = (claim, &self.sink) {
                    live.extend(tail.take(dir, (owner, *id), self.window.read_per_poll()));
                }
                continue;
            };
            spent.push(*id);
            if let Claim::Run(_) = claim {
                let (stdout, stderr) = collect(dir);
                done.push(Finished {
                    id: *id,
                    what: member.what.clone(),
                    exit,
                    stdout,
                    stderr,
                    ceiling: jobs.ceiling(owner, *ceiling),
                });
            } else {
                // Output owed to nobody is not read at all. A directory
                // that will not go costs disk, never the harvest.
                drop(std::fs::remove_dir_all(dir));
            }
        }
        for id in spent {
            table.members.remove(&id);
        }
        drop(table);
        if let Some(sink) = &self.sink {
            sink.deliver(live);
        }
        Ok(done)
    }

    /// Terminates every background command `owner` started, marks each
    /// as owed to nobody, and returns how many there were. The run has
    /// ended, so no later tool result of its own can carry them; the
    /// next harvest by any run lets their process handles and files go
    /// without reading their output to anyone.
    ///
    /// Infallible by construction: lowering a claim and killing a
    /// process hold on any state of the table, so a table a dead thread
    /// left locked is still reached, while every other call keeps
    /// answering `E_STORAGE_FATAL`. The caller is a drop, which has
    /// nobody to hand a failure to.
    pub fn release(&self, owner: RunId) -> usize {
        let mut table = self
            .table
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        table.jobs.forget(owner);
        let mut released = 0usize;
        for member in table.members.values_mut() {
            if let Body::Command { claim, child, .. } = &mut member.body
                && matches!(*claim, Claim::Window(run) | Claim::Run(run) if run == owner)
            {
                *claim = Claim::Nobody;
                // `Child::kill` answers `Ok` for a child that already
                // exited, so an error here is a kill the system refused.
                // The member is still owed to nobody and the next harvest
                // reaps it once it stops; the caller is a drop with
                // nobody to hand the refusal to.
                if let Err(err) = child.stop() {
                    eprintln!("backlog cleanup retained for retry: {err}");
                }
                released = released.saturating_add(1);
            }
        }
        released
    }

    /// What is still running at or under one address.
    ///
    /// # Errors
    /// `E_STORAGE_FATAL` when the table cannot be reached.
    pub fn standing(&self, scope: &Address) -> Result<Vec<Standing>, AxError> {
        let table = self.hold()?;
        Ok(table
            .members
            .iter()
            .filter(|(_, member)| member.scope.is_within(scope))
            .map(|(id, member)| Standing {
                id: *id,
                scope: member.scope.clone(),
                what: member.what.clone(),
                kind: member.body.kind(),
            })
            .collect())
    }

    fn hold(&self) -> Result<std::sync::MutexGuard<'_, Table>, AxError> {
        self.table.lock().map_err(|_| {
            AxError::failure(
                AxCode::StorageFatal,
                "reach the backlog",
                "the table was left locked by a thread that died",
            )
            .with_recovery("restart this city; what was running is reported by `sprawling resume`")
        })
    }

    fn mint(&self) -> Result<BacklogId, AxError> {
        let mut table = self.hold()?;
        table.next = table.next.saturating_add(1);
        Ok(BacklogId(table.next))
    }
}

#[cfg(test)]
mod tests;

mod container;
mod process;
pub(crate) use container::ContainerRequest;

mod ceiling;
mod cgroup;
mod config;
mod host;
mod jobs;
mod member;
#[cfg(windows)]
mod native_windows;
mod report;
#[cfg(windows)]
mod run_job;
mod scratch;
mod tail;
pub mod waiting;
pub use ceiling::{Ceiling, Unapplied};
pub use cgroup::{PlatformShares, platform_shares};
pub use jobs::{RunAffinity, RunProcesses, Shares};
use member::{Body, Claim, Member, RunState, collect, storage};
pub use report::{BacklogKind, Finished, Standing, Started};
use scratch::Scratch;
use tail::Tail;
pub use tail::{Chunk, Sink, Stream};
pub use waiting::{Exit, PollBudget, Unseen};
