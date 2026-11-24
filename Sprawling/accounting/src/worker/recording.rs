// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{Address, AxError, EventDraft, EventKind, EventRef, Ledger, Payload, RunId, Seq};

use crate::effect;
use crate::worker::booking::OpenClaims;

use super::RunWorker;

pub(in crate::worker) struct Stamping<'a, L> {
    pub(in crate::worker) ledger: &'a mut L,
    pub(in crate::worker) command: Option<kernel::IdemKey>,

    pub(in crate::worker) clock: &'a (dyn crate::Clock + Send + Sync),
}

impl<L: Ledger> Stamping<'_, L> {
    pub(in crate::worker) fn record_for(
        &mut self,
        run: RunId,
        line: effect::Line,
    ) -> Result<(), AxError> {
        let effect::Line {
            who,
            addr,
            kind,
            data,
        } = line;
        self.ledger.append(EventDraft {
            run,
            t: self.clock.now()?,
            who,
            addr: Some(addr),
            kind,
            data: super::commanding::entrance::stamped(self.command, data)?,
            ig: false,
        })?;
        Ok(())
    }
}

pub(in crate::worker) struct RunOpening<'a, L> {
    pub(in crate::worker) ledger: &'a mut L,
    pub(in crate::worker) command: Option<kernel::IdemKey>,
    pub(in crate::worker) session: Option<Seq>,
    pub(in crate::worker) image_input: bool,
}

impl<L: Ledger> Ledger for RunOpening<'_, L> {
    fn append(&mut self, draft: EventDraft) -> Result<EventRef, AxError> {
        self.ledger.append(self.stamp(draft)?)
    }

    fn append_all(&mut self, drafts: Vec<EventDraft>) -> Result<Vec<EventRef>, AxError> {
        let stamped = drafts
            .into_iter()
            .map(|draft| self.stamp(draft))
            .collect::<Result<_, _>>()?;
        self.ledger.append_all(stamped)
    }
}

impl<L> RunOpening<'_, L> {
    fn stamp(&self, mut draft: EventDraft) -> Result<EventDraft, AxError> {
        draft.data = in_session(self.session, draft.data)?;
        if draft.kind == EventKind::RunStarted {
            draft.data = super::commanding::entrance::stamped(self.command, draft.data)?;
            if self.image_input {
                let mut fields = draft.data.as_map().clone();
                fields.insert("image_input".to_owned(), true.into());
                draft.data = Payload::new(fields)?;
            }
        }
        Ok(draft)
    }
}

fn in_session(session: Option<Seq>, data: Payload) -> Result<Payload, AxError> {
    let Some(began) = session else {
        return Ok(data);
    };
    let mut fields = data.as_map().clone();
    fields.insert("session_began".to_owned(), began.value().into());
    Payload::new(fields)
}

#[derive(Clone)]
pub(in crate::worker) struct Notes(
    std::sync::Arc<std::sync::Mutex<runtime::diagnostics::Diagnostics>>,
);

impl Notes {
    pub(in crate::worker) fn over(log: runtime::diagnostics::Diagnostics) -> Notes {
        Notes(std::sync::Arc::new(std::sync::Mutex::new(log)))
    }

    pub(in crate::worker) fn write(
        &self,
        level: runtime::diagnostics::Level,
        seq: kernel::Seq,
        module: &str,
        message: &str,
    ) {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .write(
                level,
                runtime::diagnostics::Site {
                    run: RunId::CITY,
                    seq,
                    module,
                },
                message,
            );
    }
}

impl RunWorker {
    pub(super) fn note(&mut self, level: runtime::diagnostics::Level, module: &str, message: &str) {
        self.log
            .write(level, self.ledger.position(), module, message);
    }

    pub(super) fn record(&mut self, kind: EventKind, data: Payload) -> Result<(), AxError> {
        self.record_where(kind, None, data)
    }

    pub(super) fn note_lineage(
        &mut self,
        addr: &Address,
        run: RunId,
        origin: kernel::Origin,
        resume: Option<Seq>,
    ) -> Result<(), AxError> {
        let t = self.clock.now()?;
        let mut draft =
            runtime::fork::fork_draft(origin, run, addr.clone(), t, "owner".to_owned())?;
        draft.data = in_session(resume, draft.data)?;
        self.ledger.append(draft)?;
        if resume.is_none() {
            self.origins.spent(addr);
        }
        Ok(())
    }

    pub(super) fn record_at(
        &mut self,
        kind: EventKind,
        addr: Address,
        data: Payload,
    ) -> Result<(), AxError> {
        self.record_where(kind, Some(addr), data)
    }

    fn record_where(
        &mut self,
        kind: EventKind,
        addr: Option<Address>,
        data: Payload,
    ) -> Result<(), AxError> {
        let data = super::commanding::entrance::stamped(self.doorstep.entrance.carrying(), data)?;
        let draft = EventDraft {
            run: RunId::CITY,
            t: self.clock.now()?,
            who: "owner".to_owned(),
            addr: addr.clone(),
            kind,
            data: data.clone(),
            ig: false,
        };
        self.ledger.append(draft)?;

        self.absorb(kind, RunId::CITY, addr.as_ref(), &data)
    }

    pub(super) fn record_for(&mut self, run: RunId, line: effect::Line) -> Result<(), AxError> {
        let (kind, addr, data) = self.append_for(run, line)?;
        self.absorb(kind, run, Some(&addr), &data)
    }

    pub(super) fn record_closing(
        &mut self,
        run: RunId,
        closing: effect::Closing,
        open: &mut OpenClaims,
    ) -> Result<(), AxError> {
        let (kind, addr, data) = self.append_for(run, closing.line)?;
        if let Some(node) = &closing.closes {
            open.close(node);
        }
        self.absorb(kind, run, Some(&addr), &data)
    }

    fn append_for(
        &mut self,
        run: RunId,
        line: effect::Line,
    ) -> Result<(EventKind, Address, Payload), AxError> {
        let effect::Line {
            who,
            addr,
            kind,
            data,
        } = line;
        let data = super::commanding::entrance::stamped(self.doorstep.entrance.carrying(), data)?;
        self.ledger.append(EventDraft {
            run,
            t: self.clock.now()?,
            who,
            addr: Some(addr.clone()),
            kind,
            data: data.clone(),
            ig: false,
        })?;
        Ok((kind, addr, data))
    }

    pub(super) fn show_relayed(&mut self, written: Vec<EventDraft>) {
        let runs: Vec<RunId> = written.iter().map(|line| line.run).collect();
        for line in written {
            if let Err(err) = self.absorb(line.kind, line.run, line.addr.as_ref(), &line.data) {
                self.note(
                    runtime::diagnostics::Level::Refuse,
                    "crate::worker::relay",
                    &format!("a fold refused a {:?} line a lane wrote: {err}", line.kind),
                );
            }
            if let Err(err) = self.deliver_sent(&line) {
                self.note(
                    runtime::diagnostics::Level::Refuse,
                    "collab::inbox",
                    &format!("a signal a lane sent reached no room: {err}"),
                );
            }
        }

        let mut seen = Vec::new();
        for run in runs {
            if seen.contains(&run) {
                continue;
            }
            seen.push(run);
            if let Err(err) = self.hand_over_at_call(run) {
                self.note(
                    runtime::diagnostics::Level::Refuse,
                    "collab::delegate",
                    &format!("work {run} handed down could not be read: {err}"),
                );
            }
        }

        self.answer_knocks();
    }

    fn absorb(
        &mut self,
        kind: EventKind,
        run: RunId,
        addr: Option<&Address>,
        data: &Payload,
    ) -> Result<(), AxError> {
        self.doorstep.entrance.absorb(kind, data);
        [
            self.origins.absorb(kind, run, addr, data),
            self.governance.absorb(kind, run, addr, data),
            self.planning.absorb(kind, addr, data),
            self.flight.gate.booked.absorb(kind, addr, data),
            super::collaborating::register_goal(&mut self.collaborating.goals, kind, data),
            self.credentials.absorb(kind, run, addr, data),
        ]
        .into_iter()
        .collect()
    }
}
