// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The control surface: which frames intervene in work already running, and
//! what an intervention owes when it does.
//!
//! A human's Steer arrives here, not through an Inbox;
//! an Agent's Steer arrives as a high-priority Signal. Both land in the same
//! place - the result envelope - so the model only ever learns one shape.
//!
//! This module decides and returns; it holds no Ledger handle and writes
//! nothing. The obligation it reports is discharged by the assembly layer.

use kernel::{AxCode, AxError, RunId};

use crate::command::Command;

/// The verbs the control surface shows. Three interventions plus `Release`,
/// the return path that `Halt` needs in order not to be a trap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intervention {
    Steer,
    Cancel,
    Halt,
    Release,
}

impl Intervention {
    /// The verb as the interface spells it. This is also the microcopy
    /// authority: one verb, one word, everywhere.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Steer => "Steer",
            Self::Cancel => "Cancel",
            Self::Halt => "Halt",
            Self::Release => "Release",
        }
    }
}

/// What the control surface makes of one Command.
#[derive(Debug)]
pub enum ControlVerdict {
    Intervene {
        verb: Intervention,
        /// The Run being interrupted, when the verb names one. `Halt` and
        /// `Release` act on a scope and leave this empty.
        run: Option<RunId>,
        /// Whether the assembly layer owes a `handoff_written` before the
        /// intervention is complete.
        must_write_handoff: bool,
    },
    NotAnIntervention,
    Refuse(AxError),
}

/// Classifies one Command.
///
/// Pure and exhaustive: every one of the Commands is answered, so
/// a new Command cannot be added without deciding whether it interrupts
/// anything. That question is the reason this module is not part of the
/// listener - the listener could not answer it, and would not know it had
/// been asked.
#[must_use]
pub fn classify(command: &Command) -> ControlVerdict {
    match *command {
        Command::Steer { run, ref text, .. } => {
            if text.trim().is_empty() {
                return ControlVerdict::Refuse(
                    AxError::failure(
                        AxCode::WireMismatch,
                        "steer a Run",
                        "the steer carries no text",
                    )
                    .with_recovery("say what should change, or use Cancel to stop the Run"),
                );
            }
            intervene(Intervention::Steer, Some(run))
        }
        Command::Cancel { run, .. } => intervene(Intervention::Cancel, Some(run)),
        Command::Halt { .. } => scope_intervention(Intervention::Halt),
        Command::Release { .. } => scope_intervention(Intervention::Release),
        Command::Dispatch { .. }
        | Command::Wake { .. }
        | Command::ProbeEndpoint { .. }
        | Command::ConfigureBuilding { .. }
        | Command::AttachEndpoint { .. }
        | Command::SelectModel { .. }
        // Starting a session reaches no run in flight: the assembly
        // refuses it with `E_BUSY` while one is going, and that refusal
        // belongs where the run is, not here.
        | Command::OpenSession { .. }
        | Command::CreateBuilding { .. }
        // Removing a building with a run going is refused where the
        // runs are, so this verb never reaches one.
        | Command::RemoveBuilding { .. }
        | Command::PutSecret { .. }
        // Opening a file manager reaches nothing a run is doing.
        | Command::Reveal { .. }
        // Putting a discarded file back only creates a file that is
        // absent: memory refuses a link on the path and any file already
        // at the target, so a run writing there is never overwritten.
        | Command::RestoreDiscard { .. }
        // Installing a tool and looking at this machine again are both
        // about the machine rather than about the city: neither reaches
        // a run, and neither leaves a scene for anybody to hand over.
        | Command::DoctorInstall { .. }
        | Command::DoctorRefresh { .. }
        | Command::BatchByBuilding { .. }
        | Command::Approve { .. }
        // Giving a question to a resident changes who answers it. The
        // run that asked stays stopped either way, so nothing is being
        // interrupted and there is no scene to hand over.
        | Command::HandOff { .. }
        | Command::SetAutonomy { .. }
        // Pausing a standing goal stops the city taking new work; it
        // does not reach into a run that is already going, and a verb
        // that owed a Handoff would be claiming it had.
        | Command::Pursue { .. }
        // Writing a document that governs the city changes what the
        // next run is given, and reaches into no run that is already
        // going: the frozen prefix of a live run was assembled before
        // this frame arrived.
        | Command::PutDocument { .. }
        | Command::PutIdentity { .. }
        // A building's rules and the city's own layer are read by the
        // next dispatch; a live run holds the policy it was frozen with.
        | Command::PutRules(_)
        // Taking a file back is refused while a run works in that
        // building, so it reaches no run that is going.
        | Command::RestoreFile { .. }
        // Where the person stands in the guide is the page's place, not
        // anything a run reads.
        | Command::PutGuide { .. }
        | Command::ConfigureCity(_)
        // A building's own spine documents take the same reading, and
        // one more: they have a second writer, so the frame carries the
        // text it started from and a file that moved is refused.
        | Command::PutSpine { .. }
        // Saving any document, and landing what a person accepted of a
        // proposal, takes the same reading: a file changes on disk under
        // a baseline, and no run that is going is reached by it.
        | Command::PutRange(_)
        | Command::DecideProposals(_)
        // What a person settled about their own reading of the city
        // reaches no run at all, and a skill written onto a shelf is
        // admitted by the run after this one.
        | Command::PutPreferences { .. }
        | Command::PutShelved { .. }
        // Connecting an outside application changes what tools the next
        // run is offered and reaches nothing a run is already doing: a
        // live run holds the tool table it was assembled with, and an
        // application connected halfway through it joins the one after.
        | Command::ConnectToolkit { .. }
        // A session's name is what a page shows. A policy change is a
        // ledger line the run under way reads at its own next safe
        // point, so it interrupts no turn and owes no handoff.
        | Command::NameSession(_)
        | Command::ChangeRunPolicy(_)
        // The remote door decides who reaches the city from outside; it
        // reaches no run. Closing it ends remote sessions, which are
        // connections, not turns.
        | Command::OpenRemoteDoor { .. }
        | Command::ReplaceCityKey { .. }
        | Command::ConfirmRemoteDoor { .. }
        | Command::CloseRemoteDoor { .. }
        // A run that already redeemed a key holds the value it read, and
        // the next one finds the vault without it; no turn is cut.
        | Command::ForgetSecret { .. }
        | Command::Auth { .. } => ControlVerdict::NotAnIntervention,
    }
}

/// Interrupting a turn always owes a Handoff: the next holder - a person
/// taking over, or the Run itself after a resume - must find the complete
/// scene rather than a truncated one.
fn intervene(verb: Intervention, run: Option<RunId>) -> ControlVerdict {
    ControlVerdict::Intervene {
        verb,
        run,
        must_write_handoff: true,
    }
}

/// Halting a scope stops future work rather than interrupting a turn in
/// progress, so there is no scene to hand over.
fn scope_intervention(verb: Intervention) -> ControlVerdict {
    ControlVerdict::Intervene {
        verb,
        run: None,
        must_write_handoff: false,
    }
}
