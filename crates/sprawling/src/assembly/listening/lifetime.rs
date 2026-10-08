// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The served city carried through its lifecycle
//! (`crates/sprawling/spec/Console/Lifecycle.lean`, `crates/sprawling/spec/Console.lean` §8-11).
//!
//! Every way a served process can be asked to change - a signal, the
//! console's `/web` and `/quit`, a page's `CloseCity`, a lost terminal,
//! the listener failing, the worker landing, the time limit passing -
//! arrives here as one `Event` and is fed to `console::lifecycle::step`,
//! which alone decides what it means. This file only carries the answer
//! out: the desk closed with the cause the step gave, the runs
//! interrupted, the limit armed, the listener dropped, the face
//! published for the console to draw.
//!
//! The point a reader most often gets wrong: the listener stops
//! accepting the moment the city starts closing, and the worker is
//! joined on a blocking thread, so a close that waits for runs keeps the
//! reactor free to serve the lanes that are landing.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use accounting::worker::{Closing, CommandDesk};
use kernel::{AxCode, AxError};
use wire::CloseMode;

use crate::console::lifecycle::{self, Asker, Cause, Deadline, Event, Face, Handoff, Surface};

/// How long a city whose terminal went away has to write its handoff.
/// Windows ends a process five seconds after its console window closes;
/// four leaves the handoff a second, and the other platforms take the
/// same number so the three behave alike.
pub(crate) const LOST_TERMINAL_GRACE: Duration = Duration::from_secs(4);

/// How many events may wait for the lifecycle: a person types slower
/// than this drains, and a signal that finds it full was a repeat.
pub(crate) const EVENTS: usize = 64;

/// What the lifecycle carries its answers out on.
pub(crate) struct Living {
    pub(crate) desk: Arc<CommandDesk>,
    pub(crate) worker: std::thread::JoinHandle<()>,
    pub(crate) faces: tokio::sync::watch::Sender<Face>,
    pub(crate) surface: Surface,
}

/// Feeds every event to the lifecycle until the process is gone, and
/// answers how it went: `Ok` once the handoff is written after a close
/// somebody asked for, the failure when serving failed, and a refusal
/// when the process ends without its handoff.
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
            // The listener stops accepting first; the worker is joined
            // off the reactor, which keeps serving the lanes that land.
            serving = None;
            landing = worker
                .take()
                .map(|worker| tokio::task::spawn_blocking(move || worker.join()));
        }
        if let Face::Stopping {
            mode: CloseMode::Interrupt,
            deadline: armed,
        } = next
        {
            desk.interrupt();
            let newly_armed = !matches!(
                face,
                Face::Stopping {
                    deadline: Deadline::Armed,
                    ..
                }
            );
            if armed == Deadline::Armed && newly_armed {
                deadline = Some(Box::pin(tokio::time::sleep(LOST_TERMINAL_GRACE)));
            }
        }
        face = next;
        match faces.send(face) {
            // Nobody watching the face is a city with no console.
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

/// The Commands the listener is given, with a page's `CloseCity` taken
/// first and handed to the lifecycle (`crates/sprawling/spec/Console.lean` §8-11).
///
/// Accepted only while the listener is bound to loopback: the command
/// sink carries no peer address, and a loopback bind is the one case in
/// which every peer is on this machine without asking. The relay never
/// brings it, because its class is `LocalOnly`.
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
                // A full queue holds a close already on its way, and a
                // closed one belongs to a city that is already closing:
                // either way the page asked for what is happening.
                Ok(()) | Err(_) => Ok(()),
            }
        }
    })
}

/// The closing the desk is told, from the cause the step gave.
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

/// A future that may be absent: absent, it never completes.
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

/// The signals a served process listens for, each read as the event it
/// is. A handler that cannot be installed leaves that signal at its
/// platform default, which is said once; the city keeps serving.
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

/// One handler, or nothing and a line that says so.
fn installed<T>(name: &str, handler: std::io::Result<T>) -> Option<T> {
    match handler {
        Ok(handler) => Some(handler),
        Err(source) => {
            // Said before the console takes the terminal, which is the
            // one moment a line on standard error reaches nobody's screen
            // twice.
            eprintln!(
                "{name} cannot be listened for ({source}); it ends the city without a handoff"
            );
            None
        }
    }
}

/// The next arrival of one signal; a handler that is absent, or whose
/// stream ended, never completes.
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
