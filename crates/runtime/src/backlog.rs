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
//! (runtime-SPEC 8-28-2). One table, one `halt`, so a stopped scope has
//! nothing left going in it under either name.

use std::collections::BTreeMap;
use std::process::{Command, Stdio};

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
    pub fn new() -> Backlog {
        Backlog::default()
    }

    /// A table whose callers block for this window before a command is
    /// handed to the background.
    #[must_use]
    pub fn with_window(window: PollBudget) -> Backlog {
        Backlog {
            table: std::sync::Arc::default(),
            scratch: Scratch::open(),
            window,
            sink: None,
        }
    }

    /// The same table, handing what a command writes to `sink` while
    /// the command is still inside its window (runtime-SPEC 8-28-3).
    #[must_use]
    pub fn with_sink(self, sink: Sink) -> Backlog {
        Backlog {
            sink: Some(sink),
            ..self
        }
    }

    /// Starts a command, waits out the short window, and hands back
    /// either its result or a handle onto it.
    ///
    /// The command's program, arguments, working directory and
    /// environment are the caller's; this module owns only where the
    /// output goes and how the waiting is done. A command that outlives
    /// its window is owed to `owner` and to no other run.
    ///
    /// # Errors
    /// `E_TOOL_UNAVAILABLE` when the program cannot be started, and
    /// `E_STORAGE_FATAL` when the place its output goes cannot be
    /// written.
    pub fn run(
        &self,
        owner: RunId,
        scope: &Address,
        what: String,
        mut command: Command,
    ) -> Result<Started, AxError> {
        let id = self.mint()?;
        let dir = self.scratch.dir(id);
        std::fs::create_dir_all(&dir).map_err(|err| storage(&dir, &err))?;
        let out = std::fs::File::create(dir.join("out")).map_err(|err| storage(&dir, &err))?;
        let errs = std::fs::File::create(dir.join("err")).map_err(|err| storage(&dir, &err))?;
        command
            .stdin(Stdio::null())
            .stdout(Stdio::from(out))
            .stderr(Stdio::from(errs));
        let child = command.spawn().map_err(|err| {
            AxError::failure(
                AxCode::ToolUnavailable,
                "start a command",
                format!("{what}: {err}"),
            )
            .with_recovery("check the program name, or use the shell arm")
        })?;
        self.enrol(
            id,
            Member {
                scope: scope.clone(),
                what: what.clone(),
                body: Body::Command {
                    child,
                    dir: dir.clone(),
                    claim: Claim::Window(owner),
                    tail: Tail::default(),
                },
            },
        )?;
        let mut tail = Tail::default();
        for _ in 0..self.window.polls() {
            if let Some(exit) = self.settle(id)? {
                let (stdout, stderr) = collect(&dir);
                return Ok(Started::Settled {
                    exit,
                    stdout,
                    stderr,
                });
            }
            if let Some(sink) = &self.sink {
                sink.deliver(tail.take(&dir, (owner, id), self.window.read_per_poll()));
            }
            std::thread::sleep(self.window.interval());
        }
        self.hand_over(id, tail)?;
        Ok(Started::Backgrounded { id, what })
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
        for member in table.members.values_mut() {
            let covered = scope.is_none_or(|within| member.scope.is_within(within));
            if !covered {
                continue;
            }
            let stopped = match &mut member.body {
                Body::Command { child, .. } => child.kill().is_ok(),
                Body::Run(state) => {
                    *state = RunState::Stopping;
                    true
                }
            };
            if stopped {
                reached = reached.saturating_add(1);
            }
        }
        Ok(reached)
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
        for (id, member) in &mut table.members {
            let Body::Command {
                child,
                dir,
                claim,
                tail,
            } = &mut member.body
            else {
                continue;
            };
            match claim {
                Claim::Run(run) if *run == owner => {}
                Claim::Nobody => {}
                Claim::Window(_) | Claim::Run(_) => continue,
            }
            let Some(exit) = Exit::polled(child) else {
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
                && *claim == Claim::Run(owner)
            {
                *claim = Claim::Nobody;
                // `Child::kill` answers `Ok` for a child that already
                // exited, so an error here is a kill the system refused.
                // The member is still owed to nobody and the next harvest
                // reaps it once it stops; the caller is a drop with
                // nobody to hand the refusal to.
                drop(child.kill());
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

mod jobs;
mod member;
mod report;
mod scratch;
mod tail;
pub mod waiting;
pub use jobs::RunProcesses;
use member::{Body, Claim, Member, RunState, collect, storage};
pub use report::{BacklogKind, Finished, Standing, Started};
use scratch::Scratch;
use tail::Tail;
pub use tail::{Chunk, Sink, Stream};
pub use waiting::{Exit, PollBudget, Unseen};
