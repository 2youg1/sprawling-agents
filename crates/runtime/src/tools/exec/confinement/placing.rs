// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The adapter that places a command: a fresh copy of the working tree,
//! the platform's wrapper where the arm has one, and the budget that
//! keeps a copy from growing without a bound.
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

/// The arm in use, and the copies it has outstanding.
///
/// The copy of a command that settled is removed as soon as its wait
/// ends. A command handed to the backlog keeps its copy until the member
/// reports its ending, because the process is still in it.
pub struct Confined {
    arm: Confinement,
    scratch: Option<PathBuf>,
    outstanding: BTreeMap<BacklogId, PathBuf>,
}

/// Where one command was put, so that whoever ends the wait can end the
/// copy too.
pub struct Placed {
    copy: Option<PathBuf>,
}

impl Confined {
    /// The arm this machine gives, and where its copies go.
    pub fn detect() -> Confined {
        let offered = Offerings::this_machine();
        Confined {
            arm: Confinement::choose(&offered),
            scratch: offered.scratch,
            outstanding: BTreeMap::new(),
        }
    }

    /// An arm stated rather than found: a test's machine, or a report
    /// about one.
    pub fn with_arm(arm: Confinement, scratch: Option<PathBuf>) -> Confined {
        Confined {
            arm,
            scratch,
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

    /// Places one command under this machine's arm: a copy of `workdir`
    /// for the command to run in, wrapped in the platform's isolation
    /// where the arm is one that has a wrapper.
    ///
    /// # Errors
    /// `E_SANDBOX_DENIED` when this machine has no arm, when the arm
    /// cannot be given by any build of this crate, or when the working
    /// tree is too large or too deep to copy.
    pub fn place(
        &self,
        mut command: Command,
        workdir: &Path,
    ) -> Result<(Command, Placed), AxError> {
        match &self.arm {
            Confinement::Unavailable { missing } => Err(no_arm(*missing)),
            Confinement::WindowsJobObject => Err(no_job_object()),
            Confinement::LinuxNamespaces { wrapper } => {
                let copy = self.copy_of(workdir)?;
                let wrapped = namespaced(wrapper, &copy, workdir, &command);
                Ok((wrapped, Placed { copy: Some(copy) }))
            }
            Confinement::CopiedTree => {
                let copy = self.copy_of(workdir)?;
                command.current_dir(&copy);
                Ok((command, Placed { copy: Some(copy) }))
            }
        }
    }

    /// The wait ended and the command is over: its copy goes now.
    ///
    /// A copy that cannot be removed costs disk space rather than the
    /// answer, so the command's ending is not turned into a failure of
    /// the tool: the judgment is `backlog/member.rs`'s about the place
    /// a child's output was written, and it is recorded here rather than
    /// repeated there.
    pub fn settled(&self, placed: Placed) {
        if let Some(copy) = placed.copy {
            drop(std::fs::remove_dir_all(copy));
        }
    }

    /// The command is still going: its copy is kept against the member
    /// that will report the ending.
    pub fn handed(&mut self, id: BacklogId, placed: Placed) {
        if let Some(copy) = placed.copy {
            self.outstanding.insert(id, copy);
        }
    }

    /// Copies whose member has reported go now.
    pub fn reaped(&mut self, finished: &[Finished]) {
        for member in finished {
            if let Some(copy) = self.outstanding.remove(&member.id) {
                drop(std::fs::remove_dir_all(copy));
            }
        }
    }

    /// A fresh copy of the working tree.
    fn copy_of(&self, workdir: &Path) -> Result<PathBuf, AxError> {
        let root = self
            .scratch
            .as_deref()
            .ok_or_else(|| no_arm(Missing::ScratchDirectory))?;
        let copy = fresh(root)?;
        let mut budget = Budget::new();
        match copy_into(
            &Stage {
                from: workdir,
                into: &copy,
                copy: &copy,
            },
            0,
            &mut budget,
        ) {
            Ok(()) => Ok(copy),
            Err(fault) => {
                // The half copy goes before the refusal is returned: a
                // scratch root that grew a tree per refusal would be a
                // leak nothing reports. A removal that fails here is the
                // secondary failure and stays unstated rather than
                // replacing the one that caused it.
                remove(&copy);
                Err(fault)
            }
        }
    }
}

impl Drop for Confined {
    fn drop(&mut self) {
        // A copy left by a command that was still running when the tool
        // ended. There is no caller left to tell, and the alternative is
        // a panic, which this crate forbids: the directories stay behind
        // in the scratch root, where an operating system clears them.
        for copy in self.outstanding.values() {
            drop(std::fs::remove_dir_all(copy));
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

/// The refusal for the one arm no build of this crate can give.
///
/// Refused rather than served as the copied tree: the copied tree does
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

use copy::{Budget, Stage, copy_into, fresh, remove};

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
