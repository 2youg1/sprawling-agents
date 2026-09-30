// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Starting a new session at an address: the verb that gives a room a
//! way out of the shape its first run froze.
//!
//! **Why the city owes this.** A room's first run writes the model it
//! calls and how hard it thinks into the room's own `CONFIG.toml`, and
//! every later run there refuses to move either, because a provider
//! caches a conversation's prefix only while the shape of the calls
//! behind it holds still. The refusal is right and was a dead end: a
//! person who changed the model could no longer dispatch into that room
//! at all. What this verb adds is the thing that was missing - a session
//! is a *stretch* of a room, and starting a new one is a thing a person
//! asks for rather than a change made behind their back
//! (`sprawling-SPEC.md` 8-82).
//!
//! **The city decides nothing about what a session keeps.** That ruling
//! is `carry`, and the two cities-side acts it names live in
//! `city::session`: forget the frozen shape always, empty the handoff
//! slot unless the person asked to carry it. This module's job is the
//! two facts the city owns - whether a run is working there, and what
//! the ledger records.
//!
//! **One refusal.** A run is driving in that room right now. Replacing
//! the session under a run that is writing this session's history would
//! let two stretches claim one prefix, so the caller is told to stop it
//! first: `/stop`, then `/new`.
//!
//! **`carried` is read, not assumed.** The frame says what the person
//! asked for; whether there was a handoff to carry is a fact about the
//! room, and a record of what was *asked* would answer "did this session
//! inherit anything" with a wish. A `--carry` with nothing to carry is
//! not a refusal either - the person asked for a new session, which is a
//! thing that can be done (`sprawling-SPEC.md` 8-82).

use kernel::event::record::SessionOpened;
use kernel::{Address, AxCode, AxError, EventKind, Payload};

use super::super::RunWorker;

impl RunWorker {
    /// Begins a new session at `addr`, keeping what `carry` names.
    ///
    /// # Errors
    /// Refuses `E_BUSY` while a run is driving in that room - the run is
    /// named so the caller can stop it - and propagates the room's own
    /// configuration failing to read or write. A ledger that refuses the
    /// record propagates too, and by then the room is already cleared:
    /// the two halves are ordered so the durable record is the last
    /// thing, and a failure here leaves a room whose next dispatch
    /// simply chooses a shape again rather than a session the client was
    /// told about and the history never saw.
    pub(in crate::assembly) fn open_session(
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
        // The branch is checked before anything is cleared: a session
        // that cannot inherit must not have cost the room its shape.
        if let Some(origin) = from {
            self.origin_is_real(origin)?;
        }
        match carry {
            wire::Carry::Nothing => city::clear_session(&self.city_root, addr)?,
            // The frozen shape always goes: that is what made the room
            // dispatchable again, and a person who carries the summary
            // changed the model. Only the handoff slot is kept.
            wire::Carry::Handoff => city::forget_shape(&self.city_root, addr)?,
        }
        // Read after the act rather than before it: `Nothing` has just
        // removed the file, and asking first would answer about a slot
        // this call is in the middle of emptying.
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
    /// Refuses a branch that names a line this history does not have, or
    /// a line of another run.
    ///
    /// The check happens here rather than at the first run, because the
    /// person is standing there when they ask for it: a branch that
    /// cannot be rebuilt is a refusal now, in words about the line they
    /// named, rather than a run that starts and finds nothing.
    fn origin_is_real(&mut self, origin: kernel::Origin) -> Result<(), AxError> {
        let dir = kernel::layout::CityLayout::new(&self.city_root).ledger();
        self.index
            .refresh(&dir)
            .map_err(storage::StorageError::into_ax)?;
        let owner = match self.index.reader(&dir).line_at(origin.at_seq) {
            // A line of a newer kind is not a line of that run's
            // conversation, which is the refusal below; a line that is no
            // record at all is a damaged ledger, and says so.
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
