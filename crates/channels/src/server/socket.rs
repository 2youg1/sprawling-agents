// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One WebSocket session, from the upgrade to its end: the shell around
//! [`decide_frame`]. Every branch is a send, a receive, or the end of
//! the session; the judgements are `channels::reception`'s.
//!
//! A refusal made minutes later has no way home, which is why a command
//! carries the [`Reply`] address of whoever sent it.

use std::sync::Arc;

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use kernel::{AxCode, AxError, Seq};
use tokio::sync::broadcast;

use crate::reception::inbound::Inbound;
use crate::reception::{SessionState, SessionStep, Stream, WelcomeFacts, decide_frame};
use crate::wire::{Answered, Ask, AskOutcome, Sample, ServerFrame};

use super::config::{Answering, ShellState};
use super::reply::{Delivered, Reply};

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
                match event {
                    Ok(committed) => {
                        if phase == SessionState::Live {
                            let seq = committed.record().seq();
                            // The far end of a skipped range is only knowable
                            // here, at the record that ends it: the count the
                            // subscription reports says how many messages went
                            // by and names no endpoint of the range they went
                            // by in.
                            let (lag, next) = stream.before(seq);
                            if let Some(lag) = lag
                                && send(&mut socket, &ServerFrame::Lagged(lag)).await.is_err()
                            {
                                return;
                            }
                            if socket.send(Message::Text(committed.frame())).await.is_err() {
                                return;
                            }
                            stream = next;
                        }
                    }
                    // A slow client does not hold the writer back: the
                    // subscription leaves it behind and the Ledger answers
                    // the range it lost. It is told which range that is, so
                    // it can ask; before this, the middle of the stream
                    // vanished and nothing said so.
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        stream = stream.skipped(phase == SessionState::Live);
                    }
                    // The sender is gone, which means the city is going down:
                    // there is nothing left to wait for, and a receiver left
                    // on a closed channel answers immediately for ever.
                    Err(broadcast::error::RecvError::Closed) => return,
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
