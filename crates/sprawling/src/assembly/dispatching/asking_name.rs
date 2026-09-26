// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Work sent to a building with no room waits here, off the accounting
//! thread, while the digest model names it.
//!
//! The naming call waits on a provider for seconds; the accounting
//! thread serves every run's appends. A name asked for on that thread
//! stalled every run already going for as long as the provider took
//! (sprawling-SPEC.md 8-86). So the call is built on the accounting
//! thread, made on a thread of its own, and the name comes back through
//! the same queue that wakes the accounting thread for everything else.
//! Nothing is written for the dispatch until the name is home, so
//! opening the room is still the first thing this city puts on disk.

use std::sync::mpsc;

use kernel::{AxCode, AxError};

use super::super::{Assignment, Owing, RunWorker};
use super::session::unnamed;
use crate::serving::relay::Wake;

/// The names still being asked for, and the way they come home.
pub(in crate::assembly) struct Namings {
    back: mpsc::Sender<Named>,
    home: mpsc::Receiver<Named>,
    pending: u32,
}

/// A dispatch whose naming call came back, carried whole so it resumes
/// exactly where it paused.
struct Named {
    at: Assignment,
    task: String,
    goal: String,
    reply: channels::Reply,
    name: Result<Option<kernel::SessionName>, AxError>,
}

impl Namings {
    pub(in crate::assembly) fn open() -> Namings {
        let (back, home) = mpsc::channel();
        Namings {
            back,
            home,
            pending: 0,
        }
    }

    /// Whether a name is still out: a city that lands the rest of its
    /// work waits for these too, because each one is a run about to
    /// start.
    pub(in crate::assembly) fn pending(&self) -> bool {
        self.pending > 0
    }
}

impl RunWorker {
    /// Agrees to the work, then asks for its name on a thread of its own.
    ///
    /// Agreeing comes first, so a person does not pay a provider to name
    /// work this city was never going to take; it writes nothing, and it
    /// is asked again when the name is home, because a halt may have
    /// arrived while the provider was thinking.
    ///
    /// # Errors
    /// Propagates every refusal agreeing can owe, the refusal to route a
    /// confidential building's text off this machine, and the operating
    /// system's refusal to start a thread.
    pub(in crate::assembly) fn name_then_dispatch(
        &mut self,
        at: Assignment,
        task: String,
        goal: String,
        reply: channels::Reply,
    ) -> Result<(), AxError> {
        let agreed = self.agree_to_work(&at)?;
        let call = self.naming_call(agreed.rules.policy())?;
        let back = self.namings.back.clone();
        let bell = self.bell();
        std::thread::Builder::new()
            .name("sprawling-naming".to_owned())
            .spawn(move || {
                let name = call.name(&task);
                // Nobody listening means the city closed while the
                // provider was thinking; nothing was written for this
                // dispatch, so there is nothing to undo.
                drop(back.send(Named {
                    at,
                    task,
                    goal,
                    reply,
                    name,
                }));
                drop(bell.send(Wake::Command));
            })
            .map_err(|source| {
                AxError::failure(
                    AxCode::StorageFatal,
                    "ask the digest model for a room name",
                    source.to_string(),
                )
                .with_recovery("check process thread limits, or name the room yourself")
            })?;
        self.namings.pending = self.namings.pending.saturating_add(1);
        Ok(())
    }

    /// Takes every dispatch whose name came home into a lane, and hands
    /// each refusal back to whoever asked.
    pub(in crate::assembly) fn dispatch_the_named(&mut self) {
        while let Ok(named) = self.namings.home.try_recv() {
            self.namings.pending = self.namings.pending.saturating_sub(1);
            let Named {
                mut at,
                task,
                goal,
                reply,
                name,
            } = named;
            let taken = name
                .and_then(|name| name.ok_or_else(|| unnamed(&at.addr)))
                .and_then(|name| {
                    at.session = Some(name);
                    self.dispatch_into_lane(at, task, goal, Owing::asked(reply.clone()))
                });
            if let Err(err) = taken {
                self.note(
                    runtime::diagnostics::Level::Refuse,
                    "bin::assembly",
                    &format!("dispatch refused: {err}; {}", err.recovery()),
                );
                self.hand_back(&reply, err);
            }
        }
    }
}
