// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The verbs a person sends, and what each one does to the city.

use kernel::event::Scope;
use kernel::event::record::{
    Admittance, ApprovalResolved, AutonomyChanged, CityHalted, GovernedDocumentWritten,
    SpineDocumentWritten,
};
use kernel::{Address, AxCode, AxError, EventKind, Payload};

use super::super::{RunWorker, Unasked, scope_of};

impl RunWorker {
    /// Shuts a scope to new work, or opens it again.
    ///
    /// Halting shuts a scope to new work, and stops the background
    /// commands that scope already started.
    ///
    /// It still does not end a run: stopping a run in flight is
    /// `Cancel`, and one verb that did both would leave a person unable
    /// to ask for either alone. What changed (runtime-SPEC 8-28) is that
    /// a background command is not a run. It is a child process nobody
    /// could reach, because a run waiting on one is inside a system call
    /// rather than at a phase boundary; leaving it going would make
    /// "this city is stopped" false in the one case where it matters
    /// most. The refusal a halted city gives a dispatch says which scope
    /// refused and how to open it.
    pub(in crate::assembly) fn set_admission(
        &mut self,
        scope: &channels::HaltScope,
        state: Admittance,
    ) -> Result<(), AxError> {
        if matches!(state, Admittance::Halted) {
            let within = match scope {
                channels::HaltScope::City => None,
                channels::HaltScope::Building(addr) | channels::HaltScope::Workshop(addr) => {
                    Some(addr.clone())
                }
            };
            let reached = self.flight.backlog.halt(within.as_ref())?;
            if reached > 0 {
                self.note(
                    runtime::diagnostics::Level::Effect,
                    "runtime::backlog",
                    &format!("{reached} background commands were stopped"),
                );
            }
        }
        // Recorded and nothing else: the fold reads `city_halted` and
        // sets the scope, in the one place a restart reads it too.
        self.record(
            EventKind::CityHalted,
            Payload::of(&CityHalted {
                scope: scope_of(scope),
                state,
            })?,
        )
    }

    /// Which shut scope covers this address, if one does.
    ///
    /// The city covers everything; a building or a workshop covers what
    /// is inside it, by the same containment `WriteDomain` uses, so
    /// "inside" means one thing in this city rather than two.
    pub(in crate::assembly) fn halted_by(&self, addr: &Address) -> Option<Scope> {
        self.governance
            .halted
            .iter()
            .find(|scope| scope.covers(addr))
            .cloned()
    }

    /// Records who may answer for a scope from now on.
    ///
    /// The current value is folded from the ledger rather than mirrored
    /// anywhere: what the panel shows is what a replay can verify.
    pub(in crate::assembly) fn set_autonomy(
        &mut self,
        scope: &channels::HaltScope,
        autonomy: kernel::Autonomy,
    ) -> Result<(), AxError> {
        self.record(
            EventKind::AutonomyChanged,
            Payload::of(&AutonomyChanged {
                scope: scope_of(scope),
                autonomy,
            })?,
        )
    }

    /// Answers one approval item.
    ///
    /// The rule lives in `kernel::approval`: humans answer everything, a
    /// resident answers only as the appointed delegate, never the three
    /// classes, never a tainted item, and never its own action. This
    /// method is where production consults it, so a delegate answering
    /// its own item is refused on the same path the person's answer
    /// takes rather than on a parallel one.
    pub(in crate::assembly) fn answer_approval(
        &mut self,
        item: &kernel::ApprovalId,
        verdict: kernel::Ruling,
        answerer: &kernel::Answerer,
    ) -> Result<(), AxError> {
        let pending = self
            .governance
            .pending
            .get(item.as_str())
            .cloned()
            .ok_or_else(|| {
                AxError::failure(
                    AxCode::InvalidArgs,
                    "answer an approval",
                    item.as_str().to_owned(),
                )
                .with_recovery("this item is not waiting; the inbox lists the ones that are")
            })?;
        match kernel::approval::may_answer(&self.governance.autonomy, &pending, answerer) {
            kernel::AnswerVerdict::May => {}
            refused @ (kernel::AnswerVerdict::SelfApprovalBarred
            | kernel::AnswerVerdict::NotTheDelegate) => {
                return Err(AxError::failure(
                    AxCode::ApprovalDenied,
                    "answer an approval",
                    format!("{}: {refused:?}", item.as_str()),
                )
                .with_recovery(
                    "a resident answers only as the appointed delegate, and never its own \
                     action; this one is the person's to answer",
                ));
            }
        }
        // The cluster travels with the answer. The person was shown a
        // group and answered the group, so what a resumed run may do
        // without asking again is exactly that group — and reading it
        // back from the ledger is what makes the answer survive a
        // restart.
        let answered = Payload::of(&ApprovalResolved {
            id: item.clone(),
            verdict,
            cluster: pending.cluster_key.clone(),
        })?;
        // Read before the answer is recorded, because recording it is
        // what closes the item: the fold drops the origin along with the
        // pending entry, and carrying the work on is this method's job
        // rather than the record's.
        let blocked = self.governance.origins.get(item.as_str()).cloned();
        self.record(EventKind::ApprovalResolved, answered)?;
        // The cluster the person allowed and the closing of the item are
        // both folded from the line just written. Setting either field
        // here as well would be a second authority for a rule the fold
        // already holds.
        if verdict == kernel::Ruling::Allow
            && let Some(job) = blocked
        {
            // The work the person just unblocked carries on without
            // them: an answer that still needed the same command typed
            // again would make the inbox a place to acknowledge things
            // rather than a place to decide them. This is the same piece
            // of work, interrupted.
            //
            // Into a lane, so that answering one item does not hold the
            // desk for the length of the run it releases - which is how
            // a person answering a queue of approvals used to wait
            // minutes between two clicks (sprawling-SPEC.md 8-46-2). The
            // room it was interrupted in already exists, so no session
            // is opened.
            // The start reports its own refusal; there is no run id to
            // carry back here, because the person already left this desk.
            self.start_unasked(job.addr, job.task, job.goal, Unasked::Unblocked);
        }
        Ok(())
    }

    /// Writes one of the three documents that govern this city, and
    /// records that it happened.
    ///
    /// The bytes go to disk and the line goes to the Ledger. The line
    /// carries which document and how long it is, never the text: the
    /// document is on disk and readable, and copying it into history
    /// would put the same words under two authorities that later
    /// disagree.
    ///
    /// # Errors
    /// Propagates a reserved subtree that cannot be written, and a
    /// history that will not take the line announcing it.
    pub(in crate::assembly) fn put_document(
        &mut self,
        which: channels::GovernedDocument,
        body: &str,
    ) -> Result<(), AxError> {
        let which = super::super::governed_of(which);
        city::write_governed(&self.city_root, which, body)?;
        self.record(
            EventKind::GovernedDocumentWritten,
            Payload::of(&GovernedDocumentWritten {
                which: which.file().to_owned(),
                bytes: body.len(),
            })?,
        )
    }

    /// Writes one of a building's own spine documents, and records that
    /// it happened.
    ///
    /// A read-modify-write through `city::document`, so a resident
    /// writing `Roadmap.md` through `plan` cannot be lost between this
    /// frame's read and its write. `base` is the text the sender started
    /// from, and a file that has moved is refused: these documents have
    /// two writers, which is exactly the case [`Self::put_document`]
    /// says it does not have, so the two doors hold different guards.
    ///
    /// # Errors
    /// Refuses when the file is no longer the text the sender started
    /// from, when it cannot be read for any other reason, when the
    /// write will not land, and when the history will not take the line
    /// announcing it.
    pub(in crate::assembly) fn put_spine(
        &mut self,
        building: &Address,
        which: channels::SpineDocument,
        base: &str,
        body: &str,
    ) -> Result<(), AxError> {
        let name = spine_name(which);
        let path = self.city_root.join(building.as_str()).join(name);
        city::edit_document(&path, |held| {
            let on_disk = match std::fs::read(&path) {
                Ok(bytes) => bytes,
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => Vec::new(),
                Err(err) => {
                    return Err(AxError::failure(
                        AxCode::StorageFatal,
                        format!("read {name}"),
                        format!("{}: {err}", path.display()),
                    )
                    .with_recovery("fix the file's permissions, then send the change again"));
                }
            };
            if on_disk != base.as_bytes() {
                return Err(AxError::failure(
                    AxCode::InvalidArgs,
                    format!("write {name}"),
                    "the file is no longer the text you started from",
                )
                .with_recovery("read the file again and send the change once more"));
            }
            held.replace(body.as_bytes())
        })?;
        self.record(
            EventKind::SpineDocumentWritten,
            Payload::of(&SpineDocumentWritten {
                building: building.as_str().to_owned(),
                which: name.to_owned(),
                bytes: body.len(),
            })?,
        )
    }
}

/// The file name one spine document is written to.
fn spine_name(which: channels::SpineDocument) -> &'static str {
    match which {
        channels::SpineDocument::Roadmap => city::ROADMAP_FILE,
        channels::SpineDocument::Memo => city::MEMO_FILE,
        channels::SpineDocument::Handoff => city::HANDOFF_FILE,
        channels::SpineDocument::Spec => city::SPEC_FILE,
    }
}
