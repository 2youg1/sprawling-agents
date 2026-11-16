// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::event::record::ToolAnswer;
use kernel::{EventKind, RunId};
use serde_json::{Map, Value, json};

use crate::city::History;
use crate::script::Step;

pub(crate) struct Setup {
    pub(crate) building: &'static str,
    pub(crate) room: &'static str,
    pub(crate) neighbour: &'static str,
}

pub(crate) const LAB: Setup = Setup {
    building: "lab",
    room: "lab/lead",
    neighbour: "lab/other",
};

pub(crate) const HALL: Setup = Setup {
    building: "hall",
    room: "hall/mayor",
    neighbour: "hall/clerk",
};

pub(crate) struct Observed<'a> {
    pub(crate) setup: &'a Setup,
    pub(crate) city: &'a Path,
    pub(crate) history: &'a History,

    pub(crate) lead: RunId,

    pub(crate) rules_before: &'a [u8],
    pub(crate) answer: &'a ToolAnswer,
}

pub(crate) struct Episode {
    pub(crate) step: Step,
    pub(crate) holds: fn(&Observed<'_>) -> Result<(), String>,
}

const MARK: &str = "kiln-temperature-1280";

pub(crate) fn for_builders(setup: &Setup) -> Vec<Episode> {
    let mut episodes = shared(setup);
    episodes.extend([
        pr_open(),
        rules(),
        exec(),
        delegate(setup),
        workshop(setup),
        crate::web_search::episode(),
        succeed(),
    ]);
    episodes
}

pub(crate) fn for_city_hall(setup: &Setup) -> Vec<Episode> {
    let mut episodes = shared(setup);
    episodes.extend([
        pr_list(),
        rules(),
        city(),
        crate::web_search::episode(),
        succeed(),
    ]);
    episodes
}

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
        detail(),
        run_itself(),
        list("sprawling"),
        list("record"),
        list("web"),
    ]
}

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
                "file_path": notes(setup),
                "old_string": "",
                "new_string": format!("# Notes\n\n{MARK}\n"),
            }),
        },
        holds: |seen| {
            let result = answered(seen.answer)?;
            ensure(
                result.get("diff").is_some(),
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
            args: json!({ "file_path": notes(setup) }),
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
            args: json!({ "pattern": MARK, "output_mode": "content" }),
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
                result.get("delivered") == Some(&json!(true))
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

fn draft(setup: &Setup) -> String {
    format!("{}/draft.md", setup.room)
}

const DRAFT: &str = "# Draft\n\nThe kiln is fired at noon. It cools overnight.\n";

fn draft_path(city: &Path, setup: &Setup) -> std::path::PathBuf {
    draft(setup)
        .split('/')
        .fold(city.to_path_buf(), |at, part| at.join(part))
}

pub(crate) fn lay_draft(city: &Path, setup: &Setup) {
    let path = draft_path(city, setup);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, DRAFT).unwrap();
}

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

fn exec() -> Episode {
    Episode {
        step: Step {
            tool: "exec",
            args: json!({ "command": "echo hi > hi.txt" }),
        },
        holds: |seen| {
            let error = failed(seen.answer)?;
            ensure(
                error.get("code") == Some(&json!("E_INVALID_ARGS"))
                    && error.get("subject") == Some(&json!("`>` outside quotes")),
                format!("the command answered {error:?}"),
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

fn detail() -> Episode {
    Episode {
        step: Step {
            tool: "tool",
            args: json!({ "op": "detail", "name": "archive" }),
        },
        holds: |seen| {
            let text = answered_text(seen.answer)?;
            ensure(
                text.starts_with("tool archive (sprawling): run it with `tool`")
                    && text.contains("input schema:"),
                format!("the guide of a dormant tool read {text:?}"),
            )
        },
    }
}

fn list(namespace: &'static str) -> Episode {
    Episode {
        step: Step {
            tool: "tool",
            args: json!({ "op": "list", "namespace": namespace }),
        },
        holds: |seen| {
            let text = answered_text(seen.answer)?;
            ensure(
                text.lines().all(|line| line.starts_with("- ")),
                format!("a namespace listed {text:?}"),
            )
        },
    }
}

fn run_itself() -> Episode {
    Episode {
        step: Step {
            tool: "tool",
            args: json!({ "op": "run", "name": "tool", "args": {} }),
        },
        holds: |seen| {
            let error = failed(seen.answer)?;
            ensure(
                error.get("subject") == Some(&json!("`tool` cannot run itself")),
                format!("a `tool` `run` naming itself answered {error:?}"),
            )
        },
    }
}

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

fn answered(answer: &ToolAnswer) -> Result<&Map<String, Value>, String> {
    match answer {
        ToolAnswer::Answered { result } => Ok(result.as_map()),
        ToolAnswer::Failed { error } => Err(format!("failed: {:?}", error.as_map())),
    }
}

fn failed(answer: &ToolAnswer) -> Result<&Map<String, Value>, String> {
    match answer {
        ToolAnswer::Failed { error } => Ok(error.as_map()),
        ToolAnswer::Answered { result } => Err(format!("answered: {:?}", result.as_map())),
    }
}

pub(crate) fn answered_text(answer: &ToolAnswer) -> Result<&str, String> {
    answered(answer)?
        .get("text")
        .and_then(Value::as_str)
        .ok_or_else(|| "the result carries no text".to_owned())
}

const HEARD: &str = "fire the kiln at dawn";

const EARS: &str = "whisper-1";

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

#[test]
fn a_confidential_building_is_not_offered_a_transcription_endpoint_off_this_machine() {
    let dir = tempfile::tempdir().unwrap();
    let (factory, offered, _) = crate::script::scripted(Vec::new());
    let (mut worker, _) = crate::city::city_with_a_model(dir.path(), factory);

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

const STORED: &[u8] = b"RIFF\x24\x00\x00\x00WAVEfmt a stored recording";

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
            named: Vec::new(),

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

        if seen.len() >= head.len().saturating_add(4).saturating_add(declared) {
            break;
        }
    }
    String::from_utf8_lossy(&seen).into_owned()
}
