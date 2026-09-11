// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! How a city is stood up and served, as opposed to how one piece of
//! work is run.
//!
//! Three things happen here and nothing else: the one writer thread is
//! started with the ledger inside it (`serving::attending`), the socket
//! is handed the sinks it may reach the city through, and the whole of
//! it is stopped when the person stops it.

use std::sync::Arc;

use kernel::{AxCode, AxError};

use super::attending::{Outward, Started, spawn_worker};
use super::desk::CommandDesk;
use super::serve::Opening;
use super::serve::Serving;
use crate::assembly::{acp_dispatch, ledger_dir, rebuild_views};
use crate::views::Views;

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

/// Serves one city until the person stops it, and returns when the last
/// worker has finished what it was doing.
///
/// The worker runs on its own thread and the socket never touches the
/// Ledger: a refreshed page cannot kill work, and a command is accepted
/// in one place and executed in another.
///
/// # Errors
/// Refuses before serving when the city cannot be opened — an unreadable
/// chain, a store that will not open — and propagates whatever binding
/// the address reports.
pub async fn serve(serving: Serving) -> Result<(), AxError> {
    let Serving {
        city_root,
        addr,
        token,
        client,
        vault,
        vault_notice,
        log,
        console,
    } = serving;
    let city_root = city_root.as_path();
    let token = token.as_deref();
    let cas_root = city_root.join(".sprawling").join("cas");
    std::fs::create_dir_all(&cas_root).map_err(|source| {
        AxError::failure(
            AxCode::StorageFatal,
            "prepare the upload store",
            format!("{}: {source}", cas_root.display()),
        )
        .with_recovery("check the city directory is writable")
    })?;

    let token_digest = match token {
        Some(raw) => Some(channels::PairingToken::from_configured(raw)?.digest()),
        None => None,
    };

    // The event fan-out, and the one writer that feeds it. The worker
    // owns the ledger, so a city has a single writer no matter how many
    // tabs are open; the socket tasks only read from the broadcast.
    let (events, _first) = tokio::sync::broadcast::channel(1024);
    // Increments, on their own channel. Smaller and separate: an
    // increment nobody received is nothing, and sharing the event
    // channel would let a talkative model push records out of a slow
    // reader's window.
    let (deltas, _watching) = tokio::sync::broadcast::channel(256);
    // The views the control surface reads. Rebuilt from the ledger here,
    // folded forward by the write observer inside the worker: one fold
    // rule, two call sites, no second definition of what a view means.
    let mut rebuilt = rebuild_views(&ledger_dir(city_root))?;
    // One look at this machine, before the socket exists. Every item is
    // a program started and asked its version - seconds rather than
    // milliseconds - so a page asks what the city found rather than
    // making the city look again (sprawling-SPEC.md 8-53).
    rebuilt.found_on_this_machine(crate::doctor::report());
    let views = Arc::new(std::sync::Mutex::new(rebuilt));
    let query_views = Arc::clone(&views);
    // Built once and handed to both surfaces below. The socket and the
    // terminal are two ways into one city, and this is the read half of
    // what makes that literally true rather than a claim.
    let answering: crate::console::Answering = Arc::new(move |query: channels::Query| {
        let mut views = query_views.lock().map_err(|_| {
            AxError::failure(
                AxCode::StorageFatal,
                "read the city views",
                "the view lock is poisoned",
            )
            .with_recovery("restart the server; its views rebuild from the ledger")
        })?;
        Ok(views.answer(&query))
    });
    // Read once, at startup, from the views the ledger just rebuilt.
    let city_name = views.lock().ok().and_then(|views| views.city());
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
            log,
        },
        Outward {
            desk: Arc::clone(&desk),
            views: Arc::clone(&views),
            to_clients: events.clone(),
            to_watchers: deltas.clone(),
        },
    )?;

    let sink_root = cas_root.clone();
    let audio_views = Arc::clone(&views);
    let audio_vault = city_vault;
    let config = channels::ServeConfig {
        addr,
        token_digest,
        client: Arc::new(client),
        commands: Arc::new(
            move |command: channels::WireCommand, reply: channels::Reply| {
                commands_desk.post(command.into(), reply);
                Ok(())
            },
        ),
        events,
        deltas,
        city: city_name,
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
        acp: Arc::new(move |body, authentic| acp_dispatch(&acp_desk, body, authentic)),
        transcribe_sink: hearing(audio_views, audio_vault),
        upload_sink: Arc::new(move |bytes: Vec<u8>| {
            // Attach bytes reach the content-addressed store, and the handle
            // a later Command names is the address they landed at. Nothing
            // enters a work tree here: staging is read-only and outside every
            // WriteDomain.
            let digest = kernel::B3Hash::digest(&bytes).to_string();
            let path = sink_root.join(&digest);
            std::fs::write(&path, &bytes).map_err(|source| {
                AxError::failure(
                    AxCode::StorageFatal,
                    "stage an attachment",
                    format!("{}: {source}", path.display()),
                )
                .with_recovery("check free space under the city directory")
            })?;
            channels::UploadId::parse(&digest)
        }),
    };
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
        result = channels::serve(config) => result,
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
    desk.close();
    // Joined rather than left to the process exit: the handoff is
    // written by that thread, and a main that returned first would end
    // the process before the line it exists to write.
    if let Err(panicked) = worker_thread.join() {
        eprintln!("the run worker ended abnormally: {panicked:?}");
    }
    served
}
