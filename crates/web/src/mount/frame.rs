// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The frame: what one painted frame may move.

#[cfg(target_arch = "wasm32")]
use crate::app::Snapshot;
#[cfg(target_arch = "wasm32")]
use crate::asking::{hold, invalidated_by, started_here};
#[cfg(target_arch = "wasm32")]
use crate::route::View;
#[cfg(target_arch = "wasm32")]
use dioxus::prelude::*;

/// The signals one painted frame may move.
///
/// A struct rather than fifteen parameters, and by value because every
/// field is a `Copy` handle: this is the same reasoning that gave `Wiring`
/// its shape, and splitting it would produce two halves neither of which
/// can paint a frame.
#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy)]
pub(crate) struct FrameWiring {
    pub(crate) snapshot: Signal<Snapshot>,
    pub(crate) endpoints: Signal<Option<channels::EndpointsAnswer>>,
    pub(crate) city: Signal<Option<channels::CityAnswer>>,
    pub(crate) cost: Signal<Option<channels::CostAnswer>>,
    pub(crate) building: Signal<Option<channels::BuildingAnswer>>,
    pub(crate) discards: Signal<Option<channels::DiscardAnswer>>,
    pub(crate) inbox: Signal<Option<channels::InboxAnswer>>,
    pub(crate) hits: Signal<Option<channels::ArchiveAnswer>>,
    pub(crate) filed: Signal<Option<channels::RegistryAnswer>>,
    pub(crate) vitals: Signal<Option<channels::MetricsAnswer>>,
    pub(crate) changes: Signal<Option<channels::ChangesAnswer>>,
    pub(crate) records: Signal<Vec<channels::EventRecord>>,
    pub(crate) view: Signal<View>,
    pub(crate) expecting: Signal<Option<String>>,
    pub(crate) refused: Signal<Option<crate::alert::Refused>>,
    pub(crate) lang: Signal<crate::lang::Lang>,
}

/// Applies one animation frame's worth of arrivals.
///
/// The order is the one `Paint::into_parts` hands them out in and the
/// reason is recorded there: an answer describes the city as of some
/// moment, so folding the frame's events first is what stops a page
/// rendering a view its own snapshot has not caught up with.
///
/// Every event is still read one at a time - `alert::absorb` deduplicates
/// an interruption, `invalidated_by` asks again, `started_here` recognises
/// the room this client asked for. What the frame changed is *when the
/// signals move*, and they now move once for the whole burst.
#[cfg(target_arch = "wasm32")]
pub(crate) fn apply_frame(
    paint: crate::pace::Paint,
    socket: &std::rc::Rc<std::cell::RefCell<Option<web_sys::WebSocket>>>,
    alerts: &std::rc::Rc<std::cell::RefCell<crate::alert::Alerts>>,
    wiring: FrameWiring,
) {
    let FrameWiring {
        mut snapshot,
        mut endpoints,
        mut city,
        mut cost,
        mut building,
        mut discards,
        mut inbox,
        mut hits,
        mut filed,
        mut vitals,
        mut changes,
        mut records,
        mut view,
        mut expecting,
        mut refused,
        lang,
    } = wiring;
    let (events, answers, refusal, saying) = paint.into_parts();
    let said = lang();
    // Increments first, and into the snapshot's own discardable buffer.
    // Before the events on purpose: `model_returned` in this same burst
    // throws the buffer away, so a call that both streamed and settled
    // inside one frame ends with the record showing rather than the
    // increments that preceded it.
    if !saying.is_empty() {
        snapshot.with_mut(|held| {
            for delta in &saying {
                held.is_saying(delta);
            }
        });
    }
    // Everything the burst adds to history, folded and kept in one write
    // each rather than in one write each per event.
    let mut keep: Vec<channels::EventRecord> = Vec::new();
    for event in events {
        // Decided in the same pass as the snapshot: what happened and
        // whether it needs a person are two readings of one event, not two
        // readers of the stream.
        if let Ok(mut alerts) = alerts.try_borrow_mut()
            && crate::alert::absorb(said, &mut alerts, &event) == crate::alert::Raise::Interrupt
            && let Some(alert) = crate::alert::alert_for(said, &event)
        {
            crate::alert::interrupt(said, &alert);
        }
        if let Some(query) = invalidated_by(event.kind()) {
            let held = socket.borrow();
            if let Some(socket) = held.as_ref() {
                let _ = crate::socket::send(socket, &channels::ClientFrame::Query(query));
            }
        }
        // The session this person asked for, opening. Knowledge rather
        // than a guess: this client sent that dispatch and knows the room
        // it named. Read and released before the write below: a signal
        // held open across its own set is a panic in a browser and nothing
        // at all in a host test.
        let waiting = expecting.read().clone();
        if let Some(waiting) = waiting
            && started_here(&event, &waiting).is_some()
            && let Some(addr) = event.addr().cloned()
        {
            // The room, not the run: a session opened by this person is
            // named by the name they gave it, and that is the address
            // this build puts in the bar.
            expecting.set(None);
            view.set(View::Session(addr.clone()));
            crate::route::go(&View::Session(addr));
        }
        if snapshot.write().apply(&event) {
            keep.push(event);
        }
    }
    // Which session the person has open, so a store at its bound gives
    // way in what they are not reading rather than in what they are.
    let reading = match &*view.read() {
        View::Session(addr) => snapshot.read().session_at(addr).map(|(run, _)| run),
        View::Run(run) => Some(*run),
        _ => None,
    };
    if !keep.is_empty() {
        // Kept once, read by every page that reads history, and bounded by
        // the one function that answers "how much does a tab hold".
        hold(&mut records.write(), keep, reading);
    }
    // An answer reaches the view that asked for it. It is not history: it
    // moves no snapshot, and a reload asks again rather than trusting what
    // is held.
    for answer in answers {
        match answer {
            channels::Answer::Endpoints(held) => endpoints.set(Some(held)),
            channels::Answer::City(held) => city.set(Some(held)),
            channels::Answer::Cost(held) => cost.set(Some(*held)),
            channels::Answer::Building(held) => building.set(Some(*held)),
            channels::Answer::Discards(held) => discards.set(Some(held)),
            channels::Answer::Inbox(held) => inbox.set(Some(held)),
            channels::Answer::Archive(held) => hits.set(Some(held)),
            channels::Answer::Registry(held) => filed.set(Some(held)),
            channels::Answer::Metrics(held) => vitals.set(Some(*held)),
            channels::Answer::Changes(held) => changes.set(Some(held)),
            // What happened before this tab opened. Folded into the
            // snapshot and kept for the pages that read history, in the
            // same bounded store the live stream fills - one answer to
            // "how much does a tab hold".
            // What happened before this tab opened, from either of the
            // two questions that ask it: the city's own slice at connect,
            // and one session's when its page opens. The backfill is
            // forward-only and refuses the second, which is correct - a
            // snapshot already folded past these must not be walked
            // back - but the records themselves are still what the
            // session page reads, so they are kept either way.
            channels::Answer::History(held) => {
                snapshot.write().backfill(&held.records);
                hold(&mut records.write(), held.records, reading);
            }
            // What was already waiting when this page connected. The
            // stream carries what happens next; without this the inbox
            // would show only the items raised since the tab opened.
            channels::Answer::Approvals(held) => {
                snapshot.write().adopt_approvals(held.items);
            }
            // Named one by one rather than caught by a wildcard: each of
            // these has an answer the server can give and no page that
            // asks for it yet, and a wildcard here would hide the next one
            // that arrives as well.
            channels::Answer::Run(_)
            | channels::Answer::Commit(_)
            | channels::Answer::Unavailable { .. } => {}
        }
    }
    // A refusal is not history and must not move the snapshot - but it is
    // the answer to something a person just did, so it goes where they can
    // read it.
    if let Some(error) = refusal {
        refused.set(Some(crate::alert::refused(said, &error)));
    }
}
