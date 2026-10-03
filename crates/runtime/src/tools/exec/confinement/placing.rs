// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The adapter that places a command: a copy of the working tree synced
//! to it, the platform's wrapper where the arm has one, and the budget
//! that keeps a copy from growing without a bound.
//!
//! Where a command runs is the caller's choice and one of two words
//! ([`Placement`]); what runs there is the arm's, and the arm's promises
//! are [`super::Confinement`]'s. This file holds no policy about which
//! arm a machine should get.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use kernel::{AxCode, AxError};
use serde_json::{Map, Value};

use crate::backlog::{BacklogId, Finished};

use super::{Confinement, Missing, Offerings};

/// Where a host command runs. The safe arm is what a call gets when it
/// says nothing, because an agent that already knew a command was
/// dangerous would not need to be told to sandbox it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    /// In the confinement [`Confinement::detect`](super::Confinement::detect) reports: a copy of
    /// the working tree, and the platform's own isolation where there is
    /// a wrapper for it.
    Sandbox,
    /// Where it stands, on the room the run may write in. This is the
    /// placement a person decides on; a command that needs it asks for
    /// it by name.
    Host,
}

/// Reads the placement out of the call. Absent is [`Placement::Sandbox`].
pub fn parse_placement(args: &Map<String, Value>) -> Result<Placement, AxError> {
    match args.get("where") {
        None => Ok(Placement::Sandbox),
        Some(Value::String(said)) if said == "sandbox" => Ok(Placement::Sandbox),
        Some(Value::String(said)) if said == "host" => Ok(Placement::Host),
        Some(other) => Err(AxError::failure(
            AxCode::InvalidArgs,
            "run exec",
            format!("unrecognised placement: {other}"),
        )
        .with_recovery(
            "leave `where` out to run in the sandbox, or name one of `sandbox` and \
             `host`",
        )),
    }
}

impl Placement {
    /// This placement, when the run's write limit opens it.
    ///
    /// A command on the host could change, remove or link any file it
    /// reaches, and nothing on this machine makes the existing ones
    /// read-only to it without administrator rights, so under `Create`
    /// only the copy is open; what a command writes there stays there
    /// (`crates/runtime/spec/Tools.lean` §8-55, runtime D11).
    ///
    /// # Errors
    /// The refusal of `kernel::gate::replacing`, naming the run's
    /// domain, before any process starts.
    pub fn opened_by(self, setup: &crate::tools::ExecSetup) -> Result<Placement, AxError> {
        match self {
            Placement::Sandbox => Ok(self),
            Placement::Host => {
                match kernel::gate::replacing(setup.policy.now().write, &setup.domain) {
                    kernel::GateOutcome::Allow => Ok(self),
                    kernel::GateOutcome::Deny { refusal } => Err(*refusal),
                    kernel::GateOutcome::Ask { question } => Err(*question),
                }
            }
        }
    }
}

/// The arm in use, the copy kept for the next command, and the copies
/// commands still hold.
///
/// The copy of a command that settled is kept for the next command,
/// which syncs it rather than making another (runtime D10). A
/// command handed to the backlog keeps its copy until the member reports
/// its ending, because the process is still in it.
pub struct Confined {
    arm: Confinement,
    scratch: Option<PathBuf>,
    idle: Option<Mirror>,
    outstanding: BTreeMap<BacklogId, Mirror>,
}

/// One copy, and the working directory it mirrors.
struct Mirror {
    of: PathBuf,
    at: PathBuf,
}

/// Where one command was put, so that whoever ends the wait can hand
/// the copy back.
pub struct Placed {
    copy: Option<Mirror>,
    work: storage::FileWork,
}

impl Placed {
    /// What putting the copy in place cost the filesystem (`crates/storage/Spec.lean`
    /// §8-31).
    pub fn work(&self) -> storage::FileWork {
        self.work
    }
}

impl Confined {
    /// The arm this machine gives, and where its copies go.
    pub fn detect() -> Confined {
        let offered = Offerings::this_machine();
        Confined {
            arm: Confinement::choose(&offered),
            scratch: offered.scratch,
            idle: None,
            outstanding: BTreeMap::new(),
        }
    }

    /// An arm stated rather than found: a test's machine, or a report
    /// about one.
    pub fn with_arm(arm: Confinement, scratch: Option<PathBuf>) -> Confined {
        Confined {
            arm,
            scratch,
            idle: None,
            outstanding: BTreeMap::new(),
        }
    }

    pub fn arm(&self) -> &Confinement {
        &self.arm
    }

    /// What this arm promises and does not, for the caller's own
    /// description of itself.
    pub fn statement(&self) -> String {
        self.arm.statement()
    }

    /// Places one command under this machine's arm: the tool's copy of
    /// `workdir`, synced to it, for the command to run in, wrapped in the
    /// platform's isolation where the arm is one that has a wrapper.
    ///
    /// # Errors
    /// `E_SANDBOX_DENIED` when this machine has no arm, when the arm
    /// cannot be given by any build of this crate, or when the working
    /// tree is too large or too deep to copy.
    pub fn place(
        &mut self,
        mut command: Command,
        workdir: &Path,
    ) -> Result<(Command, Placed), AxError> {
        let wrapper = match &self.arm {
            Confinement::Unavailable { missing } => return Err(no_arm(*missing)),
            Confinement::WindowsJobObject => return Err(no_job_object()),
            Confinement::LinuxNamespaces { wrapper } => Some(wrapper.clone()),
            Confinement::CopiedTree => None,
        };
        let (copy, work) = self.synced(workdir)?;
        let command = match wrapper {
            Some(wrapper) => namespaced(&wrapper, &copy.at, workdir, &command),
            None => {
                command.current_dir(&copy.at);
                command
            }
        };
        Ok((
            command,
            Placed {
                copy: Some(copy),
                work,
            },
        ))
    }

    /// The wait ended and the command is over: its copy is kept for the
    /// next command, or goes when one is kept already.
    pub fn settled(&mut self, placed: Placed) {
        if let Some(copy) = placed.copy {
            self.keep(copy);
        }
    }

    /// The command is still going: its copy is kept against the member
    /// that will report the ending.
    pub fn handed(&mut self, id: BacklogId, placed: Placed) {
        if let Some(copy) = placed.copy {
            self.outstanding.insert(id, copy);
        }
    }

    /// Copies whose member has reported are kept for the next command,
    /// or go.
    pub fn reaped(&mut self, finished: &[Finished]) {
        for member in finished {
            if let Some(copy) = self.outstanding.remove(&member.id) {
                self.keep(copy);
            }
        }
    }

    /// Keeps `copy` for the next command when no copy is kept yet, and
    /// removes it otherwise.
    ///
    /// A copy that cannot be removed costs disk space rather than the
    /// answer, so the command's ending is not turned into a failure of
    /// the tool: the judgment is `backlog/member.rs`'s about the place
    /// a child's output was written, and it is recorded here rather than
    /// repeated there.
    fn keep(&mut self, copy: Mirror) {
        match self.idle {
            None => self.idle = Some(copy),
            Some(_) => remove(&copy.at),
        }
    }

    /// The copy kept for the next command, or a fresh one, brought to
    /// what `workdir` holds.
    fn synced(&mut self, workdir: &Path) -> Result<(Mirror, storage::FileWork), AxError> {
        let root = self
            .scratch
            .as_deref()
            .ok_or_else(|| no_arm(Missing::ScratchDirectory))?;
        let copy = match self.idle.take() {
            Some(kept) if kept.of == workdir => kept,
            Some(other) => {
                remove(&other.at);
                Mirror {
                    of: workdir.to_path_buf(),
                    at: fresh(root)?,
                }
            }
            None => Mirror {
                of: workdir.to_path_buf(),
                at: fresh(root)?,
            },
        };
        let mut budget = Budget::new();
        match mirror(
            &Stage {
                from: workdir,
                into: &copy.at,
                copy: &copy.at,
            },
            0,
            &mut budget,
        ) {
            Ok(()) => Ok((copy, budget.work())),
            Err(fault) => {
                // The half-synced copy goes before the refusal is
                // returned: a scratch root that grew a tree per refusal
                // would be a leak nothing reports. A removal that fails
                // here is the secondary failure and stays unstated rather
                // than replacing the one that caused it.
                remove(&copy.at);
                Err(fault)
            }
        }
    }
}

impl Drop for Confined {
    fn drop(&mut self) {
        // The copy kept for a next command that will not come, and those
        // of commands still running when the tool ended. There is no
        // caller left to tell, and the alternative is a panic, which this
        // crate forbids: a directory that will not go stays behind in the
        // scratch root, where an operating system clears it.
        for copy in self.idle.iter().chain(self.outstanding.values()) {
            remove(&copy.at);
        }
    }
}

/// The refusal for a machine that cannot take a command at all.
fn no_arm(missing: Missing) -> AxError {
    AxError::failure(
        AxCode::SandboxDenied,
        "run outside the confinement",
        missing.phrase(),
    )
    .with_recovery(missing.recovery())
}

/// The refusal for the one arm no build of this crate constructs.
///
/// No build constructs it because a job object made through the safe
/// interface keeps neither a job-wide resource limit nor the processes a
/// child starts before it joins the job (`crates/runtime/spec/Tools/Exec.lean`
/// §8-13-2). Refused rather than served as the copied tree: the copied tree does
/// not isolate the network, and answering for an arm that does would
/// tell a caller its network was closed while it was open.
fn no_job_object() -> AxError {
    AxError::failure(
        AxCode::SandboxDenied,
        "run under a job object",
        "this build makes no job object",
    )
    .with_recovery(
        "run under the copied tree instead: it copies the working directory and does \
         not isolate the network, so keep the command off the network or run it on a \
         machine that has the arm",
    )
}

mod copy;

use copy::{Budget, Stage, fresh, mirror, remove};

/// The command as the platform's wrapper runs it: the whole machine
/// readable, the copy bound over the working directory, and every
/// namespace the wrapper knows how to separate.
fn namespaced(wrapper: &Path, copy: &Path, workdir: &Path, command: &Command) -> Command {
    let mut wrapped = Command::new(wrapper);
    wrapped
        .current_dir(workdir)
        .arg("--ro-bind")
        .arg("/")
        .arg("/")
        .arg("--bind")
        .arg(copy)
        .arg(workdir)
        .arg("--chdir")
        .arg(workdir)
        .arg("--proc")
        .arg("/proc")
        .arg("--dev")
        .arg("/dev")
        .arg("--unshare-all")
        .arg("--die-with-parent")
        .arg("--")
        .arg(command.get_program());
    for arg in command.get_args() {
        wrapped.arg(arg);
    }
    for (name, value) in command.get_envs() {
        match value {
            Some(value) => wrapped.env(name, value),
            None => wrapped.env_remove(name),
        };
    }
    wrapped
}
