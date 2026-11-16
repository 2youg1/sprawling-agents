// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::event::Who;
use kernel::event::record::{
    ModelCalled, RunPolicyChanged, RunStarted, SessionNamed, SessionOpened,
};
use kernel::{Address, AxCode, AxError, EventKind, Payload, Seq};

use super::super::RunWorker;

impl RunWorker {
    pub(in crate::worker) fn open_session(
        &mut self,
        addr: &Address,
        carry: wire::Carry,
        from: Option<kernel::Origin>,
    ) -> Result<(), AxError> {
        if let Some(working) = self.collaborating.rooms.worked_by(addr) {
            return Err(AxError::failure(
                AxCode::Busy,
                "start a new session",
                format!("{}: {working} is running there", addr.as_str()),
            )
            .with_recovery(
                "stop that run first (`/stop`), then start the session (`/new`); \
                 a session cannot begin underneath the run writing it",
            ));
        }

        if let Some(origin) = from {
            self.origin_is_real(origin)?;
        }
        match carry {
            wire::Carry::Nothing => city::clear_session(&self.city_root, addr)?,

            wire::Carry::Handoff => city::forget_shape(&self.city_root, addr)?,
        }
        if let Some(origin) = from
            && let Some(shape) = self.origin_shape(origin)?
        {
            let mut frozen =
                city::SessionShape::new(&shape.model, shape.effort, shape.generation.clone());
            frozen.endpoint = shape.endpoint;
            city::write_session(&self.city_root, addr, &frozen)?;
            if let Some(naming) = shape.naming {
                city::freeze_naming(&self.city_root, addr, naming)?;
            }
        }

        let carried = matches!(carry, wire::Carry::Handoff)
            && city::handoff(&self.city_root, addr)?.is_some();
        self.record_at(
            EventKind::SessionOpened,
            addr.clone(),
            Payload::of(&SessionOpened { carried, from })?,
        )
    }
}

impl RunWorker {
    pub(in crate::worker) fn name_session(
        &mut self,
        room: &Address,
        began: Seq,
        name: String,
    ) -> Result<(), AxError> {
        if name.chars().any(char::is_control) {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "name a session",
                name.escape_debug().to_string(),
            )
            .with_recovery("write the name on one line, without tabs or control characters"));
        }
        self.record_at(
            EventKind::SessionNamed,
            room.clone(),
            Payload::of(&SessionNamed { began, name })?,
        )
    }

    pub(in crate::worker) fn change_run_policy(
        &mut self,
        room: &Address,
        policy: kernel::model::RunPolicy,
    ) -> Result<(), AxError> {
        self.record_at(
            EventKind::RunPolicyChanged,
            room.clone(),
            Payload::of(&RunPolicyChanged {
                policy,
                by: Who::Person,
            })?,
        )?;
        self.collaborating.rooms.post_policy(room, policy);
        Ok(())
    }
}

struct SessionShape {
    endpoint: Option<String>,
    model: String,
    effort: Option<kernel::Effort>,
    generation: Option<kernel::FrozenGeneration>,
    naming: Option<kernel::B3Hash>,
}

impl RunWorker {
    fn origin_shape(&mut self, origin: kernel::Origin) -> Result<Option<SessionShape>, AxError> {
        let dir = kernel::layout::CityLayout::new(&self.city_root).ledger();
        let seqs: Vec<Seq> = self.index.run_seqs_before(origin.run, None).collect();
        let mut model = None;
        for at in seqs {
            let bytes = self
                .index
                .reader(&dir)
                .line_at(at)
                .map_err(storage::StorageError::into_ax)?;
            let line = storage::read_line(&bytes)
                .map_err(|fault| fault.into_ax(at.value().saturating_add(1)))?;
            if let storage::CheckedLine::Known(record) = line
                && record.run() == origin.run
            {
                if record.kind() == EventKind::ModelCalled && model.is_none() {
                    model = Some(record.data().read::<ModelCalled>()?.model);
                } else if record.kind() == EventKind::RunStarted {
                    let started = record.data().read::<RunStarted>()?;
                    return Ok(model.map(|model| SessionShape {
                        endpoint: started.routing.map(|routing| routing.model.endpoint),
                        model,
                        effort: started.effort,
                        generation: started.generation,
                        naming: started.naming,
                    }));
                }
            }
        }
        Ok(None)
    }

    fn origin_is_real(&mut self, origin: kernel::Origin) -> Result<(), AxError> {
        let dir = kernel::layout::CityLayout::new(&self.city_root).ledger();
        self.index
            .refresh(&dir)
            .map_err(storage::StorageError::into_ax)?;
        let owner = match self.index.reader(&dir).line_at(origin.at_seq) {
            Ok(line) => match storage::read_line(&line) {
                Ok(storage::CheckedLine::Known(record)) => Some(record.run()),
                Ok(storage::CheckedLine::IgnoredUnknown(_)) => None,
                Err(fault) => return Err(fault.into_ax(origin.at_seq.value().saturating_add(1))),
            },
            Err(storage::StorageError::SeqMissing { .. }) => None,
            Err(other) => return Err(other.into_ax()),
        };
        if owner != Some(origin.run) {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "branch a session",
                format!(
                    "seq {} is not an event of run {}",
                    origin.at_seq.value(),
                    origin.run
                ),
            )
            .with_recovery(
                "name a line of that run; a session can only continue a conversation \
                 this history holds",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
