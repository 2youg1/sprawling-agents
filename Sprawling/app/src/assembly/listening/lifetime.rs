// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Lifecycle carry-out for a served city
//! (`Sprawling/app/spec/Console/Lifecycle.lean`, `Sprawling/app/spec/Console.lean` §8-11).

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use accounting::worker::{Closing, CommandDesk};
use kernel::{AxCode, AxError};
use wire::CloseMode;

use crate::console::lifecycle::{
    self, Asker, Cause, Event, Face, Handoff, INTERRUPT_GRACE, Surface,
};

pub(crate) const EVENTS: usize = 64;

pub(crate) struct Living {
    pub(crate) desk: Arc<CommandDesk>,
    pub(crate) worker: std::thread::JoinHandle<()>,
    pub(crate) faces: tokio::sync::watch::Sender<Face>,
    pub(crate) surface: Surface,
}

pub(crate) async fn live(
    serving: impl Future<Output = Result<(), AxError>>,
    mut events: tokio::sync::mpsc::Receiver<Event>,
    living: Living,
) -> Result<(), AxError> {
    let Living {
        desk,
        worker,
        faces,
        surface,
    } = living;
    let mut signals = Signals::install();
    let mut serving: Option<Pin<Box<_>>> = Some(Box::pin(serving));
    let mut worker = Some(worker);
    let mut landing: Option<tokio::task::JoinHandle<std::thread::Result<()>>> = None;
    let mut deadline: Option<Pin<Box<tokio::time::Sleep>>> = None;
    let mut failure: Option<AxError> = None;
    let mut face = Face::Opening;
    let mut next_event = Some(Event::Ready(surface));
    loop {
        let event = match next_event.take() {
            Some(event) => event,
            None => tokio::select! {
                served = maybe(&mut serving) => {
                    serving = None;
                    failure = Some(served.err().unwrap_or_else(listener_ended));
                    Event::Failed
                }
                Some(event) = events.recv() => event,
                event = signals.next() => event,
                joined = maybe(&mut landing) => {
                    landing = None;
                    if !matches!(joined, Ok(Ok(()))) {
                        failure.get_or_insert_with(worker_ended);
                    }
                    Event::Landed
                }
                () = maybe(&mut deadline) => {
                    deadline = None;
                    Event::Deadline
                }
            },
        };
        let (next, cause) = lifecycle::step(face, event);
        if let Some(cause) = cause {
            desk.close(closing(cause, next, failure.as_ref()));
            serving = None;
            landing = worker
                .take()
                .map(|worker| tokio::task::spawn_blocking(move || worker.join()));
        }
        if next.armed() {
            desk.interrupt();
            if !face.armed() {
                deadline = Some(Box::pin(tokio::time::sleep(INTERRUPT_GRACE)));
            }
        }
        face = next;
        match faces.send(face) {
            Ok(()) | Err(_) => {}
        }
        match face {
            Face::Gone(Handoff::Written) => return failure.map_or(Ok(()), Err),
            Face::Gone(Handoff::Skipped) => return Err(without_a_handoff()),
            Face::Opening
            | Face::Cli
            | Face::QuietHost
            | Face::Headless
            | Face::Stopping { .. } => {}
        }
    }
}

/// `CloseCity` from a page reaches the lifecycle only on loopback
/// (`Sprawling/app/spec/Console.lean` §8-11); the sink has no peer address.
pub(super) fn closing_from_a_page(
    at: std::net::SocketAddr,
    lifecycle: tokio::sync::mpsc::Sender<Event>,
    rest: crate::outside::asking::Commands,
) -> crate::outside::asking::Commands {
    Arc::new(move |command, reply| {
        let wire::WireCommand::CloseCity(wire::CityClosing { mode, .. }) = command else {
            return rest(command, reply);
        };
        {
            if !at.ip().is_loopback() {
                return Err(AxError::failure(
                    AxCode::GateDenied,
                    "close the city",
                    "a city listening beyond this machine is closed only from its own terminal",
                )
                .with_recovery("type /quit in the terminal the city runs in"));
            }
            match lifecycle.try_send(Event::Quit(Asker::Page, mode)) {
                Ok(()) | Err(_) => Ok(()),
            }
        }
    })
}

fn closing(cause: Cause, next: Face, failure: Option<&AxError>) -> Closing {
    let mode = match next {
        Face::Stopping { mode, .. } => mode,
        Face::Opening | Face::Cli | Face::QuietHost | Face::Headless | Face::Gone(_) => {
            CloseMode::Drain
        }
    };
    match cause {
        Cause::Closed(by) => Closing::Chosen { by, mode },
        Cause::Failed => Closing::Broken {
            cause: failure.map_or_else(|| listener_ended().to_string(), ToString::to_string),
        },
    }
}

async fn maybe<F: Future + Unpin>(future: &mut Option<F>) -> F::Output {
    match future {
        Some(future) => future.await,
        None => std::future::pending().await,
    }
}

fn listener_ended() -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        "serve the city",
        "the listener stopped accepting connections",
    )
    .with_recovery("start the city again; `sprawling resume` first if a run was going")
}

fn worker_ended() -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        "close the city",
        "the run worker ended abnormally",
    )
    .with_recovery("run `sprawling resume` on this city, which closes what was left open")
}

fn without_a_handoff() -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        "close the city",
        "the city exits without a handoff",
    )
    .with_recovery("run `sprawling resume` on this city, which closes what was left open")
}

struct Signals {
    #[cfg(windows)]
    interrupt: Option<tokio::signal::windows::CtrlC>,
    #[cfg(windows)]
    broken: Option<tokio::signal::windows::CtrlBreak>,
    #[cfg(windows)]
    closed: Option<tokio::signal::windows::CtrlClose>,
    #[cfg(unix)]
    interrupt: Option<tokio::signal::unix::Signal>,
    #[cfg(unix)]
    terminate: Option<tokio::signal::unix::Signal>,
    #[cfg(unix)]
    hangup: Option<tokio::signal::unix::Signal>,
}

fn installed<T>(name: &str, handler: std::io::Result<T>) -> Option<T> {
    match handler {
        Ok(handler) => Some(handler),
        Err(source) => {
            eprintln!(
                "{name} cannot be listened for ({source}); it ends the city without a handoff"
            );
            None
        }
    }
}

macro_rules! arrival {
    ($signal:expr) => {
        async {
            loop {
                match $signal.as_mut() {
                    Some(signal) => {
                        if signal.recv().await.is_some() {
                            break;
                        }
                    }
                    None => std::future::pending::<()>().await,
                }
                $signal = None;
            }
        }
    };
}

#[cfg(windows)]
impl Signals {
    fn install() -> Signals {
        use tokio::signal::windows;
        Signals {
            interrupt: installed("Ctrl-C", windows::ctrl_c()),
            broken: installed("Ctrl-Break", windows::ctrl_break()),
            closed: installed("closing the window", windows::ctrl_close()),
        }
    }

    async fn next(&mut self) -> Event {
        let Signals {
            interrupt,
            broken,
            closed,
        } = self;
        tokio::select! {
            () = arrival!(*interrupt) => Event::InterruptSignal,
            () = arrival!(*broken) => Event::BreakSignal,
            () = arrival!(*closed) => Event::TerminalLost,
        }
    }
}

#[cfg(unix)]
impl Signals {
    fn install() -> Signals {
        use tokio::signal::unix::{SignalKind, signal};
        Signals {
            interrupt: installed("SIGINT", signal(SignalKind::interrupt())),
            terminate: installed("SIGTERM", signal(SignalKind::terminate())),
            hangup: installed("SIGHUP", signal(SignalKind::hangup())),
        }
    }

    async fn next(&mut self) -> Event {
        let Signals {
            interrupt,
            terminate,
            hangup,
        } = self;
        tokio::select! {
            () = arrival!(*interrupt) => Event::InterruptSignal,
            () = arrival!(*terminate) => Event::Terminate,
            () = arrival!(*hangup) => Event::TerminalLost,
        }
    }
}
