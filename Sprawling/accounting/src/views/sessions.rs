// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;

use kernel::event::record::{ModelCalled, RunStarted, SessionNamed, WorktreeOpened};
use kernel::{Address, AxError, EventKind, EventRecord};
use wire::{
    Carry, SESSION_PREVIEW_MAX, SESSIONS_MAX, SessionLine, SessionRef, SessionStart, SessionsAnswer,
};

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub(crate) struct RoomSessions {
    by_room: BTreeMap<Address, Vec<SessionLine>>,
    by_run: BTreeMap<kernel::RunId, SessionRef>,
}

impl RoomSessions {

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
            let parent = opened
                .from
                .as_ref()
                .and_then(|origin| self.by_run.get(&origin.run).cloned());
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
                    parent,
                    branched_from: opened.from,
                    tail: None,
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
        let started = if kind == EventKind::RunStarted {
            Some(record.data().read::<RunStarted>()?)
        } else {
            None
        };
        let parent = started
            .as_ref()
            .and_then(|line| line.parent)
            .and_then(|run| self.by_run.get(&run).cloned());
        let runs = u64::from(started.is_some());
        let mut opened_run = None;
        match self.by_room.get_mut(room).and_then(|held| held.last_mut()) {
            Some(current) => {
                let first = current.runs == 0;
                current.runs = current.runs.saturating_add(runs);
                if first
                    && matches!(&current.start, SessionStart::Dispatched { .. })
                    && current.parent.is_none()
                {
                    current.parent = parent;
                }
                if started.is_some()
                    || current
                        .tail
                        .as_ref()
                        .is_some_and(|tail| tail.run == record.run())
                {
                    current.tail = Some(kernel::Origin {
                        run: record.run(),
                        at_seq: seq,
                    });
                }
                if started.is_some() {
                    opened_run = Some(SessionRef {
                        room: room.clone(),
                        began: current.began,
                    });
                }
                current.last = seq;
                current.at = at;
                describe(current, record)?;
            }
            None if runs > 0 => {
                let mut opened = SessionLine {
                    began: seq,
                    start: SessionStart::Dispatched {
                        by: record.data().read::<RunStarted>()?.dispatched_by,
                    },
                    runs,
                    last: seq,
                    at,
                    name: None,
                    model: None,
                    effort: None,
                    workspace: None,
                    preview: None,
                    parent,
                    branched_from: None,
                    tail: Some(kernel::Origin {
                        run: record.run(),
                        at_seq: seq,
                    }),
                };
                describe(&mut opened, record)?;
                opened_run = Some(SessionRef {
                    room: room.clone(),
                    began: opened.began,
                });
                self.by_room.insert(room.clone(), vec![opened]);
            }
            None => {}
        }
        if let Some(opened) = opened_run {
            self.by_run.insert(record.run(), opened);
        }
        Ok(())
    }

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
