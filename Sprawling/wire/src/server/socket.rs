// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

mod framing;

use std::sync::Arc;

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use kernel::{AxCode, AxError, Seq};
use tokio::sync::broadcast;

use crate::frames::{Answered, Ask, AskOutcome, ClientFrame, Sample, ServerFrame};
use crate::reception::inbound::Inbound;
use crate::reception::{
    Keys, SessionState, SessionStep, Standing, Stream, WelcomeFacts, decide_frame, decide_standing,
};
use crate::reply::{Delivered, Reply};

use super::config::{Answering, ShellState};
use super::seat::Seat;
use framing::{Arrival, Ending, drained, framed, write_frame};

pub(crate) async fn upgrade(
    State(state): State<Arc<ShellState>>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(move |socket| session(socket, state))
}

async fn session(mut socket: WebSocket, state: Arc<ShellState>) {
    let mut phase = SessionState::AwaitingHello;
    let mut inbound = Inbound::new();
    let mut events = state.events.subscribe();
    let mut deltas = state.deltas.subscribe();
    let mut logs = state.logs.subscribe();
    let mut outputs = state.outputs.subscribe();

    let (refused, mut refusals) = tokio::sync::mpsc::unbounded_channel::<AxError>();

    let mut stream = Stream::opening();
    let mut watching: Option<Watching> = None;
    let mut seat: Option<Seat> = None;
    loop {
        tokio::select! {
            incoming = socket.recv() => {
                let Some(Ok(message)) = incoming else { return };
                let Message::Text(text) = message else { continue };
                let (step, presented) = judged(&state, phase, &mut inbound, &text);
                match step {
                    SessionStep::Welcome(welcome) => {
                        match state.door.seat(presented) {
                            Ok(seated) => seat = Some(seated),
                            Err(error) => {

                                let _unsent: Result<(), ()> = send(&mut socket, &ServerFrame::Refusal(Box::new(error))).await;
                                return;
                            }
                        }
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
                    SessionStep::Beat(beat) => (state.monitor.beat)(beat),
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

            () = seated(&mut seat) => {
                let presented = seat.as_ref().and_then(Seat::presented);
                let standing = state.door.pairing(&state.face, presented).map(decide_standing);
                let error = match standing {
                    Ok(Standing::Stands) => continue,
                    Ok(Standing::Ends(error)) => error,
                    Err(error) => Box::new(error),
                };

                let _unsent: Result<(), ()> = send(&mut socket, &ServerFrame::Refusal(error)).await;
                return;
            }

            late = refusals.recv() => {
                let Some(error) = late else { return };
                if send(&mut socket, &ServerFrame::Refusal(Box::new(error))).await.is_err() {
                    return;
                }
            }
            event = events.recv() => {

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

fn judged(
    state: &ShellState,
    phase: SessionState,
    inbound: &mut Inbound,
    text: &str,
) -> (SessionStep, Option<String>) {
    let frame = match inbound.read(text) {
        Ok(frame) => frame,
        Err(unreadable) => return (unreadable, None),
    };
    let presented = match &frame {
        ClientFrame::Hello(hello) => hello.token.clone(),
        ClientFrame::Command(_) | ClientFrame::Ask(_) | ClientFrame::Monitor(_) => None,
    };
    let standing = WelcomeFacts {
        city: state.city.as_ref(),
        head: state.head.read(),
        epoch: state.epoch,
    };
    let step = state
        .door
        .with_sessions(|sessions, now| {
            let keys = Keys {
                face: &state.face,
                sessions,
                now,
            };
            decide_frame(phase, frame, &keys, standing)
        })
        .unwrap_or_else(|error| SessionStep::Refuse {
            error: Box::new(error),
            close: true,
        });
    (step, presented)
}

async fn seated(seat: &mut Option<Seat>) {
    match seat {
        Some(seat) => seat.ended().await,
        None => std::future::pending().await,
    }
}

async fn answered(answering: Answering, Ask { ask_id, query }: Ask) -> ServerFrame {
    let (as_of, outcome) = match tokio::task::spawn_blocking(move || answering(query)).await {
        Ok((as_of, Ok(answer))) => (as_of, AskOutcome::Answer(answer)),
        Ok((as_of, Err(error))) => (as_of, AskOutcome::Refusal(error)),

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

type Watching = (Box<dyn Send>, broadcast::Receiver<Sample>);

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
