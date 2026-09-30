// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The two ends of every key moment and every message
//! (accounting-SPEC.md 8-12).
//!
//! A key moment is a set of lines grouped under a stable key: a run, an
//! approval, a pull request. It closes only on its real closing line,
//! never at the edge of the selected window, so a moment closed at one
//! cutoff stays closed at every later one
//! (`crates/accounting/spec/Playback/Project.lean`). A closing line also
//! inherits the buildings its opening line touched, which is how an
//! answer written on the city run stays behind the confidential
//! building that asked the question.

use std::collections::{BTreeMap, BTreeSet};

use kernel::event::record::{ApprovalResolved, SignalConsumed, SignalEnqueued};
use kernel::{Address, ApprovalItem, AxError, EventKind, EventRecord, RunId, Seq};

use super::document::{Decimal, End, Family, Message, Moment, Related};

/// What one line is keyed to.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Key {
    Moment(Family, String),
    Message(String),
}

/// Which part of a keyed pair one line is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Role {
    Opens,
    Closes,
    Member,
}

/// One key a line touches, and its role there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Touch {
    pub(super) key: Key,
    pub(super) role: Role,
}

/// Whether the reader saw a line, and whether the selection holds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Seen {
    pub(super) visible: bool,
    pub(super) in_range: bool,
}

#[derive(Debug, Clone, Copy)]
struct Seat {
    seq: Seq,
    seen: Seen,
}

#[derive(Debug, Default)]
struct Link {
    opened: Option<Seat>,
    closed: Option<Seat>,
    /// The buildings the opening line touched; a closing line inherits them.
    buildings: BTreeSet<Address>,
    members: Vec<Seq>,
    /// `from` and `room` of a message, when its sending line was visible.
    sent_by: Option<(String, Address)>,
}

#[derive(Debug, Default)]
pub(super) struct Links {
    links: BTreeMap<Key, Link>,
    /// The key of the latest request opened on each branch.
    open_requests: BTreeMap<String, String>,
}

impl Links {
    /// The keys `record` touches. Reads the payload of the kinds that
    /// open or close a pair.
    ///
    /// # Errors
    /// A payload of one of those kinds that does not read as its type.
    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "eight kinds open or close a pair; every other kind is at most a member of its run"
    )]
    pub(super) fn touches(&self, record: &EventRecord) -> Result<Vec<Touch>, AxError> {
        let data = record.data();
        let paired = match record.kind() {
            EventKind::ApprovalRequested => Some((
                Key::Moment(
                    Family::Approval,
                    data.read::<ApprovalItem>()?.id.as_str().to_owned(),
                ),
                Role::Opens,
            )),
            EventKind::ApprovalResolved => Some((
                Key::Moment(
                    Family::Approval,
                    data.read::<ApprovalResolved>()?.id.as_str().to_owned(),
                ),
                Role::Closes,
            )),
            EventKind::PrOpened => Some((
                Key::Moment(
                    Family::Pr,
                    request_key(
                        &collab::OpenRequest::from_payload(data)?.branch,
                        record.seq(),
                    ),
                ),
                Role::Opens,
            )),
            EventKind::PrMerged => Some((
                self.closing_request(&data.read::<collab::MergedRequest>()?.branch),
                Role::Closes,
            )),
            EventKind::PrRejected => Some((
                self.closing_request(&data.read::<collab::RejectedRequest>()?.request.branch),
                Role::Closes,
            )),
            EventKind::SignalEnqueued => Some((
                Key::Message(data.read::<SignalEnqueued>()?.id.as_str().to_owned()),
                Role::Opens,
            )),
            EventKind::SignalConsumed => Some((
                Key::Message(data.read::<SignalConsumed>()?.id.as_str().to_owned()),
                Role::Closes,
            )),
            _ => None,
        };
        let run_role = match record.kind() {
            EventKind::RunStarted | EventKind::RunForked => Role::Opens,
            EventKind::RunFrozen => Role::Closes,
            _ => Role::Member,
        };
        let run = (record.run() != RunId::CITY).then(|| Touch {
            key: Key::Moment(Family::Run, record.run().to_string()),
            role: run_role,
        });
        Ok(run
            .into_iter()
            .chain(paired.map(|(key, role)| Touch { key, role }))
            .collect())
    }

    /// The buildings a line inherits from the opening lines of the pairs
    /// it closes or belongs to.
    pub(super) fn inherited(&self, touches: &[Touch]) -> BTreeSet<Address> {
        touches
            .iter()
            .filter(|touch| touch.role != Role::Opens)
            .filter_map(|touch| self.links.get(&touch.key))
            .flat_map(|link| link.buildings.iter().cloned())
            .collect()
    }

    /// Records where `record` sits in each pair it touches. `room` is the
    /// building of its envelope address, which a run inherits; `touched`
    /// is every building it touches, which any other pair inherits.
    ///
    /// # Errors
    /// A message whose sending payload does not read.
    pub(super) fn note(
        &mut self,
        record: &EventRecord,
        touches: Vec<Touch>,
        buildings: (&BTreeSet<Address>, Option<&Address>),
        seen: Seen,
    ) -> Result<(), AxError> {
        let (touched, room) = buildings;
        let seat = Seat {
            seq: record.seq(),
            seen,
        };
        for touch in touches {
            if let (Key::Moment(Family::Pr, key), Role::Opens) = (&touch.key, touch.role) {
                let branch = key
                    .rsplit_once('@')
                    .map_or(key.as_str(), |(branch, _)| branch);
                self.open_requests.insert(branch.to_owned(), key.clone());
            }
            let family_is_run = matches!(touch.key, Key::Moment(Family::Run, _));
            let is_message = matches!(touch.key, Key::Message(_));
            let link = self.links.entry(touch.key).or_default();
            match touch.role {
                Role::Opens if link.opened.is_none() => {
                    link.opened = Some(seat);
                    link.buildings = if family_is_run {
                        room.into_iter().cloned().collect()
                    } else {
                        touched.clone()
                    };
                    if is_message && seen.visible {
                        let sent = record.data().read::<SignalEnqueued>()?;
                        link.sent_by = Some((sent.from, sent.room));
                    }
                }
                Role::Closes if link.closed.is_none() => link.closed = Some(seat),
                Role::Opens | Role::Closes | Role::Member => {}
            }
            if seen.visible && seen.in_range {
                link.members.push(seat.seq);
            }
        }
        Ok(())
    }

    /// Every key moment with a member the reader sees in range, adding
    /// to `outside` the seqs of the ends the bundle's context must hold.
    pub(super) fn moments(&self, outside: &mut BTreeSet<Seq>) -> Vec<Moment> {
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
                Key::Message(_) => None,
            })
            .collect()
    }

    /// Every message with an end the reader sees in range.
    pub(super) fn messages(&self, outside: &mut BTreeSet<Seq>) -> Vec<Message> {
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
                Key::Moment(..) => None,
            })
            .collect()
    }

    /// What the bundle may say about another run one run points at.
    pub(super) fn related(&self, run: RunId) -> Related {
        match self
            .links
            .get(&Key::Moment(Family::Run, run.to_string()))
            .and_then(|link| link.opened)
        {
            Some(seat) if seat.seen.visible => Related::Run(run),
            Some(_) => Related::Withheld,
            None => Related::Missing,
        }
    }

    /// The key a closing line of `branch` closes: the latest request
    /// opened on it, or one no opening line is known for.
    fn closing_request(&self, branch: &str) -> Key {
        let key = self
            .open_requests
            .get(branch)
            .cloned()
            .unwrap_or_else(|| format!("{branch}@missing"));
        Key::Moment(Family::Pr, key)
    }
}

/// A request's stable key: its branch and the seq of the line that
/// opened it, so a branch opened twice is two requests.
fn request_key(branch: &str, opened: Seq) -> String {
    format!("{branch}@{}", opened.value())
}

/// One end as the bundle writes it; `absent` is what a missing end means
/// on this side.
fn end(seat: Option<Seat>, absent: End, outside: &mut BTreeSet<Seq>) -> End {
    match seat {
        None => absent,
        Some(seat) if !seat.seen.visible => End::Withheld,
        Some(seat) if seat.seen.in_range => End::At(Decimal(seat.seq.value())),
        Some(seat) => {
            outside.insert(seat.seq);
            End::Outside(Decimal(seat.seq.value()))
        }
    }
}
