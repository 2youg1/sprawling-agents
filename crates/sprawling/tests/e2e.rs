// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The acceptance gate for a real endpoint: `just e2e`.
//!
//! Every other check in this repository answers a provider this
//! repository wrote. This one attaches a base URL a person pasted,
//! chooses a model that endpoint really serves, dispatches, and reads
//! the history back — so "the key is filled in and nothing ever worked"
//! becomes a red test rather than a report.
//!
//! **The rounds are a table, not a list kept here** (sprawling-SPEC.md
//! 8-69): every face of every host `gateway::known_hosts` names, called
//! with the key and the model a person exported for that host, and then
//! the one endpoint a person pasted - a relay, a server on this machine.
//! A host added to the preset table is a round here without an edit.
//!
//! **Absent credentials print one line and pass**, the same honesty
//! `just adversary` keeps: a gate that silently skips is worse than no
//! gate, and a gate that fails on every machine without a key is one
//! nobody runs. The variables are named by the run itself, below. A job
//! that holds the credentials sets `SPRAWLING_E2E_REQUIRED=1`, and there
//! an absent variable is a broken configuration, so the gate turns red.
//!
//! **It enters by `RunWorker::handle`, the door `wire::server` hands
//! every frame to**, rather than by spawning the binary and opening a
//! socket. The endpoint under test is the provider's, and the process
//! boundary this repository judges from outside belongs to `tools/adversary/`
//! (xtask boundary gate; sprawling-SPEC.md 8-69).

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
use sprawling::assembly;

/// The four variables of the pasted endpoint, named once and printed
/// when one of them is missing. `KEY` and `MODEL` are also the stems of
/// a known host's two: `SPRAWLING_E2E_KEY_<HOST>` and
/// `SPRAWLING_E2E_MODEL_<HOST>`.
const BASE_URL: &str = "SPRAWLING_E2E_BASE_URL";
const KEY: &str = "SPRAWLING_E2E_KEY";
const MODEL: &str = "SPRAWLING_E2E_MODEL";
const DIALECT: &str = "SPRAWLING_E2E_DIALECT";

/// Set to `1` where the four variables above must be present, and where
/// a host given one of its two variables must be given both. Any other
/// value reads as unset, so a misspelt switch cannot turn back into a
/// silent skip without the job that set it noticing the skip line.
const REQUIRED: &str = "SPRAWLING_E2E_REQUIRED";

/// Where the credential is filed. The locator is the one home for this
/// fact: the realm and the name below are read out of it by the parser
/// that owns the grammar, rather than spelled a second time.
const SECRET_LOCATOR: &str = "secret:e2e/key";

/// What the endpoint, the building and the session are called. Each
/// name is written once and read back by the assertions.
const ENDPOINT: &str = "e2e";
const BUILDING: &str = "lab";
const SESSION: &str = "gate";

/// A key of the right shape and the wrong value, so the refusal comes
/// from the endpoint and not from a parser on this side.
const WRONG_KEY: &str = "sk-e2e-deliberately-wrong-key";

/// One request may take a minute and is never retried.
///
/// Both figures serve the runtime limit this gate is written under:
/// with retries at zero the gateway's backoff
/// never sleeps, and three calls at a minute each still land inside the
/// 180-second timeout the recipe imposes.
const REQUEST_TIMEOUT_MS: u64 = 60_000;
const NO_RETRY: u32 = 0;

/// One endpoint this gate calls, as the settings page would have it: a
/// base URL, its face, the key and the model.
struct Round {
    /// What a failure names: the host and its face, or the pasted URL.
    label: String,
    base_url: String,
    key: String,
    model: String,
    dialect: kernel::DialectKind,
}

/// Every round the variables on this machine make up: each face of each
/// known host whose key and model are exported, then the pasted
/// endpoint. A line says what was skipped; where [`REQUIRED`] is set, a
/// half-given round is a failure instead.
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

/// The two variables one known host's round reads: the host spelled in
/// capitals, every other character an underscore.
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

/// The pasted endpoint's four variables, or one line saying which of
/// them the machine running this does not have.
///
/// The dialect is parsed by `serde`, so the spellings this accepts are
/// the spellings the wire carries and the error names them; a table
/// here would be a second authority on `DialectKind`.
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

/// One variable, where it is set to something.
fn read(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

/// A round this machine cannot make: a failure where [`REQUIRED`] is
/// set, one line otherwise.
fn broken(needs: &str) {
    assert!(
        std::env::var(REQUIRED).as_deref() != Ok("1"),
        "{REQUIRED}=1, and {needs}"
    );
    println!("skipped: {needs}");
}

/// A city with the endpoint attached, the model chosen and one building
/// standing: exactly what the settings page leaves behind.
///
/// The key is a parameter rather than read from the round, because the
/// broken-key case differs from the working one in this value alone.
///
/// Attaching is itself a call to the endpoint — an empty `admit` list
/// asks it which models it serves — so a key the endpoint rejects is
/// refused here rather than at the dispatch. Every caller therefore
/// chains this into [`dispatch`] and judges the one `Result`.
fn settled(round: &Round, key: &str, city_root: &Path) -> Result<RunWorker, AxError> {
    let secret = kernel::SecretRef::parse(SECRET_LOCATOR)?;
    let endpoint = wire::ProviderName::parse(ENDPOINT)?;
    let mut worker = RunWorker::new(
        city_root,
        runtime::diagnostics::Diagnostics::off(),
        // The in-session vault: a gate that reached the platform
        // credential service would write a key into the machine running
        // it and leave it there.
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
        idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, b"e2e-create"),
    })?;
    Ok(worker)
}

/// One dispatch, asking for the shortest answer a model can give.
fn dispatch(worker: &mut RunWorker) -> Result<(), AxError> {
    worker.handle(wire::Command::Dispatch {
        addr: Address::parse(BUILDING)?,
        task: "Reply with one word: ready.".to_owned(),
        goal: "one answer from the endpoint this city was pointed at".to_owned(),
        policy: kernel::RunPolicy::of(kernel::Mode::Work),
        idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, b"e2e-dispatch"),
        session: Some(kernel::SessionName::parse(SESSION)?),
        effort: None,
        model: None,
    })?;
    Ok(())
}

/// The city's history, verified and parsed, which is the only thing
/// these assertions read: what a city did is what its ledger says.
fn history(ledger_dir: &Path) -> Vec<EventRecord> {
    let verified = runtime::replay::verify_ledger_dir(ledger_dir).unwrap();
    verified
        .raw_lines()
        .iter()
        .map(|line| EventRecord::parse_line(line).unwrap())
        .collect()
}

/// Every error code this history carries, in the order it was written.
///
/// Read from the `code` field the failing paths serialize their
/// `AxError` into, and compared against `AxCode::as_str`, so a code
/// renamed in `kernel` is renamed here too.
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
    let raised = assembly::init_city(dir.path()).unwrap();
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
    // The two codes that mean this city and that endpoint disagree about
    // the wire, rather than that the model said something unwelcome.
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
    let raised = assembly::init_city(dir.path()).unwrap();
    // The registration alone, without a dispatch: the rung is decided
    // when the model is chosen, so a gate that also required a call to
    // come home would report an endpoint outage as a ceiling nobody
    // stated. What a call does with the figure is the test above.
    if let Err(err) = settled(round, &round.key, dir.path()) {
        panic!("{label}: the endpoint did not take this key and this model: {err}");
    }

    // A separate test from the one above, because the two facts are
    // different: that a call went out, and that the account says which
    // link supplied the ceiling it carried. `settled` states a figure
    // of its own, so the rung owed here is the person's - a record
    // naming any other rung is a registration that quietly replaced a
    // number somebody entered.
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
        Some(gateway::CeilingSource::Person.as_str()),
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
    assembly::init_city(dir.path()).unwrap();

    let Err(refused) =
        settled(round, WRONG_KEY, dir.path()).and_then(|mut worker| dispatch(&mut worker))
    else {
        panic!("{label}: a wrong key reached the endpoint and the city called it a success");
    };
    // A refusal is a promise in three parts, and the third is the one a
    // person acts on; a gate that accepted a bare code would pass on the
    // day this became an unreadable failure.
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
