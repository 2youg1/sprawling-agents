// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::AxError;

use super::super::RunWorker;
use crate::worker::Posted;

pub(in crate::worker) struct Finishing {
    name: &'static str,
    key: Option<kernel::IdemKey>,
    reply: wire::Reply,
}

pub(in crate::worker) struct InHand {
    pub(in crate::worker) name: &'static str,
    pub(in crate::worker) reply: wire::Reply,
}

pub(in crate::worker) struct KeptRefusal(std::sync::mpsc::Receiver<AxError>);

impl KeptRefusal {
    pub(in crate::worker) fn open() -> (wire::Reply, KeptRefusal) {
        let (keep, kept) = std::sync::mpsc::channel();
        let reply = wire::Reply::to(move |refusal| match keep.send(refusal) {
            Ok(()) => wire::Delivered::ToThePeer,
            Err(_) => wire::Delivered::PeerGone,
        });
        (reply, KeptRefusal(kept))
    }

    pub(in crate::worker) fn outcome(self) -> Result<(), AxError> {
        match self.0.try_recv() {
            Ok(refusal) => Err(refusal),
            Err(
                std::sync::mpsc::TryRecvError::Empty | std::sync::mpsc::TryRecvError::Disconnected,
            ) => Ok(()),
        }
    }
}

impl RunWorker {
    pub fn handle(&mut self, command: wire::Command) -> Result<(), AxError> {
        self.history_admits(&command)?;
        let (reply, refused_later) = KeptRefusal::open();
        let carried = self.carry_out(command, reply);

        let landed = self.land_the_rest();
        carried.and(landed).and(refused_later.outcome())
    }

    fn carry_out(&mut self, command: wire::Command, reply: wire::Reply) -> Result<(), AxError> {
        let name = command.name();
        let in_hand = InHand {
            name,
            reply: reply.clone(),
        };
        let outcome = self.with_in_hand(in_hand, |worker| worker.run_command(command, reply));
        if let Err(err) = &outcome {
            self.note_refused(name, err);
        }
        outcome
    }

    fn note_refused(&mut self, name: &str, err: &AxError) {
        self.note(
            runtime::diagnostics::Level::Refuse,
            "accounting::worker",
            &format!("{name} refused: {err}; {}", err.recovery()),
        );
    }

    pub(in crate::worker) fn with_in_hand<R>(
        &mut self,
        in_hand: InHand,
        work: impl FnOnce(&mut RunWorker) -> R,
    ) -> R {
        let outer = self.doorstep.in_hand.replace(in_hand);
        let done = work(self);
        self.doorstep.in_hand = outer;
        done
    }

    pub(in crate::worker) fn finishing(&self) -> Finishing {
        let (name, reply) = match &self.doorstep.in_hand {
            Some(InHand { name, reply }) => (*name, reply.clone()),
            None => ("a request the city made itself", wire::Reply::nowhere()),
        };
        Finishing {
            name,
            key: self.doorstep.entrance.carrying(),
            reply,
        }
    }

    pub(in crate::worker) fn finish(
        &mut self,
        finishing: Finishing,
        work: impl FnOnce(&mut RunWorker) -> Result<(), AxError>,
    ) {
        let Finishing { name, key, reply } = finishing;
        let outer = self.doorstep.entrance.carrying();
        self.doorstep.entrance.carry(key);
        let outcome = work(self);
        self.doorstep.entrance.carry(outer);
        if let Err(err) = outcome {
            self.note_refused(name, &err);
            if let Some(key) = key {
                self.doorstep.entrance.refused_later(key, &err);
            }
            self.hand_back(&reply, err);
        }
    }

    pub(in crate::worker) fn serve_one(&mut self, posted: Posted) {
        let Posted { command, reply } = posted;

        if let Err(err) = self.history_admits(&command) {
            self.hand_back(&reply, err);
            return;
        }
        let key = command.idem().copied();
        let speech = matches!(
            command,
            wire::Command::Dispatch { .. } | wire::Command::Steer { .. }
        );
        if let Some(first) = key.and_then(|key| self.doorstep.entrance.answered(&key)) {
            let said = super::entrance::repeated(command.name());
            self.note(
                runtime::diagnostics::Level::Effect,
                "accounting::worker",
                &said,
            );
            match first {
                Err(err) => self.hand_back(&reply, err),
                Ok(()) if speech => {
                    if let Some(key) =
                        key.filter(|key| self.doorstep.entrance.accepted_speech(*key))
                    {
                        let _delivered = reply.accept(key);
                    }
                }
                Ok(()) => {}
            }
            return;
        }
        if let Some(key) = key {
            self.doorstep.entrance.begin(key);
        }

        let reply = if speech {
            reply.for_request(key)
        } else {
            reply
        };
        let outcome = self.carry_out(command, reply.clone());
        self.doorstep.entrance.settle(&outcome);
        if let Err(err) = outcome {
            self.hand_back(&reply, err);
        }
    }

    fn history_admits(&mut self, command: &wire::Command) -> Result<(), AxError> {
        let refused = self.ledger.admits().map_err(storage::StorageError::into_ax);
        if let Err(err) = &refused {
            self.note(
                runtime::diagnostics::Level::Refuse,
                "accounting::worker",
                &format!("{} refused: {err}; {}", command.name(), err.recovery()),
            );
        }
        refused
    }

    pub(in crate::worker) fn hand_back(&mut self, reply: &wire::Reply, error: AxError) {
        if let Some(key) = reply
            .request()
            .filter(|key| !self.doorstep.entrance.accepted_speech(*key))
        {
            self.doorstep.entrance.refused_later(key, &error);
        }
        match reply.refuse(error) {
            wire::Delivered::ToThePeer | wire::Delivered::NobodyAsked => {}
            wire::Delivered::PeerGone => self.note(
                runtime::diagnostics::Level::Refuse,
                "accounting::worker",
                "the refusal above reached nobody: the peer that asked had closed its socket",
            ),
        }
    }
}
