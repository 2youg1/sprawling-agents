// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeSet;

use kernel::Seq;

use super::super::document::{Call, Callee, Decimal, End, Message, Moment, Took};
use super::{CallId, Key, Links, Seat, Visibility};

impl Links {

    pub(in crate::playback) fn moments(&self, outside: &mut BTreeSet<Seq>) -> Vec<Moment> {
        self.links
            .iter()
            .filter(|(_, link)| !link.members.is_empty())
            .filter_map(|(key, link)| match key {
                Key::Moment(family, key) => Some(Moment {
                    family: *family,
                    key: key.clone(),
                    opened: end(link.opened, End::Missing, outside),
                    closed: end(link.closed, End::Pending, outside),
                    seqs: link
                        .members
                        .iter()
                        .map(|seq| Decimal(seq.value()))
                        .collect(),
                }),
                Key::Message(_) | Key::Call(..) => None,
            })
            .collect()
    }

    pub(in crate::playback) fn messages(&self, outside: &mut BTreeSet<Seq>) -> Vec<Message> {
        self.links
            .iter()
            .filter(|(_, link)| !link.members.is_empty())
            .filter_map(|(key, link)| match key {
                Key::Message(id) => Some(Message {
                    id: id.clone(),
                    from: link.sent_by.as_ref().map(|(from, _)| from.clone()),
                    room: link.sent_by.as_ref().map(|(_, room)| room.clone()),
                    sent: end(link.opened, End::Missing, outside),
                    consumed: end(link.closed, End::Pending, outside),
                }),
                Key::Moment(..) | Key::Call(..) => None,
            })
            .collect()
    }

    pub(in crate::playback) fn calls(&self, outside: &mut BTreeSet<Seq>) -> Vec<Call> {
        let mut calls: Vec<(Seq, Call)> = self
            .links
            .iter()
            .filter_map(|(key, link)| match key {
                Key::Call(run, id) => Some((
                    *link.members.first()?,
                    Call {
                        run: *run,
                        callee: callee(
                            id,
                            link.asked.as_ref().and_then(|(name, _, _)| name.clone()),
                        ),
                        called: end(link.opened, End::Missing, outside),
                        answered: end(link.closed, End::Pending, outside),
                        took: link
                            .took
                            .map_or(Took::Unknown, |took| Took::Measured(Decimal(took))),
                    },
                )),
                Key::Moment(..) | Key::Message(_) => None,
            })
            .collect();
        calls.sort_by_key(|(first, _)| *first);
        calls.into_iter().map(|(_, call)| call).collect()
    }
}

fn callee(id: &CallId, name: Option<String>) -> Callee {
    match id {
        CallId::Tool(id) => Callee::Tool {
            id: id.clone(),
            name,
        },
        CallId::Model(_) => Callee::Model { name },
    }
}

fn end(seat: Option<Seat>, absent: End, outside: &mut BTreeSet<Seq>) -> End {
    match seat {
        None => absent,
        Some(seat) if seat.seen.visibility != Visibility::Visible => End::Withheld,
        Some(seat) if seat.seen.in_range => End::At(Decimal(seat.seq.value())),
        Some(seat) => {
            outside.insert(seat.seq);
            End::Outside(Decimal(seat.seq.value()))
        }
    }
}
