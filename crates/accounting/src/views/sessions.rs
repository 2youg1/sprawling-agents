// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Each address's stretches - its sessions - folded from the records the
//! Ledger files under it, and the answer for one room (`crates/accounting/spec/Views/Snapshot.lean`
//! §8-19(c), `crates/wire/Spec.lean` §8-71).
//!
//! **Held in the views rather than read back from the Ledger.** Every
//! fact a stretch shows is settled as its lines go by, and the table
//! grows with the stretches rather than with the records, so a question
//! is a copy out of memory and no ledger line is read to answer it.

use std::collections::BTreeMap;

use kernel::event::record::{ModelCalled, RunStarted, SessionNamed, WorktreeOpened};
use kernel::{Address, AxError, EventKind, EventRecord};
use wire::{Carry, SESSION_PREVIEW_MAX, SESSIONS_MAX, SessionLine, SessionStart, SessionsAnswer};

/// Every address's stretches, oldest first.
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub(crate) struct RoomSessions {
    by_room: BTreeMap<Address, Vec<SessionLine>>,
}

impl RoomSessions {
    /// Folds one record: `session_opened` begins a stretch at its address;
    /// `run_started` begins one where the address has none yet - the
    /// dispatch that opened the room - and otherwise counts a run of the
    /// current one; every record filed under an address that has a
    /// stretch becomes that stretch's last line. A record with no address
    /// is not a room's.
    ///
    /// # Errors
    /// Refuses a `session_opened` whose payload cannot be read: a stretch
    /// that does not say what it carried or branched from would be shown
    /// as one that carried nothing, which is a different stretch.
    pub(crate) fn absorb(&mut self, record: &EventRecord) -> Result<(), AxError> {
        let Some(room) = record.addr() else {
            return Ok(());
        };
        let (seq, at) = (record.seq(), record.t());
        let kind = record.kind();
        if kind == EventKind::SessionOpened {
            let opened = record
                .data()
                .read::<kernel::event::record::SessionOpened>()?;
            let carry = if opened.carried {
                Carry::Handoff
            } else {
                Carry::Nothing
            };
            self.by_room
                .entry(room.clone())
                .or_default()
                .push(SessionLine {
                    began: seq,
                    start: SessionStart::Opened {
                        carry,
                        from: opened.from,
                    },
                    runs: 0,
                    last: seq,
                    at,
                    name: None,
                    model: None,
                    effort: None,
                    workspace: None,
                    preview: None,
                });
            return Ok(());
        }
        if kind == EventKind::SessionNamed {
            let named = record.data().read::<SessionNamed>()?;
            if let Some(stretch) = self
                .by_room
                .get_mut(room)
                .and_then(|held| held.iter_mut().find(|line| line.began == named.began))
            {
                stretch.name = (!named.name.is_empty()).then_some(named.name);
            }
        }
        let runs = u64::from(kind == EventKind::RunStarted);
        match self.by_room.get_mut(room).and_then(|held| held.last_mut()) {
            Some(current) => {
                current.runs = current.runs.saturating_add(runs);
                current.last = seq;
                current.at = at;
                describe(current, record)?;
            }
            None if runs > 0 => {
                let mut opened = SessionLine {
                    began: seq,
                    start: SessionStart::Dispatched,
                    runs,
                    last: seq,
                    at,
                    name: None,
                    model: None,
                    effort: None,
                    workspace: None,
                    preview: None,
                };
                describe(&mut opened, record)?;
                self.by_room.insert(room.clone(), vec![opened]);
            }
            None => {}
        }
        Ok(())
    }

    /// The newest `SESSIONS_MAX` stretches of `room`, newest first, and
    /// how many older ones are left out. A room with no stretch answers an
    /// empty list: whether the directory exists is `Query::Listing`'s.
    pub(crate) fn answer(&self, room: &Address) -> SessionsAnswer {
        let held = self.by_room.get(room).map_or(&[][..], Vec::as_slice);
        let sessions: Vec<SessionLine> = held.iter().rev().take(SESSIONS_MAX).cloned().collect();
        SessionsAnswer {
            room: room.clone(),
            earlier: u64::try_from(held.len().saturating_sub(sessions.len())).unwrap_or(u64::MAX),
            sessions,
        }
    }
}

/// Keeps what a stretch's line says about its last run: the effort it
/// froze, the model it called, the worktree it was lent and the start of
/// its last reply (`crates/wire/spec/Answer/Sessions.lean` D27). A line of
/// any other kind says none of these.
///
/// # Errors
/// Refuses a `run_started`, `model_called` or `worktree_opened` whose
/// payload cannot be read, for the reason `absorb` refuses an unreadable
/// `session_opened`.
fn describe(stretch: &mut SessionLine, record: &EventRecord) -> Result<(), AxError> {
    let data = record.data();
    let kind = record.kind();
    if kind == EventKind::RunStarted {
        stretch.effort = data.read::<RunStarted>()?.effort;
    } else if kind == EventKind::ModelCalled {
        stretch.model = Some(data.read::<ModelCalled>()?.model);
    } else if kind == EventKind::WorktreeOpened {
        stretch.workspace = Some(data.read::<WorktreeOpened>()?.name);
    } else if kind == EventKind::ModelReturned
        && let Some(said) = data.as_map().get("message").and_then(wire::said_in)
    {
        stretch.preview = Some(said.chars().take(SESSION_PREVIEW_MAX).collect());
    }
    Ok(())
}
