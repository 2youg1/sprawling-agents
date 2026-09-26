// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! How a city is stood up and served, as opposed to how one piece of
//! work is run.
//!
//! Three things happen here and nothing else: the port is taken and the
//! one writer thread is started with the ledger inside it
//! (`serving::attending`), in that order ([`listen`]); the socket is
//! handed the sinks it may reach the city through; and the whole of it
//! is stopped when the person stops it ([`Listening::serve`]).
//!
//! The order is sprawling-SPEC.md 8-88, and this file is its one
//! definition: a port another process holds is refused before a writer
//! exists, so a refused serve leaves the Ledger exactly as it found it.

use std::sync::Arc;

use kernel::{AxCode, AxError};

use super::attending::{Outward, Started, spawn_worker};
use super::desk::CommandDesk;
use super::output_ring::OutputRing;
use super::serve::Opening;
use super::serve::Serving;
use crate::assembly::{Closing, acp_dispatch, fold_city, ledger_dir};
use crate::views::{Views, answer_outside_the_lock};

/// One recording in, one line of text back.
///
/// The choice of endpoint is read from the city's own state under its
/// lock, and the provider round trip happens after that lock is gone: a
/// call that takes ten seconds must not hold the answer to every other
/// read for ten seconds.
fn hearing(
    views: Arc<std::sync::Mutex<Views>>,
    vault: Arc<std::sync::Mutex<gateway::Custodian>>,
) -> channels::TranscribeSink {
    Arc::new(move |bytes: Vec<u8>, media: String| {
        let recording = gateway::Recording::new(bytes, gateway::AudioType::of_media_type(&media)?)?;
        let speaking = {
            let held = views.lock().map_err(|_| {
                AxError::failure(
                    AxCode::StorageFatal,
                    "read the city views",
                    "the view lock is poisoned",
                )
                .with_recovery("restart the server; its views rebuild from the ledger")
            })?;
            held.transcriber(crate::assembly::resolving(Arc::clone(&vault)))?
        };
        speaking.transcribe(&recording)
    })
}

/// Waits for the person to stop the city from the keyboard.
///
/// A Windows console delivers two of these - Ctrl-C and Ctrl-Break - and
/// a city that closed on one and died on the other would be two
/// behaviours for one gesture, decided by which key a person happened to
/// press. Elsewhere there is one.
#[cfg(windows)]
async fn closed_by_hand() -> std::io::Result<()> {
    let mut broken = tokio::signal::windows::ctrl_break()?;
    tokio::select! {
        result = tokio::signal::ctrl_c() => result,
        _ = broken.recv() => Ok(()),
    }
}

#[cfg(not(windows))]
async fn closed_by_hand() -> std::io::Result<()> {
    tokio::signal::ctrl_c().await
}

/// A city that has taken its port and opened its one writer, and does
/// not answer yet.
///
/// Holding one is the proof that every refusal a serve can make has
/// been made: the caller prints "running" only once it has this value.
/// Dropped without [`Listening::serve`], the writer thread waits until
/// the process ends and writes no handoff.
#[must_use = "a listening city answers nobody until it is served"]
pub struct Listening {
    bound: channels::Bound,
    config: channels::ServeConfig,
    desk: Arc<CommandDesk>,
    answering: crate::console::Answering,
    worker: std::thread::JoinHandle<()>,
    console: Option<crate::console::Terminal>,
}

/// Takes the city's port, then opens its one writer.
///
/// The worker runs on its own thread and the socket never touches the
/// Ledger: a refreshed page cannot kill work, and a command is accepted
/// in one place and executed in another.
///
/// # Errors
/// Refuses before a writer exists when the address may not or cannot be
/// bound (`E_CONFIG_INVALID`); refuses before anything is written when
/// another process holds the city (`E_LEDGER_HELD`) or the city cannot
/// be opened - an unreadable chain, a store that will not open.
pub async fn listen(serving: Serving) -> Result<Listening, AxError> {
    let Serving {
        city_root,
        addr,
        token,
        client,
        vault,
        vault_notice,
        log,
        journal,
        console,
    } = serving;
    let city_root = city_root.as_path();
    let token_digest = match token.as_deref() {
        Some(raw) => Some(channels::PairingToken::from_configured(raw)?.digest()),
        None => None,
    };
    // The port first: a serve refused here has opened nothing and
    // written nothing.
    let bound = channels::bind(addr, token_digest).await?;
    let cas_root = kernel::layout::CityLayout::new(city_root).cas();
    std::fs::create_dir_all(&cas_root).map_err(|source| {
        AxError::failure(
            AxCode::StorageFatal,
            "prepare the upload store",
            format!("{}: {source}", cas_root.display()),
        )
        .with_recovery("check the city directory is writable")
    })?;

    // The event fan-out, and the one writer that feeds it. The worker
    // owns the ledger, so a city has a single writer no matter how many
    // tabs are open; the socket tasks only read from the broadcast.
    let events = tokio::sync::broadcast::Sender::new(1024);
    // Increments, on their own channel. Smaller and separate: an
    // increment nobody received is nothing, and sharing the event
    // channel would let a talkative model push records out of a slow
    // reader's window.
    let deltas = tokio::sync::broadcast::Sender::new(256);
    // Running commands' output, on a fourth channel for the same reason.
    let outputs = tokio::sync::broadcast::Sender::new(256);
    // And what they already wrote, for a page that opens mid-command.
    let kept = Arc::new(OutputRing::default());
    let kept_reader = Arc::clone(&kept);
    // The process log, on the third channel. Its sender was made before
    // the `Diagnostics` was, because the sink is what writes into it.
    let logs = journal.lines();
    // The views the control surface reads, and what the worker inherits,
    // folded from one verified read of the ledger here; the views are
    // folded forward by the write observer inside the worker: one fold
    // rule, two call sites, no second definition of what a view means.
    let (rebuilt, held) = fold_city(&ledger_dir(city_root))?;
    // This machine is not asked here (sprawling-SPEC.md 8-54): the
    // table is thirty-two items, most of them a program started and
    // asked its version, and a serve that waited for all of them holds
    // the socket shut for seconds to answer a question only one page
    // asks. The answer stays `None` until `DoctorRefresh` fills it,
    // which is the one verb that asks this machine.
    let views = Arc::new(std::sync::Mutex::new(rebuilt));
    let query_views = Arc::clone(&views);
    // Built once and handed to both surfaces below. The socket and the
    // terminal are two ways into one city, and this is the read half of
    // what makes that literally true rather than a claim.
    let answering: crate::console::Answering =
        Arc::new(move |query: channels::Query| answer_outside_the_lock(&query_views, &query));
    // Read once, at startup, from the views the ledger just rebuilt.
    let city_name = views.lock().ok().and_then(|views| views.city());
    let epoch = views.lock().ok().and_then(|views| views.epoch());
    let head = Arc::new(channels::LedgerHead::at(
        views.lock().ok().and_then(|views| views.head()),
    ));
    // The in-process Command set, not the wire one: the enrolment
    // route delivers a sealed credential here, and no wire frame can.
    let desk = Arc::new(CommandDesk::new());
    let commands_desk = Arc::clone(&desk);
    let secrets_desk = Arc::clone(&desk);
    let acp_desk = Arc::clone(&desk);
    // The one sanctioned thread besides the runtime's own, running by
    // the time this returns.
    let Started {
        thread: worker_thread,
        vault: city_vault,
    } = spawn_worker(
        Opening {
            city_root: city_root.to_path_buf(),
            vault,
            notice: vault_notice,
            audit_log: log
                .floor()
                .map_or_else(runtime::diagnostics::Diagnostics::off, |floor| {
                    runtime::diagnostics::Diagnostics::new(floor, journal.sink())
                }),
            log,
            held,
        },
        Outward {
            desk: Arc::clone(&desk),
            views: Arc::clone(&views),
            to_clients: events.clone(),
            to_watchers: deltas.clone(),
            head: Arc::clone(&head),
            to_readers: outputs.clone(),
            kept,
        },
    )?;

    // The vault the worker opened, lent to the reads that need one.
    // Lent rather than opened a second time: "a credential is redeemed
    // at the last moment, through one door" stops being true the moment
    // there are two handles on the same secrets.
    if let Ok(mut held) = views.lock() {
        held.lend_the_vault(Arc::clone(&city_vault));
    }
    let audio_views = Arc::clone(&views);
    let audio_vault = city_vault;
    let config = channels::ServeConfig {
        client: Arc::new(client),
        commands: Arc::new(
            move |command: channels::WireCommand, reply: channels::Reply| {
                commands_desk.post(command.into(), reply);
                Ok(())
            },
        ),
        events,
        deltas,
        logs,
        outputs,
        outputs_so_far: Arc::new(move || kept_reader.so_far()),
        monitor: watched(city_root)?,
        city: city_name,
        head,
        epoch,
        secrets: Arc::new(move |command: channels::Command, reply: channels::Reply| {
            // The route waits for whichever comes first, so the
            // reply address is the credential's own request rather
            // than nowhere: a vault that refuses is a fact the
            // person typing the key needs, and it used to reach
            // nobody at all.
            secrets_desk.post(command, reply);
            Ok(())
        }),
        queries: Arc::clone(&answering),
        // An outside editor's request becomes an ordinary Dispatch on
        // the same desk a person's does. It is not a second control
        // surface: the admission decides what a stranger may learn, and
        // everything after that is the city's usual path.
        acp: Arc::new(move |body, pairing| acp_dispatch(&acp_desk, body, pairing)),
        transcribe_sink: hearing(audio_views, audio_vault),
    };
    Ok(Listening {
        bound,
        config,
        desk,
        answering,
        worker: worker_thread,
        console,
    })
}

impl Listening {
    /// Answers until the person stops the city, and returns once the
    /// writer thread has written its handoff and ended.
    ///
    /// # Errors
    /// Propagates the accept failures the listener reports, and a
    /// signal handler that cannot be installed.
    pub async fn serve(self) -> Result<(), AxError> {
        let Listening {
            bound,
            config,
            desk,
            answering,
            worker,
            console,
        } = self;
        // The terminal this city is running in, if it was asked for. It gets
        // the same desk the socket posts to and the same event stream the
        // browser reads, so nothing here is a second control surface - it is
        // the first one, reached from the keyboard that started the city.
        if let Some(terminal) = console {
            let console_desk = Arc::clone(&desk);
            let watching = config.events.subscribe();
            // The same answering function the socket was given, not a second
            // one built beside it: a count this terminal prints and a count
            // a browser draws are one call, so they cannot disagree.
            crate::console::start(terminal, console_desk, Arc::clone(&answering), watching);
        }
        // Ctrl-C used to be a process death: `sprawling resume` recovered
        // it, and a stop somebody chose and a stop that was a crash left the
        // same silence in the record. The listener stops accepting first,
        // then the worker is told - it reads that where it reads its queue,
        // so whatever command is running finishes and the handoff is the
        // last line rather than a line in the middle of one.
        let served = tokio::select! {
            result = channels::serve(bound, config) => result,
            signal = closed_by_hand() => {
                // A signal handler that cannot be installed is worth saying
                // out loud: the city keeps serving, and the person now knows
                // that Ctrl-C will be the hard stop it always was.
                signal.map_err(|source| {
                    AxError::failure(
                        AxCode::StorageFatal,
                        "listen for an orderly close",
                        source.to_string(),
                    )
                    .with_recovery("stop the city from the console instead; /quit closes it")
                })
            }
        };
        desk.close(Closing::of(&served));
        // Joined rather than left to the process exit: the handoff is
        // written by that thread, and a main that returned first would end
        // the process before the line it exists to write.
        if let Err(panicked) = worker.join() {
            eprintln!("the run worker ended abnormally: {panicked:?}");
        }
        served
    }
}

/// The monitor a session watches over the socket, and the thread that
/// samples it once a second (sprawling-SPEC.md 8-90, 8-92). Whether
/// anybody watches is the monitor's count; a session holds its
/// [`crate::monitor::Watch`] for as long as it watches.
///
/// # Errors
/// `StorageFatal` when the sampler's thread cannot be started.
fn watched(city_root: &std::path::Path) -> Result<channels::MonitorFeed, AxError> {
    let monitor = Arc::new(std::sync::Mutex::new(crate::monitor::Monitor::new()));
    let samples = tokio::sync::broadcast::channel(1).0;
    crate::monitor::sampler::spawn_sampler(
        Arc::downgrade(&monitor),
        samples.clone(),
        city_root.to_path_buf(),
    )?;
    Ok(channels::MonitorFeed {
        watch: Arc::new(move |watched| -> Box<dyn Send> {
            // The count is an atomic, so a poisoned lock guards no
            // half-written state and the watcher still counts.
            let held = monitor
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            Box::new(held.watch(watched))
        }),
        samples,
    })
}
