// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use super::{LAB, answered_text};
use crate::script::Step;

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
