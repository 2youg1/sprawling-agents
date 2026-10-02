// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One episode per built-in tool: the call the script makes, and what
//! the tool's own SPEC says the answer is.
//!
//! A tool the workbench registers without an episode here is named by
//! the catalogue tests as offered and never called. The verdicts read
//! the part of a result that belongs to the tool's contract rather than
//! the whole result, so a field added for every tool later does not
//! turn them red. A verdict that fails is either an episode that
//! misreads its tool's SPEC, fixed here, or a product that disagrees
//! with its SPEC, fixed in the crate that owns the tool; what the run
//! happened to answer is never copied into a verdict.

use std::path::Path;

use kernel::event::record::ToolAnswer;
use kernel::{EventKind, RunId};
use serde_json::{Map, Value, json};

use crate::city::History;
use crate::script::Step;

/// Where one catalogue test works: the building, the room the task is
/// sent to, and a room beside it with a resident.
pub(crate) struct Setup {
    pub(crate) building: &'static str,
    pub(crate) room: &'static str,
    pub(crate) neighbour: &'static str,
}

/// A building that builds, with a tree of its own for each room.
pub(crate) const LAB: Setup = Setup {
    building: "lab",
    room: "lab/lead",
    neighbour: "lab/other",
};

/// City Hall, which plans (`crates/city/Spec.lean` §8-22).
pub(crate) const HALL: Setup = Setup {
    building: "hall",
    room: "hall/mayor",
    neighbour: "hall/clerk",
};

/// What a verdict may look at once the dispatch has landed.
pub(crate) struct Observed<'a> {
    pub(crate) setup: &'a Setup,
    pub(crate) city: &'a Path,
    pub(crate) history: &'a History,
    /// The run the script drove.
    pub(crate) lead: RunId,
    /// The building's `RULES.toml` as it stood before the dispatch.
    pub(crate) rules_before: &'a [u8],
    pub(crate) answer: &'a ToolAnswer,
}

pub(crate) struct Episode {
    pub(crate) step: Step,
    pub(crate) holds: fn(&Observed<'_>) -> Result<(), String>,
}

/// Written into the notes and looked for again, so `read` and `search`
/// are judged on bytes this run put there.
const MARK: &str = "kiln-temperature-1280";

/// The episodes for a building that builds, in the order they run.
/// `succeed` is last because the successor starts only when this run
/// ends.
pub(crate) fn for_builders(setup: &Setup) -> Vec<Episode> {
    let mut episodes = shared(setup);
    episodes.extend([
        pr_open(),
        rules(),
        exec(),
        delegate(setup),
        workshop(setup),
        succeed(),
    ]);
    episodes
}

/// The episodes for City Hall, in the order they run. It has no tree of
/// its own, so `pr` lists rather than opens; it has no `exec`,
/// `delegate` or `workshop`, and it has `city`.
pub(crate) fn for_city_hall(setup: &Setup) -> Vec<Episode> {
    let mut episodes = shared(setup);
    episodes.extend([pr_list(), rules(), city(), succeed()]);
    episodes
}

/// The episodes every building's table shares.
fn shared(setup: &Setup) -> Vec<Episode> {
    vec![
        status(),
        neighbours(),
        edit(setup),
        read(setup),
        search(),
        archive(),
        goal(setup),
        plan(),
        signal(setup),
        crate::playback::episode(),
        proposal(setup),
        describe(),
        call(),
    ]
}

/// The notes file the lead creates in its own room.
fn notes(setup: &Setup) -> String {
    format!("{}/notes.md", setup.room)
}

fn status() -> Episode {
    Episode {
        step: Step {
            tool: "status",
            args: json!({}),
        },
        holds: |seen| {
            let line = format!("addr: {}", seen.setup.room);
            let text = answered_text(seen.answer)?;
            ensure(
                text.lines().any(|at| at == line),
                format!("no `{line}` line in {text:?}"),
            )
        },
    }
}

fn neighbours() -> Episode {
    Episode {
        step: Step {
            tool: "neighbours",
            args: json!({}),
        },
        holds: |seen| {
            let entry = format!("- {}:", seen.setup.neighbour);
            let text = answered_text(seen.answer)?;
            ensure(
                text.lines().any(|at| at.starts_with(&entry)),
                format!("{} is not listed in {text:?}", seen.setup.neighbour),
            )
        },
    }
}

fn edit(setup: &Setup) -> Episode {
    Episode {
        step: Step {
            tool: "edit",
            args: json!({
                "path": notes(setup),
                "base_version": "new",
                "old": "",
                "new": format!("# Notes\n\n{MARK}\n"),
            }),
        },
        holds: |seen| {
            let result = answered(seen.answer)?;
            ensure(
                result.get("base_version") == Some(&json!("new")),
                format!("a creation answered {result:?}"),
            )?;
            ensure(
                seen.history
                    .wrote(seen.lead, EventKind::CheckpointCommitted),
                "no checkpoint was committed after the write".to_owned(),
            )
        },
    }
}

fn read(setup: &Setup) -> Episode {
    Episode {
        step: Step {
            tool: "read",
            args: json!({ "path": notes(setup) }),
        },
        holds: |seen| {
            let text = answered_text(seen.answer)?;
            ensure(
                text.contains(MARK),
                format!("the notes read back as {text:?}"),
            )
        },
    }
}

fn search() -> Episode {
    Episode {
        step: Step {
            tool: "search",
            args: json!({ "text": MARK }),
        },
        holds: |seen| {
            let result = answered(seen.answer)?;
            let at = notes(seen.setup);
            let found = result
                .get("matches")
                .and_then(Value::as_array)
                .is_some_and(|hits| hits.iter().any(|hit| hit.get("path") == Some(&json!(at))));
            ensure(found, format!("{at} is not among {result:?}"))
        },
    }
}

fn archive() -> Episode {
    Episode {
        step: Step {
            tool: "archive",
            args: json!({
                "action": "record",
                "kind": "decision",
                "text": "The kiln is fired at night.",
            }),
        },
        holds: |seen| {
            let result = answered(seen.answer)?;
            ensure(
                result.get("recorded") == Some(&json!(true))
                    && result.get("kind") == Some(&json!("decision")),
                format!("the record answered {result:?}"),
            )?;
            ensure(
                seen.history
                    .says(EventKind::AssetArchived, "The kiln is fired at night."),
                "no asset_archived line carries the decision".to_owned(),
            )
        },
    }
}

fn goal(setup: &Setup) -> Episode {
    Episode {
        step: Step {
            tool: "goal",
            args: json!({
                "statement": "keep the notes",
                "paths": [notes(setup)],
            }),
        },
        holds: |seen| {
            let result = answered(seen.answer)?;
            ensure(
                result.get("registered") == Some(&json!(true)),
                format!("the goal answered {result:?}"),
            )?;
            ensure(
                seen.history
                    .says(EventKind::GoalRegistered, "keep the notes"),
                "no goal_registered line carries the statement".to_owned(),
            )
        },
    }
}

fn plan() -> Episode {
    Episode {
        step: Step {
            tool: "plan",
            args: json!({ "action": "claim", "node": "1" }),
        },
        holds: |seen| {
            let result = answered(seen.answer)?;
            ensure(
                result.get("item") == Some(&json!("wire the kiln")),
                format!("the claim answered {result:?}"),
            )?;
            ensure(
                seen.history
                    .says(EventKind::RoadmapClaimed, "\"node\":\"1\""),
                "no roadmap_claimed line names node 1".to_owned(),
            )
        },
    }
}

fn signal(setup: &Setup) -> Episode {
    Episode {
        step: Step {
            tool: "signal",
            args: json!({
                "action": "send",
                "to": setup.neighbour,
                "text": "the kiln is free",
            }),
        },
        holds: |seen| {
            let result = answered(seen.answer)?;
            ensure(
                result.get("queued") == Some(&json!(true))
                    && result.get("to") == Some(&json!(seen.setup.neighbour)),
                format!("the send answered {result:?}"),
            )?;
            ensure(
                seen.history
                    .says(EventKind::SignalEnqueued, seen.setup.neighbour),
                format!("no signal_enqueued line names {}", seen.setup.neighbour),
            )
        },
    }
}

fn pr_open() -> Episode {
    Episode {
        step: Step {
            tool: "pr",
            args: json!({ "action": "open" }),
        },
        holds: |seen| {
            let result = answered(seen.answer)?;
            let Some(branch) = result.get("branch").and_then(Value::as_str) else {
                return Err(format!("the request names no branch: {result:?}"));
            };
            ensure(
                seen.history.says(EventKind::PrOpened, branch),
                format!("no pr_opened line names {branch}"),
            )
        },
    }
}

fn pr_list() -> Episode {
    Episode {
        step: Step {
            tool: "pr",
            args: json!({ "action": "list" }),
        },
        holds: |seen| {
            let result = answered(seen.answer)?;
            ensure(
                result.get("requests").is_some_and(Value::is_array),
                format!("the list answered {result:?}"),
            )
        },
    }
}

/// The document the lead offers a change to, laid in the city before the
/// dispatch: a proposal is about the city's copy (documents D35).
fn draft(setup: &Setup) -> String {
    format!("{}/draft.md", setup.room)
}

const DRAFT: &str = "# Draft\n\nThe kiln is fired at noon. It cools overnight.\n";

/// Where the draft lies in the city.
fn draft_path(city: &Path, setup: &Setup) -> std::path::PathBuf {
    draft(setup)
        .split('/')
        .fold(city.to_path_buf(), |at, part| at.join(part))
}

/// Lays the draft the `proposal` episode offers a change to.
pub(crate) fn lay_draft(city: &Path, setup: &Setup) {
    let path = draft_path(city, setup);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, DRAFT).unwrap();
}

/// An offer is a card the person is shown and nothing more: the
/// document keeps its bytes (`crates/accounting/Spec.lean` §8-30).
fn proposal(setup: &Setup) -> Episode {
    Episode {
        step: Step {
            tool: "proposal",
            args: json!({
                "action": "offer",
                "path": draft(setup),
                "old": "It cools overnight.",
                "new": "It cools by dawn.",
            }),
        },
        holds: |seen| {
            let result = answered(seen.answer)?;
            let doc = address(&draft(seen.setup));
            let open = match accounting::views::ask(seen.city, &wire::Query::Proposals(doc)) {
                Ok(wire::Answer::Proposals(cards)) => cards.open,
                other => return Err(format!("the cards answered {other:?}")),
            };
            ensure(
                open.iter().any(|card| {
                    result.get("proposal") == Some(&json!(card.id.to_string()))
                        && card.run == seen.lead
                }),
                format!("the offer answered {result:?} and the open cards are {open:?}"),
            )?;
            let now =
                std::fs::read_to_string(draft_path(seen.city, seen.setup)).unwrap_or_default();
            ensure(
                now == DRAFT,
                format!("the draft changed under an offer: {now:?}"),
            )
        },
    }
}

/// Refused at the effect layer: a run does not change what governs it
/// (`crates/city/Spec.lean` §8-2b). Judged by the file staying as it was.
fn rules() -> Episode {
    Episode {
        step: Step {
            tool: "rules",
            args: json!({ "op": "propose", "text": "confidential = true\nwrite = \"everything\"\n" }),
        },
        holds: |seen| {
            failed(seen.answer)?;
            let path = ::city::rules_path(seen.city, &address(seen.setup.building));
            let now = std::fs::read(&path).unwrap_or_default();
            ensure(
                now == seen.rules_before,
                format!("{} changed under a refused proposal", path.display()),
            )
        },
    }
}

/// A shell is off unless a building turns it on, so the shell arm has
/// nothing to run (`crates/runtime/Spec.lean`, `exec`).
fn exec() -> Episode {
    Episode {
        step: Step {
            tool: "exec",
            args: json!({ "arm": { "shell": { "text": "echo hi" } } }),
        },
        holds: |seen| {
            let error = failed(seen.answer)?;
            ensure(
                error.get("code") == Some(&json!("E_TOOL_UNAVAILABLE")),
                format!("the shell arm answered {error:?}"),
            )
        },
    }
}

fn delegate(setup: &Setup) -> Episode {
    Episode {
        step: Step {
            tool: "delegate",
            args: json!({
                "room": format!("{}/helper", setup.building),
                "task": "measure the kiln",
                "goal": "a number, then stop",
            }),
        },
        holds: |seen| {
            answered(seen.answer)?;
            started_under(seen, &format!("{}/helper", seen.setup.building))
        },
    }
}

fn workshop(setup: &Setup) -> Episode {
    Episode {
        step: Step {
            tool: "workshop",
            args: json!({
                "op": "lay_out",
                "nodes": [{
                    "room": format!("{}/reader", setup.building),
                    "goal": "read the notes",
                    "done_check": "the notes were read",
                    "stop": "when the notes were read",
                    "depends_on": [],
                }],
            }),
        },
        holds: |seen| {
            answered(seen.answer)?;
            started_under(seen, &format!("{}/reader", seen.setup.building))
        },
    }
}

fn succeed() -> Episode {
    Episode {
        step: Step {
            tool: "succeed",
            args: json!({ "reason": "the window is full" }),
        },
        holds: |seen| {
            answered(seen.answer)?;
            ensure(
                seen.history
                    .started()
                    .iter()
                    .any(|started| started.record.predecessor == Some(seen.lead)),
                "no run started as this run's successor".to_owned(),
            )
        },
    }
}

/// The guide of a dormant tool: its whole disclosure, its schema, and
/// the door it is run through (`crates/runtime/Spec.lean` §8-60).
fn describe() -> Episode {
    Episode {
        step: Step {
            tool: "describe",
            args: json!({ "name": "archive" }),
        },
        holds: |seen| {
            let text = answered_text(seen.answer)?;
            ensure(
                text.starts_with("tool archive: run it with `call`")
                    && text.contains("input schema:"),
                format!("the guide of a dormant tool read {text:?}"),
            )
        },
    }
}

/// A call through `call` that stands for nothing reaches `call` itself
/// and is refused by the catalog's own sentence (§8-61); every builder
/// call above that named a dormant tool directly passed that tool's doors.
fn call() -> Episode {
    Episode {
        step: Step {
            tool: "call",
            args: json!({ "name": "call", "args": {} }),
        },
        holds: |seen| {
            let error = failed(seen.answer)?;
            ensure(
                error.get("subject") == Some(&json!("`call` cannot run itself")),
                format!("a call through `call` naming itself answered {error:?}"),
            )
        },
    }
}

/// Refused at the effect layer: a run does not raise a building
/// (`crates/city/Spec.lean` §8-23). Judged by the building not existing.
fn city() -> Episode {
    Episode {
        step: Step {
            tool: "city",
            args: json!({ "action": "raise", "name": "kiln" }),
        },
        holds: |seen| {
            failed(seen.answer)?;
            ensure(
                !seen.city.join("kiln").exists(),
                "a refused raise left a building behind".to_owned(),
            )
        },
    }
}

/// Whether a run started at `room` with the lead as its parent.
fn started_under(seen: &Observed<'_>, room: &str) -> Result<(), String> {
    ensure(
        seen.history.started().iter().any(|started| {
            started.record.parent == Some(seen.lead)
                && started.addr.as_ref().map(kernel::Address::as_str) == Some(room)
        }),
        format!("no run started at {room} under the lead"),
    )
}

fn address(raw: &str) -> kernel::Address {
    kernel::Address::parse(raw).unwrap()
}

fn ensure(holds: bool, otherwise: String) -> Result<(), String> {
    if holds { Ok(()) } else { Err(otherwise) }
}

/// The result of a call that answered.
fn answered(answer: &ToolAnswer) -> Result<&Map<String, Value>, String> {
    match answer {
        ToolAnswer::Answered { result } => Ok(result.as_map()),
        ToolAnswer::Failed { error } => Err(format!("failed: {:?}", error.as_map())),
    }
}

/// The error of a call that failed.
fn failed(answer: &ToolAnswer) -> Result<&Map<String, Value>, String> {
    match answer {
        ToolAnswer::Failed { error } => Ok(error.as_map()),
        ToolAnswer::Answered { result } => Err(format!("answered: {:?}", result.as_map())),
    }
}

/// The `text` field of a call that answered.
pub(crate) fn answered_text(answer: &ToolAnswer) -> Result<&str, String> {
    answered(answer)?
        .get("text")
        .and_then(Value::as_str)
        .ok_or_else(|| "the result carries no text".to_owned())
}

// The transcription tool (sprawling-SPEC.md 8-131). It is offered only to
// a city that chose an endpoint to transcribe, and the two catalogue
// tests choose none, so the tool is covered here by cities of its own.

/// What the scripted transcription endpoint hears in every recording.
const HEARD: &str = "fire the kiln at dawn";

/// The model the person chose to transcribe.
const EARS: &str = "whisper-1";

/// Rules for the lab that leave it open and writing where it works.
pub(crate) const OPEN_LAB: &str = "confidential = false\nwrite = \"everything\"\n";

#[test]
fn a_run_hears_a_recording_through_the_endpoint_chosen_to_transcribe() {
    let dir = tempfile::tempdir().unwrap();
    let (url, served) = transcription_endpoint();
    let (factory, _, _) = crate::script::scripted(vec![Step {
        tool: "transcribe",
        args: json!({ "path": format!("{}/voice.wav", LAB.room) }),
    }]);
    let (mut worker, ledger) = crate::city::city_with_a_model(dir.path(), factory);
    choose(&mut worker, &url, kernel::ModelTag::Transcribe, EARS);
    crate::city::raise(&mut worker, LAB.building, "minimal");
    crate::city::rules(dir.path(), LAB.building, OPEN_LAB);
    crate::city::move_in(dir.path(), LAB.room);
    std::fs::write(
        dir.path().join("lab").join("lead").join("voice.wav"),
        b"RIFF-a-spoken-sentence",
    )
    .unwrap();

    let dispatched = crate::city::dispatch(&mut worker, LAB.room);

    let history = History::read(&ledger);
    let lead = history.started().first().map(|started| started.run);
    let calls = lead.map(|run| history.calls_of(run)).unwrap_or_default();
    let heard = calls
        .iter()
        .find(|call| call.called.name.as_str() == "transcribe")
        .and_then(|call| call.results.first())
        .map(|result| answered_text(&result.answer));
    assert_eq!(
        heard,
        Some(Ok(HEARD)),
        "the dispatch answered {dispatched:?}"
    );
    let request = served.join().unwrap();
    assert!(
        request.starts_with("POST /v1/audio/transcriptions"),
        "{request}"
    );
    assert!(
        request.contains(&format!("name=\"model\"\r\n\r\n{EARS}")),
        "the endpoint was not asked for the model the person chose: {request}"
    );
    assert!(request.contains("RIFF-a-spoken-sentence"), "{request}");
}

/// The same question the composer's microphone asks, under the rules of
/// the building the run stands in: a confidential building is never
/// offered an endpoint off this machine.
#[test]
fn a_confidential_building_is_not_offered_a_transcription_endpoint_off_this_machine() {
    let dir = tempfile::tempdir().unwrap();
    let (factory, offered, _) = crate::script::scripted(Vec::new());
    let (mut worker, _) = crate::city::city_with_a_model(dir.path(), factory);
    // A name that never resolves (RFC 2606): not this machine, and the
    // model list attaching it asks for fails at once.
    choose(
        &mut worker,
        "http://transcribe.invalid/v1",
        kernel::ModelTag::Transcribe,
        EARS,
    );
    crate::city::raise(&mut worker, LAB.building, "minimal");
    crate::city::rules(
        dir.path(),
        LAB.building,
        "confidential = true\nwrite = \"everything\"\n",
    );
    crate::city::move_in(dir.path(), LAB.room);

    let dispatched = crate::city::dispatch(&mut worker, LAB.room);

    let offered = offered.lock().unwrap().clone();
    assert!(
        !offered.is_empty(),
        "the run was offered nothing: {dispatched:?}"
    );
    assert!(
        offered.iter().all(|name| name != "transcribe"),
        "{offered:?}"
    );
}

/// A recording a connector stored: a wav block put into the city's
/// content store for the lab, the way `runtime::pipeline::connector`
/// stores one, named by its locator.
const STORED: &[u8] = b"RIFF\x24\x00\x00\x00WAVEfmt a stored recording";

/// The transcription tool reads a block a connector stored, by the
/// locator the window showed for it (sprawling-SPEC.md 8-131): no name,
/// so the container is read from its leading bytes.
#[test]
fn a_run_hears_a_recording_a_connector_stored_by_its_locator() {
    let dir = tempfile::tempdir().unwrap();
    let (url, served) = transcription_endpoint();
    let locator = format!("cas:b3-{}", kernel::B3Hash::digest(STORED));
    let (factory, _, _) = crate::script::scripted(vec![Step {
        tool: "transcribe",
        args: json!({ "path": locator }),
    }]);
    let (mut worker, ledger) = crate::city::city_with_a_model(dir.path(), factory);
    choose(&mut worker, &url, kernel::ModelTag::Transcribe, EARS);
    crate::city::raise(&mut worker, LAB.building, "minimal");
    crate::city::rules(dir.path(), LAB.building, OPEN_LAB);
    crate::city::move_in(dir.path(), LAB.room);
    stored_for(dir.path(), LAB.building, STORED);

    let dispatched = crate::city::dispatch(&mut worker, LAB.room);

    let history = History::read(&ledger);
    let lead = history.started().first().map(|started| started.run);
    let calls = lead.map(|run| history.calls_of(run)).unwrap_or_default();
    let heard = calls
        .iter()
        .find(|call| call.called.name.as_str() == "transcribe")
        .and_then(|call| call.results.first())
        .map(|result| answered_text(&result.answer));
    assert_eq!(
        heard,
        Some(Ok(HEARD)),
        "the dispatch answered {dispatched:?}"
    );
    let request = served.join().unwrap();
    assert!(
        request.contains("filename=\"recording.wav\"") && request.contains("a stored recording"),
        "{request}"
    );
}

/// Puts `bytes` into the city's content store for `building`, as a
/// connector stores what a run's tool answered with, and names the run
/// the city's own: who stored a block does not decide who may read it.
pub(crate) fn stored_for(city: &Path, building: &str, bytes: &[u8]) -> kernel::B3Hash {
    storage::Cas::open(&kernel::layout::CityLayout::new(city).cas())
        .unwrap()
        .put_for(
            bytes,
            &storage::BlockOrigin {
                run: RunId::CITY,
                building: address(building),
            },
        )
        .unwrap()
}

/// Attaches an endpoint at `base_url` and chooses its one model, `model`,
/// for `tag`, as a person does on the settings page.
pub(crate) fn choose(
    worker: &mut accounting::worker::RunWorker,
    base_url: &str,
    tag: kernel::ModelTag,
    model: &str,
) {
    let endpoint = wire::ProviderName::parse(tag.as_str()).unwrap();
    let idem = |what: &[u8]| kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, what);
    worker
        .handle(wire::Command::AttachEndpoint {
            name: endpoint.clone(),
            base_url: base_url.to_owned(),
            dialect: kernel::DialectKind::OpenAi,
            secret: None,
            auth_header: None,
            admit: vec![model.to_owned()],
            // One brief try, so an endpoint that is not there fails the
            // model list at once rather than retrying into it.
            tuning: wire::EndpointTuning {
                timeout_ms: Some(2_000),
                request_max_retries: Some(0),
                ..wire::EndpointTuning::default()
            },
            idem: idem(format!("attach {tag}").as_bytes()),
        })
        .unwrap();
    worker
        .handle(wire::Command::SelectModel {
            endpoint,
            model: model.to_owned(),
            tag,
            context_tokens: kernel::Window::new(131_072),
            max_output_tokens: kernel::Ceiling::new(4_096),
            input: None,
            idem: idem(format!("select {tag}").as_bytes()),
        })
        .unwrap();
}

/// A transcription endpoint on loopback that answers a transcription with
/// [`HEARD`] and hands back the request that asked for it. Whatever it is
/// asked before that, the model list attaching it reads, names [`EARS`].
fn transcription_endpoint() -> (String, std::thread::JoinHandle<String>) {
    use std::io::Write as _;
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/v1", listener.local_addr().unwrap());
    let served = std::thread::spawn(move || {
        loop {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_request(&mut stream);
            let transcription = request.starts_with("POST /v1/audio/transcriptions");
            let body = if transcription {
                json!({ "text": HEARD })
            } else {
                json!({ "data": [{ "id": EARS }] })
            }
            .to_string();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\
                 connection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
            if transcription {
                return request;
            }
        }
    });
    (url, served)
}

/// One HTTP request: its head, then as many body bytes as it declared.
pub(crate) fn read_request(stream: &mut std::net::TcpStream) -> String {
    use std::io::Read as _;
    let mut seen = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let read = stream.read(&mut chunk).unwrap();
        if read == 0 {
            break;
        }
        seen.extend_from_slice(&chunk[..read]);
        let text = String::from_utf8_lossy(&seen);
        let Some((head, _)) = text.split_once("\r\n\r\n") else {
            continue;
        };
        let declared: usize = head
            .lines()
            .find_map(|line| {
                line.to_ascii_lowercase()
                    .strip_prefix("content-length:")
                    .map(|length| length.trim().parse().unwrap())
            })
            .unwrap_or(0);
        // The head is ASCII, so its length is the same count of bytes in
        // what was read as in its lossy reading.
        if seen.len() >= head.len().saturating_add(4).saturating_add(declared) {
            break;
        }
    }
    String::from_utf8_lossy(&seen).into_owned()
}
