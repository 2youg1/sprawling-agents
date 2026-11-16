// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::event::Scope;
use kernel::event::record::{
    Admittance, ApprovalResolved, AutonomyChanged, CityHalted, GovernedDocumentWritten, RunStarted,
    SpineDocumentWritten,
};
use kernel::{Address, AxCode, AxError, EventKind, Payload, RunId, Seq};

use super::super::{RunWorker, Unasked, scope_of};

impl RunWorker {

    pub(in crate::worker) fn set_admission(
        &mut self,
        scope: &wire::HaltScope,
        state: Admittance,
    ) -> Result<(), AxError> {
        if matches!(state, Admittance::Halted) {
            let within = match scope {
                wire::HaltScope::City => None,
                wire::HaltScope::Building(addr) | wire::HaltScope::Workshop(addr) => {
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

        self.record(
            EventKind::CityHalted,
            Payload::of(&CityHalted {
                scope: scope_of(scope),
                state,
            })?,
        )
    }

    pub(in crate::worker) fn halted_by(&self, addr: &Address) -> Option<Scope> {
        self.governance
            .halted
            .iter()
            .find(|scope| scope.covers(addr))
            .cloned()
    }

    pub(in crate::worker) fn set_autonomy(
        &mut self,
        scope: &wire::HaltScope,
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

    pub(in crate::worker) fn answer_approval(
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
                     action; this one is the User's to answer",
                ));
            }
        }

        let answered = Payload::of(&ApprovalResolved {
            id: item.clone(),
            verdict,
            cluster: pending.cluster_key.clone(),
        })?;

        let blocked = self.governance.origins.get(item.as_str()).cloned();
        self.record(EventKind::ApprovalResolved, answered)?;

        if verdict == kernel::Ruling::Allow
            && let Some(job) = blocked
            && let Some(sent) = self.sent_out(job.run)?
        {

            self.start_unasked(job.addr, sent.task, sent.goal, Unasked::Unblocked);
        }
        Ok(())
    }

    fn sent_out(&mut self, run: RunId) -> Result<Option<RunStarted>, AxError> {
        let dir = kernel::layout::CityLayout::new(&self.city_root).ledger();
        self.index
            .refresh(&dir)
            .map_err(storage::StorageError::into_ax)?;
        let mut seqs: Vec<Seq> = self.index.run_seqs_before(run, None).collect();
        seqs.reverse();
        let mut reader = self.index.reader(&dir);
        for seq in seqs {
            let line = reader
                .line_at(seq)
                .map_err(storage::StorageError::into_ax)?;
            match storage::read_line(&line) {
                Ok(storage::CheckedLine::Known(record))
                    if record.kind() == EventKind::RunStarted =>
                {
                    return record.data().read::<RunStarted>().map(Some);
                }
                Ok(storage::CheckedLine::Known(_) | storage::CheckedLine::IgnoredUnknown(_)) => {}
                Err(fault) => return Err(fault.into_ax(seq.value().saturating_add(1))),
            }
        }
        Ok(None)
    }

    pub(in crate::worker) fn put_document(
        &mut self,
        which: wire::GovernedDocument,
        base: &str,
        body: &str,
    ) -> Result<(), AxError> {
        let which = super::super::governed_of(which);
        city::write_governed(&self.city_root, which, base, body)?;
        let naming = match which {
            city::Governed::Clerk => None,
            city::Governed::Mayor | city::Governed::Preferences => self.naming_version()?,
        };
        self.governed_written(which, body.len(), naming)
    }

    pub(in crate::worker) fn naming_version(&self) -> Result<Option<kernel::B3Hash>, AxError> {
        match city::read_naming(&self.city_root)? {
            Ok(naming) => Ok(Some(naming.version()?)),
            Err(city::Unreadable { .. }) => Ok(None),
        }
    }

    pub(in crate::worker) fn put_identity(
        &mut self,
        card: &wire::IdentityCard,
        base: &str,
    ) -> Result<(), AxError> {
        let edit = super::super::naming_edit_of(card);
        let written = city::write_naming(&self.city_root, &edit, base)?;
        self.governed_written(
            edit.document(),
            written.bytes,
            Some(written.naming.version()?),
        )
    }

    fn governed_written(
        &mut self,
        which: city::Governed,
        bytes: usize,
        naming: Option<kernel::B3Hash>,
    ) -> Result<(), AxError> {
        self.record(
            EventKind::GovernedDocumentWritten,
            Payload::of(&GovernedDocumentWritten {
                which: which.file().to_owned(),
                bytes,
                naming,
                at: None,
                removed: false,
            })?,
        )
    }

    pub(in crate::worker) fn put_spine(
        &mut self,
        building: &Address,
        which: wire::SpineDocument,
        base: &str,
        body: &str,
    ) -> Result<(), AxError> {
        let name = spine_name(which);
        let path = self.city_root.join(building.as_str()).join(name);
        city::edit_against(&path, base.as_bytes(), body.as_bytes())?;
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

fn spine_name(which: wire::SpineDocument) -> &'static str {
    match which {
        wire::SpineDocument::Roadmap => city::ROADMAP_FILE,
        wire::SpineDocument::Memo => city::MEMO_FILE,
        wire::SpineDocument::Handoff => city::HANDOFF_FILE,
        wire::SpineDocument::Spec => city::SPEC_FILE,
    }
}
