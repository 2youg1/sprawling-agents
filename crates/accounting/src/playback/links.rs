// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The two ends of every key moment, every message and every call, a tool
//! call or a model attempt (`crates/accounting/spec/Playback.lean` §8-12, `crates/accounting/spec/Playback/Traced.lean` §8-17 and §8-25).
//!
//! A key moment is a set of lines grouped under a stable key: a run, an
//! approval, a pull request. It closes only on its real closing line,
//! never at the edge of the selected window, so a moment closed at one
//! cutoff stays closed at every later one
//! (`crates/accounting/spec/Playback/Project.lean`). A closing line also
//! inherits the buildings its opening line touched, which is how an
//! answer written on the city run stays behind the confidential
//! building that asked the question, and a call's answer behind the
//! building its call named.

use std::collections::{BTreeMap, BTreeSet};

use kernel::event::record::{ApprovalResolved, SignalConsumed, SignalEnqueued};
use kernel::{Address, ApprovalItem, AxError, EventKind, EventRecord, RunId, Seq, TimeMs};

use super::document::{Family, Related};
use crate::views::rounds::{Attempts, answered_timing, timing_of};

mod ends;

/// What one line is keyed to.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Key {
    Moment(Family, String),
    Message(String),
    /// A call: its run, and what its two lines are paired by there.
    Call(RunId, CallId),
}

/// What a call's two lines are paired by within its run.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum CallId {
    /// A tool call: the id its call and its answer share.
    Tool(String),
    /// A model attempt: the seq of its `model_called`, which a reply
    /// answers by the rounds' rule.
    Model(Seq),
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
    pub(super) visibility: Visibility,
    pub(super) in_range: bool,
}

/// What the fold knows of whether the reader sees a line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Visibility {
    Visible,
    Hidden,
    /// Its buildings all read `Open`, and its credential scan waits until
    /// a table reads its fate: until then it gives what a visible line
    /// gives, and [`Links::resolve`] takes that back if the scan matches
    /// (`crates/accounting/spec/Playback/Project.lean`, D47).
    Deferred,
}

impl Visibility {
    /// Whether the line may give its pair a name, a sender or a time
    /// while its fate is open.
    fn may_show(self) -> bool {
        match self {
            Visibility::Visible | Visibility::Deferred => true,
            Visibility::Hidden => false,
        }
    }
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
    /// A call's tool or model name and the moment its call line recorded,
    /// and whether that moment was measured, when the call line was
    /// visible.
    asked: Option<(Option<String>, TimeMs, wire::Timing)>,
    /// A call's measured milliseconds, when both lines were visible and
    /// the rounds' rule says the span was measured.
    took: Option<u64>,
}

#[derive(Debug, Default)]
pub(super) struct Links {
    links: BTreeMap<Key, Link>,
    /// The key of the latest request opened on each branch.
    open_requests: BTreeMap<String, String>,
    /// The latest model attempt of each run, which its next reply answers.
    attempts: Attempts,
}

impl Links {
    /// The keys `record` touches. Reads the payload of the kinds that
    /// open or close a pair.
    ///
    /// # Errors
    /// A payload of one of those kinds that does not read as its type.
    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "ten kinds open or close a pair; every other kind is at most a member of its run"
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
            EventKind::ToolCalled => wire::text(data.as_map().get("id"))
                .map(|id| (Key::Call(record.run(), CallId::Tool(id)), Role::Opens)),
            EventKind::ToolResult => wire::text(data.as_map().get("tool_use_id"))
                .map(|id| (Key::Call(record.run(), CallId::Tool(id)), Role::Closes)),
            EventKind::ModelCalled => Some((
                Key::Call(record.run(), CallId::Model(record.seq())),
                Role::Opens,
            )),
            EventKind::ModelReturned => self
                .attempts
                .answered_by(record)
                .map(|sent| (Key::Call(record.run(), CallId::Model(sent)), Role::Closes)),
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
        self.attempts.note(record);
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
            let named = match &touch.key {
                Key::Call(_, CallId::Tool(_)) => Some("name"),
                Key::Call(_, CallId::Model(_)) => Some("model"),
                Key::Moment(..) | Key::Message(_) => None,
            };
            let is_call = named.is_some();
            let link = self.links.entry(touch.key).or_default();
            match touch.role {
                Role::Opens if link.opened.is_none() => {
                    link.opened = Some(seat);
                    link.buildings = if family_is_run {
                        room.into_iter().cloned().collect()
                    } else {
                        touched.clone()
                    };
                    if is_message && seen.visibility.may_show() {
                        let sent = record.data().read::<SignalEnqueued>()?;
                        link.sent_by = Some((sent.from, sent.room));
                    }
                    if let (Some(field), true) = (named, seen.visibility.may_show()) {
                        let name = wire::text(record.data().as_map().get(field));
                        link.asked = Some((name, record.t(), timing_of(record)));
                    }
                }
                Role::Closes if link.closed.is_none() => {
                    link.closed = Some(seat);
                    if let (true, Some((_, at, asked))) =
                        (is_call && seen.visibility.may_show(), &link.asked)
                    {
                        link.took = match answered_timing(*asked, record) {
                            wire::Timing::Measured => record.t().value().checked_sub(at.value()),
                            wire::Timing::Unmeasured => None,
                        };
                    }
                }
                Role::Opens | Role::Closes | Role::Member => {}
            }
            if seen.visibility == Visibility::Visible && seen.in_range {
                link.members.push(seat.seq);
            }
        }
        Ok(())
    }

    /// Settles every deferred end of a pair with a line the reader sees
    /// in range, before any table reads it: `credential` answers whether
    /// the line at a seq carries one. An end that does turns hidden and
    /// takes back the name, the sender and the time it gave its pair.
    ///
    /// # Errors
    /// The first error `credential` returns.
    pub(super) fn resolve(
        &mut self,
        mut credential: impl FnMut(Seq) -> Result<bool, AxError>,
    ) -> Result<(), AxError> {
        for link in self
            .links
            .values_mut()
            .filter(|link| !link.members.is_empty())
        {
            if settle(link.opened.as_mut(), &mut credential)? == Visibility::Hidden {
                link.sent_by = None;
                link.asked = None;
                link.took = None;
            }
            if settle(link.closed.as_mut(), &mut credential)? == Visibility::Hidden {
                link.took = None;
            }
        }
        Ok(())
    }

    /// When the call `key` has closed with no line the reader sees in
    /// range, where it opened, if it did: neither of its lines can become
    /// context any more, so the fold keeps neither.
    pub(super) fn settled_unselected(&self, key: &Key) -> Option<Option<Seq>> {
        self.links
            .get(key)
            .filter(|link| link.closed.is_some() && link.members.is_empty())
            .map(|link| link.opened.map(|seat| seat.seq))
    }

    /// What the bundle may say about another run one run points at.
    pub(super) fn related(&self, run: RunId) -> Related {
        match self
            .links
            .get(&Key::Moment(Family::Run, run.to_string()))
            .and_then(|link| link.opened)
        {
            Some(seat) => match seat.seen.visibility {
                Visibility::Visible => Related::Run(run),
                Visibility::Hidden | Visibility::Deferred => Related::Withheld,
            },
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

/// Scans the end at `seat` if its fate was deferred, and answers what
/// the reader now knows of it; a missing end is visible to nobody.
fn settle(
    seat: Option<&mut Seat>,
    credential: &mut impl FnMut(Seq) -> Result<bool, AxError>,
) -> Result<Visibility, AxError> {
    let Some(seat) = seat else {
        return Ok(Visibility::Hidden);
    };
    if seat.seen.visibility == Visibility::Deferred {
        seat.seen.visibility = if credential(seat.seq)? {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
    }
    Ok(seat.seen.visibility)
}

/// A request's stable key: its branch and the seq of the line that
/// opened it, so a branch opened twice is two requests.
fn request_key(branch: &str, opened: Seq) -> String {
    format!("{branch}@{}", opened.value())
}
