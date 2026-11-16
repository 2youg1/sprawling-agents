// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;

use kernel::{Address, AxCode, AxError, RunId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BacklogId(u64);

impl std::fmt::Display for BacklogId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "bg-{}", self.0)
    }
}

#[derive(Clone)]
pub struct Backlog {
    table: std::sync::Arc<std::sync::Mutex<Table>>,
    scratch: Scratch,
    window: PollBudget,
    sink: Option<Sink>,
    shares: Shares,
    affinity: RunAffinity,
    budget: crate::reserve::MachineBudget,
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
    #[must_use]
    pub fn with_shares(self, shares: Shares) -> Backlog {
        Backlog { shares, ..self }
    }

    #[must_use]
    pub fn shares(&self) -> Shares {
        self.shares
    }

    #[must_use]
    pub fn with_budget(self, budget: crate::reserve::MachineBudget) -> Backlog {
        Backlog { budget, ..self }
    }

    #[must_use]
    pub fn budget(&self) -> &crate::reserve::MachineBudget {
        &self.budget
    }

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

    pub fn stopping(&self, id: BacklogId) -> Result<bool, AxError> {
        let table = self.hold()?;
        Ok(table
            .members
            .get(&id)
            .is_some_and(|member| matches!(member.body, Body::Run(RunState::Stopping))))
    }

    pub fn leave(&self, id: BacklogId) -> Result<(), AxError> {
        let mut table = self.hold()?;
        table.members.remove(&id);
        Ok(())
    }

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

                if let Err(err) = child.stop() {
                    eprintln!("backlog cleanup retained for retry: {err}");
                }
                released = released.saturating_add(1);
            }
        }
        released
    }

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
