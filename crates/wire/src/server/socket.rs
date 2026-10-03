// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One WebSocket session, from the upgrade to its end: the shell around
//! [`decide_frame`]. Every branch is a send, a receive, or the end of
//! the session; the judgements are `wire::reception`'s.
//!
//! A refusal made minutes later has no way home, which is why a command
//! carries the [`Reply`] address of whoever sent it.
//!
//! The event arm writes in frames (`crates/wire/spec/Server/Socket.lean`
//! §8-47h): every record already queued when it wakes becomes its own
//! `Event` frame, and the socket is flushed once. The model there proves
//! that where the frames are cut changes nothing the page is told, that
//! records are told in seq order and none twice, and that every seq not
//! told as a record is inside a told `Lagged` range.

use std::sync::Arc;

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use futures_util::SinkExt as _;
use kernel::{AxCode, AxError, Seq};
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::TryRecvError;

use super::committed::Committed;
use crate::frames::{Answered, Ask, AskOutcome, Lagged, Sample, ServerFrame};
use crate::reception::inbound::Inbound;
use crate::reception::{SessionState, SessionStep, Stream, WelcomeFacts, decide_frame};
use crate::reply::{Delivered, Reply};

use super::config::{Answering, ShellState};

pub(crate) async fn upgrade(
    State(state): State<Arc<ShellState>>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(move |socket| session(socket, state))
}

/// The shell around [`decide_frame`]: it moves bytes and holds no policy.
/// Every judgement here is the pure function's; every branch below is
/// either a send, a receive, or the end of the session.
async fn session(mut socket: WebSocket, state: Arc<ShellState>) {
    let mut phase = SessionState::AwaitingHello;
    let mut inbound = Inbound::new();
    let mut events = state.events.subscribe();
    let mut deltas = state.deltas.subscribe();
    let mut logs = state.logs.subscribe();
    let mut outputs = state.outputs.subscribe();
    // This session's own refusals, which the worker posts into long
    // after the command was accepted. Unbounded because a refusal must
    // not be dropped and because its rate is the rate at which one
    // person makes mistakes, not the rate of the event stream.
    let (refused, mut refusals) = tokio::sync::mpsc::unbounded_channel::<AxError>();
    // What this session has delivered and what it still owes.
    let mut stream = Stream::opening();
    let mut watching: Option<Watching> = None;
    loop {
        tokio::select! {
            incoming = socket.recv() => {
                let Some(Ok(message)) = incoming else { return };
                let Message::Text(text) = message else { continue };
                // A frame this build cannot read and a frame it can are
                // judged through the same branch below, so a refusal
                // reaches the peer by one path whichever produced it.
                let step = match inbound.read(&text) {
                    Ok(frame) => {
                        decide_frame(phase, frame, &state.face, WelcomeFacts { city: state.city.as_ref(), head: state.head.read(), epoch: state.epoch })
                    }
                    Err(unreadable) => unreadable,
                };
                match step {
                    SessionStep::Welcome(welcome) => {
                        phase = SessionState::Live;
                        if send(&mut socket, &ServerFrame::Welcome(*welcome)).await.is_err() {
                            return;
                        }
                        for written in (state.outputs_so_far)() {
                            if send(&mut socket, &ServerFrame::Output(written)).await.is_err() {
                                return;
                            }
                        }
                    }
                    SessionStep::Deliver(command) => {
                        let back = refused.clone();
                        let reply = Reply::to(move |error| match back.send(error) {
                            Ok(()) => Delivered::ToThePeer,
                            Err(_) => Delivered::PeerGone,
                        });
                        if let Err(error) = (state.commands)(*command, reply)
                            && send(&mut socket, &ServerFrame::Refusal(Box::new(error))).await.is_err() {
                            return;
                        }
                    }
                    SessionStep::Answer(ask) => {
                        let frame = answered(Arc::clone(&state.queries), *ask).await;
                        if send(&mut socket, &frame).await.is_err() {
                            return;
                        }
                    }
                    SessionStep::Watch(watched) => {
                        let samples = watching
                            .take()
                            .map_or_else(|| state.monitor.samples.subscribe(), |(_, samples)| samples);
                        watching = Some(((state.monitor.watch)(watched), samples));
                    }
                    SessionStep::Release => watching = None,
                    SessionStep::Refuse { error, close } => {
                        if send(&mut socket, &ServerFrame::Refusal(error)).await.is_err() {
                            return;
                        }
                        if close {
                            return;
                        }
                    }
                }
            }
            // A refusal the worker made after this socket had already
            // answered. It reaches the peer that caused it and nobody
            // else, which is why it travels here and not as an event.
            late = refusals.recv() => {
                let Some(error) = late else { return };
                if send(&mut socket, &ServerFrame::Refusal(Box::new(error))).await.is_err() {
                    return;
                }
            }
            event = events.recv() => {
                // A slow client does not hold the writer back: the
                // subscription leaves it behind, `framed` names the range
                // it lost at the record that ends it, and the Ledger
                // answers that range. The sender gone means the city is
                // going down; a receiver left on a closed channel answers
                // at once for ever, so the session ends.
                let first = match event {
                    Ok(committed) => Arrival::Record(committed.record().seq(), committed),
                    Err(broadcast::error::RecvError::Lagged(_)) => Arrival::Skipped,
                    Err(broadcast::error::RecvError::Closed) => return,
                };
                let (arrivals, ending) = drained(first, &mut events);
                let (said, next) = framed(stream, phase, arrivals);
                stream = next;
                if write_frame(&mut socket, said).await.is_err() {
                    return;
                }
                match ending {
                    Ending::Open => {}
                    Ending::Closed => return,
                }
            }
            // Increments, on their own channel. **A skipped one is not
            // stated**: an increment is written down nowhere, so a range
            // naming it would name records that do not exist, and the
            // settled text arrives as a record either way.
            said = deltas.recv() => {
                match said {
                    Ok(delta) => {
                        if phase == SessionState::Live
                            && send(&mut socket, &ServerFrame::Delta(delta)).await.is_err() {
                            return;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Err(broadcast::error::RecvError::Closed) => return,
                }
            }
            // The process log. A skipped line is not stated for the reason
            // an increment's is not: a log is a diagnostic and not history,
            // and the position it carries is the ledger's rather than a
            // sequence of its own - so no range of it could put a reader back
            // where it was.
            written = logs.recv() => {
                match written {
                    Ok(line) => {
                        if phase == SessionState::Live
                            && send(&mut socket, &ServerFrame::Log(line)).await.is_err() {
                            return;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Err(broadcast::error::RecvError::Closed) => return,
                }
            }
            // Running commands' output. A skipped piece is not stated: the
            // whole output arrives with the call's result either way.
            written = outputs.recv() => {
                match written {
                    Ok(piece) => {
                        if phase == SessionState::Live
                            && send(&mut socket, &ServerFrame::Output(piece)).await.is_err() {
                            return;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Err(broadcast::error::RecvError::Closed) => return,
                }
            }
            read = next_sample(&mut watching) => {
                match read {
                    Ok(sample) => {
                        if send(&mut socket, &ServerFrame::Monitor(sample)).await.is_err() {
                            return;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Err(broadcast::error::RecvError::Closed) => watching = None,
                }
            }
        }
    }
}

/// What one session took from its subscription: a record and its seq,
/// or a skip the subscription reported.
pub(crate) enum Arrival<R> {
    Record(Seq, R),
    Skipped,
}

/// What one session tells the page: a range to ask for, or a record.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Said<R> {
    Lagged(Lagged),
    Record(R),
}

/// The most arrivals one frame takes: a frame this long is written out
/// first, so a long burst does not starve the session's other arms.
pub(crate) const FRAME_RECORDS_MAX: usize = 256;

/// Whether the subscription is still open after a frame was drained.
enum Ending {
    Open,
    Closed,
}

/// `first` and every arrival already queued behind it, at most
/// [`FRAME_RECORDS_MAX`], and whether the subscription closed meanwhile.
fn drained(
    first: Arrival<Committed>,
    events: &mut broadcast::Receiver<Committed>,
) -> (Vec<Arrival<Committed>>, Ending) {
    let mut arrivals = vec![first];
    while arrivals.len() < FRAME_RECORDS_MAX {
        match events.try_recv() {
            Ok(committed) => arrivals.push(Arrival::Record(committed.record().seq(), committed)),
            Err(TryRecvError::Lagged(_)) => arrivals.push(Arrival::Skipped),
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Closed) => return (arrivals, Ending::Closed),
        }
    }
    (arrivals, Ending::Open)
}

/// What a session in `phase` tells the page for one frame of arrivals,
/// and the stream state after it: each record in turn through
/// [`Stream::before`], each skip through [`Stream::skipped`]. Pure.
pub(crate) fn framed<R>(
    stream: Stream,
    phase: SessionState,
    arrivals: Vec<Arrival<R>>,
) -> (Vec<Said<R>>, Stream) {
    let live = phase == SessionState::Live;
    let mut said = Vec::with_capacity(arrivals.len());
    let after = arrivals
        .into_iter()
        .fold(stream, |stream, arrival| match arrival {
            Arrival::Record(seq, record) if live => {
                let (lag, next) = stream.before(seq);
                said.extend(lag.map(Said::Lagged));
                said.push(Said::Record(record));
                next
            }
            // A session that has not been welcomed is shown no stream.
            Arrival::Record(..) => stream,
            Arrival::Skipped => stream.skipped(live),
        });
    (said, after)
}

/// Feeds each thing said to the socket as its own frame, then flushes
/// once.
async fn write_frame(socket: &mut WebSocket, said: Vec<Said<Committed>>) -> Result<(), ()> {
    for each in said {
        let text = match each {
            Said::Lagged(lag) => serde_json::to_string(&ServerFrame::Lagged(lag))
                .map_err(|_| ())?
                .into(),
            Said::Record(committed) => committed.frame(),
        };
        socket.feed(Message::Text(text)).await.map_err(|_| ())?;
    }
    socket.flush().await.map_err(|_| ())
}

/// The frame that answers one question, read on the blocking pool.
///
/// Answering reads the disk and takes a lock. Run inside the task that
/// owns the socket, one `RunHistory` would hold a tokio worker for the
/// whole read: the peer would receive no pushed event for as long as it
/// lasted, and enough people opening a session at once could occupy
/// every worker the runtime has. The blocking pool is where a
/// synchronous read belongs, and this stays true however fast the read
/// becomes.
async fn answered(answering: Answering, Ask { ask_id, query }: Ask) -> ServerFrame {
    let (as_of, outcome) = match tokio::task::spawn_blocking(move || answering(query)).await {
        Ok((as_of, Ok(answer))) => (as_of, AskOutcome::Answer(answer)),
        Ok((as_of, Err(error))) => (as_of, AskOutcome::Refusal(error)),
        // The pool dropped the work, which means the runtime is going
        // down; say so rather than leave the page waiting on a frame
        // that will never come. Nothing was read, so the answer is
        // dated at the start of history.
        Err(_) => (
            Seq::FIRST,
            AskOutcome::Refusal(
                AxError::failure(
                    AxCode::StorageFatal,
                    "answer a query",
                    "the answering task did not finish",
                )
                .with_recovery("ask again; if it repeats, restart the server"),
            ),
        ),
    };
    ServerFrame::Answered(Box::new(Answered {
        ask_id,
        as_of,
        outcome,
    }))
}

/// What a watching session holds: the token that counts it, and its
/// subscription to the readings.
type Watching = (Box<dyn Send>, broadcast::Receiver<Sample>);

/// The next reading for a watching session; never ready for one that
/// is not watching, so the select above waits on its other arms.
async fn next_sample(
    watching: &mut Option<Watching>,
) -> Result<Sample, broadcast::error::RecvError> {
    match watching {
        Some((_, samples)) => samples.recv().await,
        None => std::future::pending().await,
    }
}

async fn send(socket: &mut WebSocket, frame: &ServerFrame) -> Result<(), ()> {
    let Ok(text) = serde_json::to_string(frame) else {
        return Err(());
    };
    socket
        .send(Message::Text(text.into()))
        .await
        .map_err(|_| ())
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests {
    use kernel::Seq;
    use proptest::prelude::*;

    use super::{Arrival, Said, framed};
    use crate::reception::{SessionState, Stream};

    /// The arrivals a subscription hands over for `ledger` when the
    /// records `skipped` marks are lost: a run of lost records is one
    /// `Skipped`, as a lagged `broadcast` reports it.
    fn arrivals(ledger: &[u64], skipped: &[bool]) -> Vec<Arrival<u64>> {
        let mut arrivals = Vec::new();
        let mut lost = false;
        for (seq, gone) in ledger.iter().zip(skipped) {
            match (gone, lost) {
                (true, true) => {}
                (true, false) => arrivals.push(Arrival::Skipped),
                (false, _) => arrivals.push(Arrival::Record(Seq::new(*seq), *seq)),
            }
            lost = *gone;
        }
        arrivals
    }

    /// `arrivals` cut into frames at `cuts` and told frame by frame.
    fn told_in_frames(
        arrivals: Vec<Arrival<u64>>,
        cuts: &[usize],
        phase: SessionState,
    ) -> (Vec<Said<u64>>, Stream) {
        let mut points: Vec<usize> = cuts.iter().map(|cut| (*cut).min(arrivals.len())).collect();
        points.sort_unstable();
        let mut said = Vec::new();
        let mut stream = Stream::opening();
        let mut rest = arrivals;
        let mut taken = 0;
        for point in points {
            let tail = rest.split_off(point - taken);
            let (told, next) = framed(stream, phase, rest);
            said.extend(told);
            stream = next;
            rest = tail;
            taken = point;
        }
        let (told, next) = framed(stream, phase, rest);
        said.extend(told);
        (said, next)
    }

    proptest! {
        /// `crates/wire/spec/Server/Socket.lean`: cutting the arrivals
        /// into frames anywhere tells what one frame tells
        /// (`framing_is_invisible`); a live session tells exactly the
        /// records it was handed, in seq order (`records_said`,
        /// `said_in_seq_order`); every seq between its first and last
        /// told record is a told record or inside a told `Lagged`
        /// (`every_seq_up_to_the_last_is_told`); a session not yet
        /// welcomed tells nothing (`nothing_said_before_welcome`).
        #[test]
        fn framing_tells_every_record_once_in_order(
            start in 0u64..3,
            gaps in proptest::collection::vec(1u64..4, 0..40),
            skipped in proptest::collection::vec(any::<bool>(), 40),
            cuts in proptest::collection::vec(0usize..48, 0..6),
        ) {
            let ledger: Vec<u64> = gaps
                .iter()
                .scan(start, |seq, gap| {
                    *seq += gap;
                    Some(*seq)
                })
                .collect();
            let handed = arrivals(&ledger, &skipped);
            let kept: Vec<u64> = ledger
                .iter()
                .zip(&skipped)
                .filter(|(_, gone)| !**gone)
                .map(|(seq, _)| *seq)
                .collect();

            let whole = framed(Stream::opening(), SessionState::Live, arrivals(&ledger, &skipped));
            let cut = told_in_frames(handed, &cuts, SessionState::Live);
            prop_assert_eq!(&cut, &whole);

            let records: Vec<u64> = whole
                .0
                .iter()
                .filter_map(|said| match said {
                    Said::Record(seq) => Some(*seq),
                    Said::Lagged(_) => None,
                })
                .collect();
            prop_assert_eq!(&records, &kept);
            prop_assert!(records.windows(2).all(|pair| pair[0] < pair[1]));

            if let (Some(first), Some(last)) = (records.first(), records.last()) {
                for k in *first..=*last {
                    let told = whole.0.iter().any(|said| match said {
                        Said::Record(seq) => *seq == k,
                        Said::Lagged(lag) => lag.from.value() <= k && k <= lag.to.value(),
                    });
                    prop_assert!(told, "seq {} between {} and {} was told nothing", k, first, last);
                }
            }

            let unwelcomed = framed(
                Stream::opening(),
                SessionState::AwaitingHello,
                arrivals(&ledger, &skipped),
            );
            prop_assert!(unwelcomed.0.is_empty());
        }
    }
}
