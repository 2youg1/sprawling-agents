// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The door a `Command` enters by, and where its refusal goes.
//!
//! One authority for what becomes of a command somebody sent: it is
//! judged against the keys this city has already answered, carried out,
//! and — if it was refused — reported both to the diagnostic log and to
//! whoever asked. Before this door existed the worker loop wrote
//! `let _ = handle(command)`, so every refusal died in the log and the
//! page that caused it said nothing.
//!
//! Two entrances, one behaviour. `serve_one` is the desk: the loop
//! behind it lands whatever the command started. `handle` is a caller
//! with no loop — a test, the command line, the startup scan — so it
//! waits for what it started itself.

use kernel::AxError;

use super::super::RunWorker;
use crate::serving::Posted;

impl RunWorker {
    /// Carries out one command and waits for whatever it started.
    ///
    /// A `Dispatch` drives in a lane here exactly as it does from the
    /// desk; what differs is that this caller waits for the lane,
    /// because a run nobody lands is a run that never finishes
    /// (sprawling-SPEC.md 8-46-2).
    ///
    /// # Errors
    /// Refuses a command this stage does not run yet, naming what does,
    /// and propagates the failure of landing whatever the command
    /// started.
    pub fn handle(&mut self, command: channels::Command) -> Result<(), AxError> {
        let carried = self.carry_out(command, channels::Reply::nowhere());
        // The lanes are landed even when the command was refused: a
        // refusal raised after take-off leaves a run driving, and this
        // caller is the only one that will ever finish it.
        let landed = self.land_the_rest();
        carried.and(landed)
    }

    /// # Errors
    /// Refuses a command this stage does not run yet, naming what does.
    fn carry_out(
        &mut self,
        command: channels::Command,
        reply: channels::Reply,
    ) -> Result<(), AxError> {
        let name = command.name();
        let outcome = self.run_command(command, reply);
        if let Err(err) = &outcome {
            // A refused command is the first thing a person asks about,
            // so it is written at the default floor. It is written here
            // rather than at the caller, because every caller wants it.
            self.note(
                runtime::diagnostics::Level::Refuse,
                "bin::assembly",
                &format!("{name} refused: {err}; {}", err.recovery()),
            );
        }
        outcome
    }

    /// Runs one command from the desk, refusal included.
    ///
    /// A repeat under a key this city has already answered is answered
    /// with that first answer and carried out no second time: this is
    /// the door that honours the `IdemKey` every state-changing Command
    /// carries, and it judges before any effect
    /// (`commanding::entrance`, sprawling-SPEC.md 8-41).
    pub(crate) fn serve_one(&mut self, posted: Posted) {
        let Posted { command, reply } = posted;
        let key = command.idem().copied();
        if let Some(first) = key.and_then(|key| self.entrance.answered(&key)) {
            let said = super::entrance::repeated(command.name());
            self.note(runtime::diagnostics::Level::Effect, "bin::assembly", &said);
            if let Err(err) = first {
                self.hand_back(&reply, err);
            }
            return;
        }
        if let Some(key) = key {
            self.entrance.begin(key);
        }
        // The key settles here even for a verb that leaves a run going,
        // because what this command does is start one: a second frame
        // under the same key arriving while the run is still driving is
        // recognised and adds no second run, which is the whole of what
        // the door is for (sprawling-SPEC.md 8-46-2).
        let outcome = self.carry_out(command, reply.clone());
        self.entrance.settle(&outcome);
        if let Err(err) = outcome {
            self.hand_back(&reply, err);
        }
    }

    /// Hands a refusal to whoever asked for the command.
    ///
    /// `carry_out` has already written it to the diagnostic log, so the
    /// only case that earns a second line is the one a reader would
    /// otherwise misread: somebody did ask, and the answer arrived at a
    /// socket that had already closed.
    pub(in crate::assembly) fn hand_back(&mut self, reply: &channels::Reply, error: AxError) {
        match reply.refuse(error) {
            channels::Delivered::ToThePeer | channels::Delivered::NobodyAsked => {}
            channels::Delivered::PeerGone => self.note(
                runtime::diagnostics::Level::Refuse,
                "bin::assembly",
                "the refusal above reached nobody: the peer that asked had closed its socket",
            ),
        }
    }
}
