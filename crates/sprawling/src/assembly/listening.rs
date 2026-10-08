// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! How a city is stood up and served, as opposed to how one piece of
//! work is run.
//!
//! Three things happen here and nothing else: the port is taken and the
//! one writer thread is started with the ledger inside it
//! (`assembly::attending`), in that order ([`listen`]); the socket is
//! handed the sinks it may reach the city through; and the whole of it
//! is stopped when the person stops it ([`Listening::serve`]).
//!
//! The order is `crates/sprawling/spec/Assembly/Listening.lean` §8-88, and this file is its one
//! definition: a port another process holds is refused before a writer
//! exists, so a refused serve leaves the Ledger exactly as it found it.
//! The properties of that order are proved in
//! `crates/sprawling/spec/Assembly/Listening.lean`.

use std::sync::Arc;

use kernel::{AxCode, AxError};

use super::attending::{Opening, Outward, Started, spawn_worker};
use super::remote_door::{CityPort, Outdoors};
use crate::doctor::host;
use crate::monitor::sampler::Gauges;
use crate::outside::keeper::CityKey;
use crate::serving::Serving;
use crate::serving::output_ring::OutputRing;
use crate::serving::standing::monotonic_now;
use accounting::views::Published;
use accounting::worker::opening_cost::{OpeningCost, Phase};
use accounting::worker::{Closing, CommandDesk, acp_dispatch, start_served_views};
use monitor_feed::watched;

/// One recording in, one line of text back.
///
/// The choice of endpoint is read from a snapshot of the city's own
/// state, and the provider round trip happens after that snapshot is let
/// go: a call that takes ten seconds must not keep a retired copy of the
/// views from the fold for ten seconds.
fn hearing(
    views: Arc<Published>,
    vault: Arc<std::sync::Mutex<gateway::Custodian>>,
) -> wire::TranscribeSink {
    Arc::new(move |bytes: Vec<u8>, media: String| {
        let recording = gateway::Recording::new(bytes, gateway::AudioType::of_media_type(&media)?)?;
        let speaking = views
            .snapshot()
            .transcriber(accounting::held_vault::resolving(Arc::clone(&vault)))?;
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
    bound: wire::Bound,
    config: wire::ServeConfig,
    desk: Arc<CommandDesk>,
    answering: crate::console::Answering,
    worker: std::thread::JoinHandle<()>,
    outdoors: Outdoors,
    front: crate::outside::asking::Front,
    doorstep: doorstep::Doorstep,
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
        mut log,
        journal,
        core,
    } = serving;
    let city_root = city_root.as_path();
    let key = wire::PairingToken::from_configured(&token)?.digest();
    // Each phase of opening is lapped on the monotonic sampling point,
    // and said in one line once the writer runs (`crates/sprawling/spec/Assembly/Listening.lean` §8-121).
    let mut cost = OpeningCost::begin(monotonic_now);
    // The port first: a serve refused here has opened nothing and
    // written nothing.
    let bound = wire::bind(addr, Some(key)).await?;
    // Every reader past this point is handed the address the listener
    // holds, which differs from `addr` when `addr` asked for port 0
    // (`crates/wire/Spec.lean` §8-46, wire D16).
    let at = bound.local_addr();
    cost.lap(Phase::Bind);
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
    // The views the control surface reads, and what the worker inherits, started together
    // from their snapshots on one pass over what they have not folded (`crates/sprawling/spec/Assembly/Listening.lean`
    // §8-122); the views are folded forward by the write observer inside the worker: one fold
    // rule, two call sites, no second definition of what a view means. The history before
    // the snapshots is proved behind the first byte.
    let (mut rebuilt, held) = start_served_views(
        &kernel::layout::CityLayout::new(city_root).ledger(),
        accounting::Clock::now(&super::SystemClock)?,
        &mut log,
        &mut cost,
    )?;
    // The fold thread alternates between two copies, so the second is
    // made here from the first (`crates/sprawling/spec/Assembly/Listening.lean` §8-99).
    rebuilt.ask_the_registry_through(crate::release::answer);
    rebuilt.ask_upstream_through(crate::doctor::newest);
    rebuilt.look_for_harnesses_through(host::find_program, host::place_set_up);
    rebuilt.ask_github_through(crate::doctor::github::login);
    // One verdict for the writer and the views (`crates/sprawling/spec/Assembly.lean` §8-134).
    let halt = storage::ChainHalt::awaiting_proof();
    rebuilt.watch_proof(halt.clone());
    let spare = rebuilt.twin()?;
    cost.lap(Phase::Twin);
    // This machine is not asked here (`crates/sprawling/spec/Doctor.lean` §8-54): asking
    // holds the socket shut for seconds, so the answer waits for `DoctorRefresh`.
    let views = Arc::new(Published::new(rebuilt));
    let privacy = super::privacy::page();
    // Built once and handed to both surfaces below. The socket and the
    // terminal are two ways into one city, and this is the read half of
    // what makes that literally true rather than a claim.
    let answering = super::privacy::answering(Arc::clone(&views), privacy.clone());
    // Read once, at startup, from the views the ledger just rebuilt.
    let started_from = views.snapshot();
    let city_name = started_from.city();
    let epoch = started_from.epoch();
    let started_at = started_from.head();
    let head = Arc::new(wire::LedgerHead::at(started_at));
    drop(started_from);
    // The in-process Command set, not the wire one: the enrolment
    // route delivers a sealed credential here, and no wire frame can.
    let desk = Arc::new(CommandDesk::default());
    let secrets_desk = Arc::clone(&desk);
    let acp_desk = Arc::clone(&desk);
    // Taken before `log` moves into the worker: the opening line is said
    // after the worker runs, at the same floor and through the same sink.
    let mut opening_log = journal.beside(&log);
    // The one sanctioned thread besides the runtime's own, running by
    // the time this returns.
    let Started {
        thread: worker_thread,
        vault: city_vault,
        health,
        backlog,
        relay,
    } = spawn_worker(
        Opening {
            city_root: city_root.to_path_buf(),
            vault,
            notice: vault_notice,
            audit_log: journal.beside(&log),
            began: cost.began(),
            log,
            held,
            core,
            halt,
        },
        Outward {
            desk: Arc::clone(&desk),
            views: Arc::clone(&views),
            spare,
            to_clients: events.clone(),
            to_watchers: deltas.clone(),
            head: Arc::clone(&head),
            to_readers: outputs.clone(),
            kept,
        },
    )?;
    cost.lap(Phase::StartWorker);
    opening_log.write(
        runtime::diagnostics::Level::Effect,
        runtime::diagnostics::Site {
            run: kernel::RunId::CITY,
            seq: started_at.unwrap_or(kernel::Seq::FIRST),
            module: "bin::assembly",
        },
        &cost.line(),
    );

    let audio_views = Arc::clone(&views);
    let remote_key = CityKey::of(Arc::clone(&city_vault), epoch);
    let audio_vault = city_vault;
    let page = Arc::new(client);
    let front = crate::outside::asking::Front::default();
    let (door, commands, queries) = doorstep::wired(
        city_root,
        super::privacy::commands(&front, Arc::clone(&desk), privacy),
        Arc::clone(&answering),
    )?;
    let config = wire::ServeConfig {
        client: Arc::clone(&page),
        commands,
        events,
        deltas,
        logs,
        outputs,
        outputs_so_far: Arc::new(move || kept_reader.so_far()),
        monitor: watched(
            city_root,
            Gauges::new(
                health,
                move || backlog.records(),
                crate::serving::standing::monotonic_now,
            ),
        )?,
        city: city_name,
        head,
        epoch,
        secrets: Arc::new(move |command: wire::Command, reply: wire::Reply| {
            // The route waits for whichever comes first, so the
            // reply address is the credential's own request rather
            // than nowhere: a vault that refuses is a fact the
            // person typing the key needs.
            secrets_desk.post(command, reply);
            Ok(())
        }),
        queries,
        door: door.clone(),
        // An outside editor's request becomes an ordinary Dispatch on
        // the same desk a person's does. It is not a second control
        // surface: the admission decides what a stranger may learn, and
        // everything after that is the city's usual path.
        acp: Arc::new(move |body, pairing| acp_dispatch(&acp_desk, body, pairing)),
        transcribe_sink: hearing(audio_views, audio_vault),
        drop_sink: super::dropping::dropping(city_root.to_path_buf()),
    };
    // Last, so a serve refused above leaves no key file behind.
    let doorstep = doorstep::Doorstep::kept(door, at.port(), &token)?;
    Ok(Listening {
        bound,
        config,
        desk,
        answering,
        worker: worker_thread,
        outdoors: Outdoors::new(city_root, relay, CityPort { at, token, page }, remote_key),
        front,
        doorstep,
    })
}

impl Listening {
    /// The address the city's listener holds: the port the operating
    /// system gave when `serve` was asked for port 0. The banner, the
    /// console and the browser this process opens all read this one.
    #[must_use]
    pub fn local_addr(&self) -> std::net::SocketAddr {
        self.bound.local_addr()
    }

    /// Answers until the person stops the city, and returns once the
    /// writer thread has written its handoff and ended. `console` is the
    /// terminal this city runs in, when it was asked for one; it is made
    /// from [`Listening::local_addr`], so it is handed in here rather
    /// than before the port exists.
    ///
    /// # Errors
    /// Propagates the accept failures the listener reports, and a
    /// signal handler that cannot be installed.
    pub async fn serve(self, console: Option<crate::console::Terminal>) -> Result<(), AxError> {
        let Listening {
            bound,
            config,
            desk,
            answering,
            worker,
            outdoors,
            front,
            doorstep,
        } = self;
        // The terminal this city is running in, if it was asked for. It gets
        // the same desk the socket posts to and the same event stream the
        // browser reads, so nothing here is a second control surface - it is
        // the first one, reached from the keyboard that started the city.
        if let Some(terminal) = console {
            let watching = config.events.subscribe();
            let (desk, answering) = (Arc::clone(&desk), Arc::clone(&answering));
            let door = doorstep.door.clone();
            outdoors.keep_aside(move |remote| {
                front.attend(&remote, Arc::new(|line: &str| println!("{line}")));
                let inside = crate::console::Inside {
                    desk,
                    answering,
                    remote,
                    door,
                };
                crate::console::start(terminal, inside, watching);
            });
        }
        // Ctrl-C is an orderly close, so a stop somebody chose and a stop
        // that was a crash do not leave the same silence in the record.
        // The listener stops accepting first,
        // then the worker is told - it reads that where it reads its queue,
        // so whatever command is running finishes and the handoff is the
        // last line rather than a line in the middle of one.
        let mut serving = std::pin::pin!(wire::serve(bound, config));
        let served = tokio::select! {
            result = &mut serving => result,
            signal = closed_by_hand() => match signal {
                Ok(()) => Ok(()),
                // A signal handler that cannot be installed is worth saying
                // out loud, and not worth closing the city over: it keeps
                // serving, and the person now knows that Ctrl-C will be the
                // hard stop it always was.
                Err(source) => {
                    eprintln!(
                        "an orderly close cannot be listened for ({source}); the city keeps \
                         serving, and Ctrl-C ends it without a handoff"
                    );
                    serving.await
                }
            },
        };
        desk.close(Closing::of(&served));
        doorstep.close();
        // Joined rather than left to the process exit: the handoff is
        // written by that thread, and a main that returned first would end
        // the process before the line it exists to write.
        if let Err(panicked) = worker.join() {
            eprintln!("the run worker ended abnormally: {panicked:?}");
        }
        served
    }
}

mod doorstep;
mod monitor_feed;
#[cfg(test)]
mod tests;
