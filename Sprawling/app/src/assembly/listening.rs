// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::Arc;

use kernel::{AxCode, AxError};

use super::attending::{Opening, Outward, Started, spawn_worker};
use super::remote_door::{CityPort, Outdoors};
use crate::console::lifecycle::{Event, Face};
use crate::doctor::host;
use crate::monitor::sampler::Gauges;
use crate::outside::keeper::CityKey;
use crate::serving::Serving;
use crate::serving::output_ring::OutputRing;
use crate::serving::standing::monotonic_now;
use accounting::offered::Offered;
use accounting::views::Published;
use accounting::worker::opening_cost::{OpeningCost, Phase};
use accounting::worker::{CommandDesk, acp_dispatch, start_served_views};
use lifetime::{EVENTS, Living, closing_from_a_page, live};
use monitor_feed::watched;

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
    lifecycle: (
        tokio::sync::mpsc::Sender<Event>,
        tokio::sync::mpsc::Receiver<Event>,
    ),
}

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
    let mut cost = OpeningCost::begin(monotonic_now);
    let bound = wire::bind(addr, Some(key)).await?;
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

    let events = tokio::sync::broadcast::Sender::new(1024);
    let deltas = tokio::sync::broadcast::Sender::new(256);
    let outputs = tokio::sync::broadcast::Sender::new(256);
    let kept = Arc::new(OutputRing::default());
    let kept_reader = Arc::clone(&kept);
    let logs = journal.lines();
    let (mut rebuilt, held) = start_served_views(
        &kernel::layout::CityLayout::new(city_root).ledger(),
        accounting::Clock::now(&super::SystemClock)?,
        &mut log,
        &mut cost,
    )?;
    let (spare, offered, halt) = handed(&mut rebuilt)?;
    cost.lap(Phase::Twin);
    let views = Arc::new(Published::new(rebuilt));
    let privacy = super::privacy::page();
    let answering = super::privacy::answering(Arc::clone(&views), privacy.clone());
    let started_from = views.snapshot();
    let city_name = started_from.city();
    let epoch = started_from.epoch();
    let started_at = started_from.head();
    journal.hears_the_process(&log, started_at.unwrap_or(kernel::Seq::FIRST));
    let head = Arc::new(wire::LedgerHead::at(started_at));
    drop(started_from);
    let desk = Arc::new(CommandDesk::default());
    let secrets_desk = Arc::clone(&desk);
    let acp_desk = Arc::clone(&desk);
    let mut opening_log = journal.beside(&log);
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
    let lifecycle = tokio::sync::mpsc::channel(EVENTS);
    let (door, commands, queries) = doorstep::wired(
        city_root,
        consenting::taking(
            city_root.to_path_buf(),
            offered,
            closing_from_a_page(
                at,
                lifecycle.0.clone(),
                super::privacy::commands(&front, Arc::clone(&desk), privacy),
            ),
        ),
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
            secrets_desk.post(command, reply);
            Ok(())
        }),
        queries,
        door: door.clone(),
        acp: Arc::new(move |body, pairing| acp_dispatch(&acp_desk, body, pairing)),
        transcribe_sink: hearing(audio_views, audio_vault),
        drop_sink: super::dropping::dropping(city_root.to_path_buf()),
        workspace: Some(super::workspace::workspace(city_root.to_path_buf())),
    };
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
        lifecycle,
    })
}

fn handed(
    rebuilt: &mut accounting::views::Views,
) -> Result<(accounting::views::Views, Offered, storage::ChainHalt), AxError> {
    rebuilt.ask_the_registry_through(crate::release::answer);
    rebuilt.ask_upstream_through(crate::doctor::newest);
    rebuilt.look_for_agents_through(host::place_set_up);
    let offered = Offered::default();
    rebuilt.remember_offers_in(offered.clone());
    rebuilt.ask_github_through(crate::doctor::github::login);
    let halt = storage::ChainHalt::awaiting_proof();
    rebuilt.watch_proof(halt.clone());
    Ok((rebuilt.twin()?, offered, halt))
}

impl Listening {
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
            lifecycle: events,
        } = self;
        let _served = crate::browser_tool::serve(bound.local_addr());
        let (faces, face) = tokio::sync::watch::channel(Face::Opening);
        let surface = console
            .as_ref()
            .map_or(crate::console::Surface::Headless, |terminal| {
                terminal.surface
            });
        let writer = Arc::new(std::sync::Mutex::new(None));
        if let Some(terminal) = console {
            let watching = config.events.subscribe();
            let (desk, answering) = (Arc::clone(&desk), Arc::clone(&answering));
            let (lifecycle, kept) = (events.0.clone(), Arc::clone(&writer));
            let door = doorstep.door.clone();
            outdoors.keep_aside(move |remote| {
                let carry_remote: crate::console::CarryRemote = {
                    let remote = remote.clone();
                    std::sync::Arc::new(move |tail: &str| {
                        crate::outside::console::carry(
                            &remote,
                            crate::outside::console::parse(tail),
                        )
                    })
                };
                let open_paired: crate::console::OpenPaired =
                    std::sync::Arc::new(crate::firstrun::open_paired);
                let inside = crate::console::Inside {
                    desk,
                    answering,
                    carry_remote,
                    open_paired,
                    door,
                    lifecycle,
                    clock: Arc::new(|| accounting::Clock::now(&super::SystemClock)),
                };
                let started = crate::console::start(terminal, inside, watching, face);
                let notice = started.notice;
                front.attend(&remote, Arc::new(move |line: &str| notice(line.to_owned())));
                if let Ok(mut kept) = kept.lock() {
                    *kept = started.writer;
                }
            });
        }
        let (_keep_open, events) = events;
        let lived = live(
            wire::serve(bound, config),
            events,
            Living {
                desk,
                worker,
                faces,
                surface,
            },
        )
        .await;
        doorstep.close();
        let writer = writer.lock().ok().and_then(|mut kept| kept.take());
        if let Some(writer) = writer {
            drop(tokio::task::spawn_blocking(move || writer.join()).await);
        }
        lived
    }
}

mod consenting;
mod doorstep;
mod lifetime;
mod monitor_feed;
#[cfg(test)]
mod tests;
