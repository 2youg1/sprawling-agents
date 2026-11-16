// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    dead_code,
    reason = "test code; readings are read through their Debug form in the report"
)]

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::time::Duration;

use accounting::worker::{Closing, CommandDesk, RunWorker};
use kernel::event::record::RunStarted;
use kernel::layout::CityLayout;
use kernel::{
    Address, EventKind, EventRecord, IdemKey, RunId, Seq,
};

use super::roles::{Counted, hands, idem};

/// A founded city whose main model sits behind a loopback port nothing
/// answers, so the vault's keep-warm never leaves this process.
pub(super) fn city(
    dir: &Path,
    factory: impl accounting::ModelFactory + std::marker::Send + 'static,
) -> (RunWorker, PathBuf) {
    let clock: Arc<dyn accounting::Clock + std::marker::Send + Sync> =
        Arc::new(Counted(AtomicU64::new(0)));
    accounting::worker::genesis::form(
        dir,
        accounting::worker::genesis::Adopt::Nothing,
        hands(&clock),
    )
    .unwrap();
    let mut worker = RunWorker::new(dir, runtime::diagnostics::Diagnostics::off(), hands(&clock))
        .unwrap()
        .with_models(Box::new(factory));
    let refusing = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base_url = format!("http://{}/v1", refusing.local_addr().unwrap());
    drop(refusing);
    let endpoint = wire::ProviderName::parse("dead").unwrap();
    worker
        .handle(wire::Command::AttachEndpoint {
            name: endpoint.clone(),
            base_url,
            dialect: kernel::DialectKind::OpenAi,
            secret: None,
            auth_header: None,
            admit: vec!["scripted".to_owned()],
            named: Vec::new(),
            tuning: wire::EndpointTuning {
                timeout_ms: Some(2_000),
                request_max_retries: Some(0),
                ..wire::EndpointTuning::default()
            },
            idem: idem(b"attach"),
        })
        .unwrap();
    worker
        .handle(wire::Command::SelectModel {
            endpoint,
            model: "scripted".to_owned(),
            tag: kernel::ModelTag::Main,
            context_tokens: kernel::Window::new(131_072),
            max_output_tokens: kernel::Ceiling::new(4_096),
            input: None,
            idem: idem(b"select"),
        })
        .unwrap();
    (worker, CityLayout::new(dir).ledger())
}

pub(super) fn raise(worker: &mut RunWorker, addr: &str) {
    worker
        .handle(wire::Command::CreateBuilding {
            addr: Address::parse(addr).unwrap(),
            template: wire::TemplateName::parse("minimal").unwrap(),
            rules: None,
            source: None,
            idem: idem(addr.as_bytes()),
        })
        .unwrap();
}

pub(super) fn path_of(dir: &Path, addr: &str) -> PathBuf {
    addr.split('/')
        .fold(dir.to_path_buf(), |at, part| at.join(part))
}

pub(super) fn move_in(dir: &Path, addr: &str) {
    let urbanite = CityLayout::new(dir).urbanite(&Address::parse(addr).unwrap());
    std::fs::create_dir_all(urbanite.parent().unwrap()).unwrap();
    std::fs::write(urbanite, "# URBANITE.md\n\nWorks here.\n").unwrap();
}

pub(super) fn dispatch(addr: &str, goal: &str) -> wire::Command {
    wire::Command::Dispatch {
        addr: Address::parse(addr).unwrap(),
        task: "Host the turtle soup, or delegate three rooms.".to_owned(),
        goal: goal.to_owned(),
        policy: kernel::RunPolicy::of(kernel::Mode::Work),
        idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, goal.as_bytes()),
        session: None,
        effort: None,
        model: None,
    }
}

pub(super) type Lines = Vec<(EventRecord, String)>;

/// Serves the city through `attend`, posts `commands` in order, and reads the
/// history until `done` holds, in counted looks.
pub(super) fn serve(
    worker: RunWorker,
    ledger: &Path,
    commands: Vec<wire::Command>,
    done: impl Fn(&Lines) -> bool,
) -> Lines {
    serve_in_turn(
        worker,
        ledger,
        commands
            .into_iter()
            .map(|command| {
                let now: Mark = Box::new(|_: &Lines| true);
                (now, command)
            })
            .collect(),
        done,
    )
}

/// What the history must hold before a command is posted.
pub(super) type Mark = Box<dyn Fn(&Lines) -> bool>;

/// Waits, in counted looks, until the history holds `mark`.
pub(super) fn look_until(ledger: &Path, mark: &dyn Fn(&Lines) -> bool) {
    for _ in 0..3_000 {
        if mark(&read(ledger)) {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Serves the city through `attend`, posts each command once the history
/// holds the mark beside it, and reads the history until `done` holds.
pub(super) fn serve_in_turn(
    mut worker: RunWorker,
    ledger: &Path,
    steps: Vec<(Mark, wire::Command)>,
    done: impl Fn(&Lines) -> bool,
) -> Lines {
    let desk = Arc::new(CommandDesk::default());
    let attending = {
        let desk = Arc::clone(&desk);
        std::thread::spawn(move || accounting::worker::attend::attend(&mut worker, &desk))
    };
    for (mark, command) in steps {
        look_until(ledger, &mark);
        desk.post(command, wire::Reply::nowhere());
    }
    look_until(ledger, &done);
    desk.close(Closing::Chosen {
        by: accounting::worker::ClosedBy::Console,
        mode: wire::CloseMode::Drain,
    });
    attending.join().unwrap();
    let lines = read(ledger);
    let trace: Vec<String> = lines
        .iter()
        .filter(|(record, _)| {
            matches!(
                record.kind(),
                EventKind::RunStarted
                    | EventKind::RunFrozen
                    | EventKind::ToolCalled
                    | EventKind::SignalEnqueued
                    | EventKind::ToolResult
            )
        })
        .map(|(record, raw)| {
            format!(
                "{}:{:?}@{} {}",
                record.seq().value(),
                record.kind(),
                record.addr().map_or("-", Address::as_str),
                raw.get(raw.len().saturating_sub(120)..).unwrap_or_default()
            )
        })
        .collect();
    assert!(
        done(&lines),
        "the history never reached the end the scenario waits for: {trace:?}"
    );
    lines
}

pub(super) fn read(ledger: &Path) -> Lines {
    runtime::replay::verify_ledger_dir(ledger)
        .map(|chain| {
            chain
                .raw_lines()
                .iter()
                .filter_map(|line| {
                    EventRecord::parse_line(line)
                        .ok()
                        .map(|record| (record, String::from_utf8_lossy(line).into_owned()))
                })
                .collect()
        })
        .unwrap_or_default()
}

pub(super) struct Start {
    pub(super) run: RunId,
    pub(super) seq: u64,
    pub(super) t: u64,
    pub(super) parent: Option<RunId>,
}

pub(super) fn started(lines: &Lines, room: &str) -> Vec<Start> {
    lines
        .iter()
        .filter(|(record, _)| {
            record.kind() == EventKind::RunStarted
                && record.addr().map(Address::as_str) == Some(room)
        })
        .map(|(record, _)| {
            let data: RunStarted = record.data().read().unwrap();
            Start {
                run: record.run(),
                seq: record.seq().value(),
                t: record.t().value(),
                parent: data.parent,
            }
        })
        .collect()
}

pub(super) fn frozen(lines: &Lines) -> usize {
    lines
        .iter()
        .filter(|(record, _)| record.kind() == EventKind::RunFrozen)
        .count()
}

/// Every run that started has frozen.
pub(super) fn quiet(lines: &Lines) -> bool {
    let starts = lines
        .iter()
        .filter(|(record, _)| record.kind() == EventKind::RunStarted)
        .count();
    starts > 0 && starts == frozen(lines)
}

pub(super) fn frozen_seq(lines: &Lines, run: RunId) -> u64 {
    lines
        .iter()
        .find(|(record, _)| record.kind() == EventKind::RunFrozen && record.run() == run)
        .map_or(u64::MAX, |(record, _)| record.seq().value())
}
