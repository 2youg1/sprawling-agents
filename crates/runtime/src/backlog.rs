// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The work still running while a run goes on, and the one place it is
//! stopped.
//!
//! `runtime::turn` consumes an interruption at a phase boundary. A child
//! process waited on inside a system call is not at a phase boundary, so
//! before this module a hung command could not be stopped at all. Every
//! command therefore enters this table before it is waited on: the wait
//! is a bounded number of short polls against a member somebody else can
//! reach, rather than one blocking call nobody can interrupt.
//!
//! Two facts about the wait are deliberate. It is counted rather than
//! timed, because the clock is sampled at one point in this city and a
//! module that sampled it to wait ten seconds would be the second one.
//! And a child's output goes to files rather than to pipes, because a
//! pipe buffer that fills stops the child inside a write — which is the
//! same hang this module exists to remove, wearing another name.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

use kernel::{Address, AxCode, AxError};

/// How many times the short window is polled, and how long each poll
/// waits. Their product is the ten seconds a caller blocks for before a
/// command is handed to the background.
const WINDOW_POLLS: u32 = 500;
const POLL_INTERVAL_MS: u64 = 20;

/// One member of the table, for as long as this process lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BacklogId(u64);

impl std::fmt::Display for BacklogId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "bg-{}", self.0)
    }
}

/// What the short window came to. Exhaustive rather than an optional
/// handle: "it finished" and "it is still going" are two different
/// things for the caller to say to a model, and a `None` would leave
/// which one it was to be inferred.
#[derive(Debug, PartialEq, Eq)]
pub enum Started {
    Settled {
        exit_code: i64,
        stdout: String,
        stderr: String,
    },
    Backgrounded {
        id: BacklogId,
        what: String,
    },
}

/// A member that is still running.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Standing {
    pub id: BacklogId,
    pub scope: Address,
    pub what: String,
}

/// A member that has stopped, collected once and then forgotten.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finished {
    pub id: BacklogId,
    pub what: String,
    pub exit_code: i64,
    pub stdout: String,
    pub stderr: String,
}

/// The table itself. Cloning gives another handle onto the same table,
/// which is how the assembly point can stop what a tool started without
/// either of them knowing about the other.
#[derive(Clone, Default)]
pub struct Backlog {
    table: std::sync::Arc<std::sync::Mutex<Table>>,
}

#[derive(Default)]
struct Table {
    next: u64,
    members: BTreeMap<BacklogId, Member>,
}

struct Member {
    scope: Address,
    what: String,
    child: Child,
    dir: PathBuf,
    /// Set once the short window has passed. Only a backgrounded member
    /// is collected by [`Backlog::harvest`]; one still inside its window
    /// belongs to the caller that is polling it.
    backgrounded: bool,
}

impl Backlog {
    #[must_use]
    pub fn new() -> Backlog {
        Backlog::default()
    }

    /// Starts a command, waits out the short window, and hands back
    /// either its result or a handle onto it.
    ///
    /// The command's program, arguments, working directory and
    /// environment are the caller's; this module owns only where the
    /// output goes and how the waiting is done.
    ///
    /// # Errors
    /// `E_TOOL_UNAVAILABLE` when the program cannot be started, and
    /// `E_STORAGE_FATAL` when the place its output goes cannot be
    /// written.
    pub fn run(
        &self,
        scope: &Address,
        what: String,
        mut command: Command,
    ) -> Result<Started, AxError> {
        let id = self.mint()?;
        let dir = std::env::temp_dir().join(format!("sprawling-{}-{}", std::process::id(), id.0));
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
                child,
                dir: dir.clone(),
                backgrounded: false,
            },
        )?;
        for _ in 0..WINDOW_POLLS {
            if let Some(exit_code) = self.settle(id)? {
                let (stdout, stderr) = collect(&dir);
                return Ok(Started::Settled {
                    exit_code,
                    stdout,
                    stderr,
                });
            }
            std::thread::sleep(std::time::Duration::from_millis(POLL_INTERVAL_MS));
        }
        self.hand_over(id)?;
        Ok(Started::Backgrounded { id, what })
    }

    /// Stops every member inside a scope, or every member in the city
    /// when no scope is named. Returns how many were reached.
    ///
    /// This is the sentence `runtime::turn` could not previously make
    /// true: a member is stopped where it stands, and the run that
    /// started it comes back to its next boundary rather than staying
    /// inside a system call.
    ///
    /// # Errors
    /// `E_STORAGE_FATAL` when the table cannot be reached, which means
    /// a thread died holding it.
    pub fn halt(&self, scope: Option<&Address>) -> Result<usize, AxError> {
        let mut table = self.hold()?;
        let mut reached = 0usize;
        for member in table.members.values_mut() {
            let covered = scope.is_none_or(|within| member.scope.is_within(within));
            if covered && member.child.kill().is_ok() {
                reached = reached.saturating_add(1);
            }
        }
        Ok(reached)
    }

    /// Takes every background member that has stopped, in table order.
    ///
    /// A member is reported once and then forgotten: this is what a
    /// caller appends to the tail of its next tool result, and a result
    /// delivered twice would read as the command having run twice.
    ///
    /// # Errors
    /// `E_STORAGE_FATAL` when the table cannot be reached.
    pub fn harvest(&self) -> Result<Vec<Finished>, AxError> {
        let mut table = self.hold()?;
        let mut done = Vec::new();
        let mut spent = Vec::new();
        for (id, member) in &mut table.members {
            if !member.backgrounded {
                continue;
            }
            if let Ok(Some(status)) = member.child.try_wait() {
                let (stdout, stderr) = collect(&member.dir);
                done.push(Finished {
                    id: *id,
                    what: member.what.clone(),
                    exit_code: i64::from(status.code().unwrap_or(-1)),
                    stdout,
                    stderr,
                });
                spent.push(*id);
            }
        }
        for id in spent {
            table.members.remove(&id);
        }
        Ok(done)
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

    fn enrol(&self, id: BacklogId, member: Member) -> Result<(), AxError> {
        let mut table = self.hold()?;
        table.members.insert(id, member);
        Ok(())
    }

    /// Whether this member has stopped. Removing it here is what keeps
    /// [`Backlog::harvest`] from reporting a result its own caller is
    /// about to return.
    fn settle(&self, id: BacklogId) -> Result<Option<i64>, AxError> {
        let mut table = self.hold()?;
        let Some(member) = table.members.get_mut(&id) else {
            return Ok(Some(-1));
        };
        let stopped = match member.child.try_wait() {
            Ok(Some(status)) => Some(i64::from(status.code().unwrap_or(-1))),
            Ok(None) => None,
            Err(_) => Some(-1),
        };
        if stopped.is_some() {
            table.members.remove(&id);
        }
        Ok(stopped)
    }

    fn hand_over(&self, id: BacklogId) -> Result<(), AxError> {
        let mut table = self.hold()?;
        if let Some(member) = table.members.get_mut(&id) {
            member.backgrounded = true;
        }
        Ok(())
    }
}

/// Reads what a child wrote and takes the place it wrote to away.
///
/// Failing to read is answered with what was read so far rather than
/// with an error: the command's exit code is the fact the caller is
/// owed, and a temporary file that vanished must not turn a run that
/// finished into a run that failed.
fn collect(dir: &std::path::Path) -> (String, String) {
    let read = |name: &str| {
        std::fs::read(dir.join(name))
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            .unwrap_or_default()
    };
    let out = read("out");
    let err = read("err");
    drop(std::fs::remove_dir_all(dir));
    (out, err)
}

fn storage(dir: &std::path::Path, err: &std::io::Error) -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        "keep a command's output",
        format!("{}: {err}", dir.display()),
    )
    .with_recovery("make the system temporary directory writable")
}
