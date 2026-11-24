// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use std::path::Path;

use accounting::worker::RunWorker;
use kernel::{Address, AxCode, AxError, EventKind, EventRecord, IdemKey, RunId, Seq};

use accounting::worker::genesis::{Adopt, form};
use sprawling::assembly;

const BASE_URL: &str = "SPRAWLING_E2E_BASE_URL";
const KEY: &str = "SPRAWLING_E2E_KEY";
const MODEL: &str = "SPRAWLING_E2E_MODEL";
const DIALECT: &str = "SPRAWLING_E2E_DIALECT";

const REQUIRED: &str = "SPRAWLING_E2E_REQUIRED";

const SECRET_LOCATOR: &str = "secret:e2e/key";

const ENDPOINT: &str = "e2e";
const BUILDING: &str = "lab";
const SESSION: &str = "gate";

const WRONG_KEY: &str = "sk-e2e-deliberately-wrong-key";

const REQUEST_TIMEOUT_MS: u64 = 60_000;
const NO_RETRY: u32 = 0;

struct Round {
    label: String,
    base_url: String,
    key: String,
    model: String,
    dialect: kernel::DialectKind,
}

fn rounds() -> Vec<Round> {
    let mut rounds = Vec::new();
    let mut absent = Vec::new();
    for known in gateway::known_hosts().unwrap() {
        let (key_name, model_name) = host_variables(known.host);
        match (read(&key_name), read(&model_name)) {
            (Some(key), Some(model)) => {
                rounds.extend(known.faces.iter().map(|(dialect, base_url)| Round {
                    label: format!("{} {dialect:?}", known.host),
                    base_url: base_url.clone(),
                    key: key.clone(),
                    model: model.clone(),
                    dialect: *dialect,
                }));
            }
            (None, None) => absent.push(known.host),
            (Some(_), None) | (None, Some(_)) => {
                broken(&format!(
                    "{} needs both {key_name} and {model_name}",
                    known.host
                ));
            }
        }
    }
    if !absent.is_empty() {
        println!(
            "skipped the known hosts with no key exported: {}; each takes {KEY}_<HOST> and \
             {MODEL}_<HOST>, as in {}",
            absent.join(", "),
            host_variables("api.deepseek.com").0
        );
    }
    rounds.extend(pasted());
    rounds
}

fn host_variables(host: &str) -> (String, String) {
    let spelled: String = host
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect();
    (format!("{KEY}_{spelled}"), format!("{MODEL}_{spelled}"))
}

fn pasted() -> Option<Round> {
    let (Some(base_url), Some(key), Some(model), Some(raw)) =
        (read(BASE_URL), read(KEY), read(MODEL), read(DIALECT))
    else {
        broken(&format!(
            "the pasted endpoint's round needs {BASE_URL}, {KEY}, {MODEL} and {DIALECT}"
        ));
        return None;
    };
    let dialect = match serde_json::from_value(serde_json::Value::String(raw)) {
        Ok(kind) => kind,
        Err(err) => panic!("{DIALECT} is not a dialect this city speaks: {err}"),
    };
    Some(Round {
        label: base_url.clone(),
        base_url,
        key,
        model,
        dialect,
    })
}

fn read(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

fn broken(needs: &str) {
    assert!(
        std::env::var(REQUIRED).as_deref() != Ok("1"),
        "{REQUIRED}=1, and {needs}"
    );
    println!("skipped: {needs}");
}

fn settled(round: &Round, key: &str, city_root: &Path) -> Result<RunWorker, AxError> {
    let secret = kernel::SecretRef::parse(SECRET_LOCATOR)?;
    let endpoint = wire::ProviderName::parse(ENDPOINT)?;
    let mut worker = RunWorker::new(
        city_root,
        runtime::diagnostics::Diagnostics::off(),
        assembly::hands(gateway::Custodian::in_memory()),
    )?;
    worker.handle(wire::Command::PutSecret {
        realm: secret.realm().to_owned(),
        name: secret.name().to_owned(),
        value: kernel::Sealed::new(Box::new(key.to_owned())),
    })?;
    worker.handle(wire::Command::AttachEndpoint {
        name: endpoint.clone(),
        base_url: round.base_url.clone(),
        dialect: round.dialect,
        secret: Some(SECRET_LOCATOR.to_owned()),
        auth_header: None,
        admit: Vec::new(),
        named: Vec::new(),
        tuning: wire::EndpointTuning {
            timeout_ms: Some(REQUEST_TIMEOUT_MS),
            request_max_retries: Some(NO_RETRY),
            ..wire::EndpointTuning::default()
        },
        idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, b"e2e-attach"),
    })?;
    worker.handle(wire::Command::SelectModel {
        endpoint,
        model: round.model.clone(),
        tag: kernel::ModelTag::Main,
        context_tokens: kernel::Window::new(32_768),
        max_output_tokens: kernel::Ceiling::new(1_024),
        input: None,
        idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, b"e2e-select"),
    })?;
    worker.handle(wire::Command::CreateBuilding {
        addr: Address::parse(BUILDING)?,
        template: wire::TemplateName::parse("minimal")?,
        rules: None,
        source: None,
        idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, b"e2e-create"),
    })?;
    Ok(worker)
}

fn dispatch(worker: &mut RunWorker) -> Result<(), AxError> {
    worker.handle(wire::Command::Dispatch {
        addr: Address::parse(BUILDING)?,
        task: "Reply with one word: ready.".to_owned(),
        goal: "one answer from the endpoint this city was pointed at".to_owned(),
        policy: kernel::RunPolicy::of(kernel::Mode::Work),
        idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, b"e2e-dispatch"),
        session: Some(kernel::SessionName::parse(SESSION)?),
        resume: None,
        images: Vec::new(),
        effort: None,
        model: None,
    })?;
    Ok(())
}

fn history(ledger_dir: &Path) -> Vec<EventRecord> {
    let verified = runtime::replay::verify_ledger_dir(ledger_dir).unwrap();
    verified
        .raw_lines()
        .iter()
        .map(|line| EventRecord::parse_line(line).unwrap())
        .collect()
}

fn codes(records: &[EventRecord]) -> Vec<String> {
    records
        .iter()
        .filter_map(|record| {
            record
                .data()
                .as_map()
                .get("code")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .collect()
}

#[test]
fn a_real_endpoint_answers_a_dispatch_and_the_history_says_so() {
    for round in rounds() {
        answers_a_dispatch(&round);
    }
}

fn answers_a_dispatch(round: &Round) {
    let label = &round.label;
    let dir = tempfile::tempdir().unwrap();
    let raised = form(
        dir.path(),
        Adopt::Nothing,
        assembly::hands(gateway::Custodian::in_memory()),
    )
    .unwrap();
    if let Err(err) =
        settled(round, &round.key, dir.path()).and_then(|mut worker| dispatch(&mut worker))
    {
        panic!("{label}: the dispatch did not reach the endpoint and come home: {err}");
    }

    let records = history(&raised.ledger_dir);
    assert!(
        records
            .iter()
            .any(|record| record.kind() == EventKind::ModelCalled),
        "{label}: the run never called the model"
    );

    let refused: Vec<String> = codes(&records)
        .into_iter()
        .filter(|code| {
            code == AxCode::ConfigInvalid.as_str() || code == AxCode::WireMismatch.as_str()
        })
        .collect();
    assert!(
        refused.is_empty(),
        "{label}: the call went out but the city and the endpoint disagree: {refused:?}"
    );
}

#[test]
fn the_history_states_where_the_output_ceiling_came_from() {
    for round in rounds() {
        states_the_ceilings_source(&round);
    }
}

fn states_the_ceilings_source(round: &Round) {
    let label = &round.label;
    let dir = tempfile::tempdir().unwrap();
    let raised = form(
        dir.path(),
        Adopt::Nothing,
        assembly::hands(gateway::Custodian::in_memory()),
    )
    .unwrap();

    if let Err(err) = settled(round, &round.key, dir.path()) {
        panic!("{label}: the endpoint did not take this key and this model: {err}");
    }

    let stated = history(&raised.ledger_dir)
        .into_iter()
        .filter(|record| record.kind() == EventKind::ModelSelected)
        .find_map(|record| {
            record
                .data()
                .as_map()
                .get("ceiling_from")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        });
    assert_eq!(
        stated.as_deref(),
        Some(gateway::FigureSource::Person.as_str()),
        "{label}: `model_selected` does not name the person's figure as the ceiling this city \
         registered"
    );
}

#[test]
fn a_wrong_key_is_refused_in_words_a_person_can_act_on() {
    for round in rounds() {
        refuses_a_wrong_key(&round);
    }
}

fn refuses_a_wrong_key(round: &Round) {
    let label = &round.label;
    let dir = tempfile::tempdir().unwrap();
    form(
        dir.path(),
        Adopt::Nothing,
        assembly::hands(gateway::Custodian::in_memory()),
    )
    .unwrap();

    let Err(refused) =
        settled(round, WRONG_KEY, dir.path()).and_then(|mut worker| dispatch(&mut worker))
    else {
        panic!("{label}: a wrong key reached the endpoint and the city called it a success");
    };

    assert!(
        !refused.recovery().is_empty(),
        "{label}: the refusal names no way forward: {refused}"
    );
    assert_ne!(
        *refused.code(),
        AxCode::ConfigInvalid,
        "{label}: a rejected credential is the endpoint's answer, not a malformed \
         configuration: {refused}"
    );
}
