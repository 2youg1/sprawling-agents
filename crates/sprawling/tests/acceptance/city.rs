// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The city an acceptance test stands up, the commands it sends, and
//! the history it reads back.

use std::path::{Path, PathBuf};

use kernel::event::record::{RunStarted, ToolCalled, ToolResult};
use kernel::{Address, AxError, EventKind, EventRecord, IdemKey, RunId, Seq};
use sprawling::assembly;

use crate::script::Scripted;

/// The one model the city is given. Every model the worker builds is
/// this one, so the factory never has to tell a digest call apart.
const MODEL: &str = "scripted";

/// Builds the worker the way the product does, handed `factory`.
///
/// The one place in this binary that constructs a worker, so a change
/// to the constructor changes this function and nothing else.
pub(crate) fn open_worker(
    dir: &Path,
    models: Box<dyn accounting::ModelFactory + Send>,
) -> accounting::worker::RunWorker {
    accounting::worker::RunWorker::new(
        dir,
        runtime::diagnostics::Diagnostics::off(),
        assembly::hands(gateway::Custodian::in_memory()),
    )
    .unwrap()
    .with_models(models)
}

/// A founded city whose main model sits on an endpoint that refuses
/// every connection, so only the factory can answer; with the directory
/// its ledger is in.
pub(crate) fn city_with_a_model(
    dir: &Path,
    factory: Scripted,
) -> (accounting::worker::RunWorker, PathBuf) {
    city_with_models(dir, Box::new(factory))
}

/// [`city_with_a_model`] for any factory, such as one that hands each
/// run it builds a model of its own.
pub(crate) fn city_with_models(
    dir: &Path,
    models: Box<dyn accounting::ModelFactory + Send>,
) -> (accounting::worker::RunWorker, PathBuf) {
    let founded = assembly::init_city(dir).unwrap();
    let mut worker = open_worker(dir, models);
    let endpoint = wire::ProviderName::parse("dead").unwrap();
    worker
        .handle(wire::Command::AttachEndpoint {
            name: endpoint.clone(),
            base_url: refusing_url(),
            dialect: kernel::DialectKind::OpenAi,
            secret: None,
            auth_header: None,
            admit: vec![MODEL.to_owned()],
            // One try, briefly: a worker that called the dead port
            // should fail at once rather than retry into it.
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
            model: MODEL.to_owned(),
            tag: kernel::ModelTag::Main,
            context_tokens: kernel::Window::new(131_072),
            max_output_tokens: kernel::Ceiling::new(4_096),
            input: None,
            idem: idem(b"select"),
        })
        .unwrap();
    (worker, founded.ledger_dir)
}

/// Raises a building at `addr` from `template`.
pub(crate) fn raise(worker: &mut accounting::worker::RunWorker, addr: &str, template: &str) {
    worker
        .handle(wire::Command::CreateBuilding {
            addr: Address::parse(addr).unwrap(),
            template: wire::TemplateName::parse(template).unwrap(),
            idem: idem(addr.as_bytes()),
        })
        .unwrap();
}

/// Writes the building's own `RULES.toml`, as a person would.
pub(crate) fn rules(dir: &Path, building: &str, text: &str) {
    city::write_rules(dir, &Address::parse(building).unwrap(), text).unwrap();
}

/// Gives the room at `addr` a resident.
pub(crate) fn move_in(dir: &Path, addr: &str) {
    let room = addr
        .split('/')
        .fold(dir.to_path_buf(), |at, part| at.join(part));
    std::fs::create_dir_all(&room).unwrap();
    std::fs::write(
        room.join(city::URBANITE_FILE),
        "# URBANITE.md\n\nWorks here.\n",
    )
    .unwrap();
}

/// Sends one task to the room at `addr`.
pub(crate) fn dispatch(
    worker: &mut accounting::worker::RunWorker,
    addr: &str,
) -> Result<(), AxError> {
    worker.handle(wire::Command::Dispatch {
        addr: Address::parse(addr).unwrap(),
        task: "Use every tool you were given once.".to_owned(),
        goal: "each tool has answered".to_owned(),
        policy: kernel::RunPolicy::of(kernel::Mode::Work),
        idem: idem(b"dispatch"),
        session: None,
        effort: None,
        model: None,
    })
}

/// A loopback address nothing listens on: bound once for a free port,
/// then released.
fn refusing_url() -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    format!("http://127.0.0.1:{port}/v1")
}

fn idem(what: &[u8]) -> IdemKey {
    IdemKey::derive(&RunId::CITY, Seq::FIRST, what)
}

/// The city's history as it stands, oldest line first.
pub(crate) struct History(Vec<(EventRecord, String)>);

/// One call a run made, with every line that answered it.
pub(crate) struct Call {
    pub(crate) called: ToolCalled,
    pub(crate) results: Vec<ToolResult>,
}

/// A run the city started, with where it started.
pub(crate) struct Started {
    pub(crate) run: RunId,
    pub(crate) addr: Option<Address>,
    pub(crate) record: RunStarted,
}

impl History {
    pub(crate) fn read(ledger_dir: &Path) -> History {
        History(
            runtime::replay::verify_ledger_dir(ledger_dir)
                .unwrap()
                .raw_lines()
                .iter()
                .map(|line| {
                    (
                        EventRecord::parse_line(line).unwrap(),
                        String::from_utf8_lossy(line).into_owned(),
                    )
                })
                .collect(),
        )
    }

    /// Every run the city started, in the order it started them.
    pub(crate) fn started(&self) -> Vec<Started> {
        self.0
            .iter()
            .filter(|(record, _)| record.kind() == EventKind::RunStarted)
            .map(|(record, _)| Started {
                run: record.run(),
                addr: record.addr().cloned(),
                record: record.data().read().unwrap(),
            })
            .collect()
    }

    /// Every call `run` made, each with the results filed under its id.
    pub(crate) fn calls_of(&self, run: RunId) -> Vec<Call> {
        let mut calls: Vec<Call> = self
            .0
            .iter()
            .filter(|(record, _)| record.kind() == EventKind::ToolCalled && record.run() == run)
            .map(|(record, _)| Call {
                called: record.data().read().unwrap(),
                results: Vec::new(),
            })
            .collect();
        for (record, _) in &self.0 {
            if record.kind() != EventKind::ToolResult || record.run() != run {
                continue;
            }
            let result: ToolResult = record.data().read().unwrap();
            if let Some(call) = calls
                .iter_mut()
                .find(|call| call.called.id == result.tool_use_id)
            {
                call.results.push(result);
            }
        }
        calls
    }

    /// Whether a line of `kind` carries `text` anywhere in its bytes.
    pub(crate) fn says(&self, kind: EventKind, text: &str) -> bool {
        self.0
            .iter()
            .any(|(record, raw)| record.kind() == kind && raw.contains(text))
    }

    /// Whether `run` wrote a line of `kind`.
    pub(crate) fn wrote(&self, run: RunId, kind: EventKind) -> bool {
        self.0
            .iter()
            .any(|(record, _)| record.kind() == kind && record.run() == run)
    }
}
