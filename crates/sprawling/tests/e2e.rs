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
//! **Absent credentials print one line and pass**, the same honesty
//! `just adversary` keeps: a gate that silently skips is worse than no
//! gate, and a gate that fails on every machine without a key is one
//! nobody runs. The variables are named by the run itself, below.
//!
//! **It enters by `RunWorker::handle`, the door `channels::server` hands
//! every frame to**, rather than by spawning the binary and opening a
//! socket. The endpoint under test is the provider's, and the process
//! boundary this repository judges from outside belongs to `adversary/`
//! (xtask boundary gate; sprawling-SPEC.md 8-69).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use std::path::Path;

use kernel::{Address, AxCode, AxError, EventKind, EventRecord, IdemKey, RunId, Seq};
use sprawling::assembly::{self, RunWorker};

/// The four variables this gate reads, named once and printed when one
/// of them is missing.
const BASE_URL: &str = "SPRAWLING_E2E_BASE_URL";
const KEY: &str = "SPRAWLING_E2E_KEY";
const MODEL: &str = "SPRAWLING_E2E_MODEL";
const DIALECT: &str = "SPRAWLING_E2E_DIALECT";

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
/// Both figures serve the runtime constraint this gate is written under
/// (Roadmap section 0.0): with retries at zero the gateway's backoff
/// never sleeps, and three calls at a minute each still land inside the
/// 180-second timeout the recipe imposes.
const REQUEST_TIMEOUT_MS: u64 = 60_000;
const NO_RETRY: u32 = 0;

/// What a person pasted into the settings page, read from the
/// environment.
struct Live {
    base_url: String,
    key: String,
    model: String,
    dialect: kernel::DialectKind,
}

/// The four variables, or one line saying which of them the machine
/// running this does not have.
///
/// The dialect is parsed by `serde`, so the spellings this accepts are
/// the spellings the wire carries and the error names them; a table
/// here would be a second authority on `DialectKind`.
fn live() -> Option<Live> {
    let read = |name: &str| std::env::var(name).ok().filter(|v| !v.is_empty());
    let (Some(base_url), Some(key), Some(model), Some(raw)) =
        (read(BASE_URL), read(KEY), read(MODEL), read(DIALECT))
    else {
        println!(
            "skipped: this gate calls a real endpoint and needs {BASE_URL}, {KEY}, {MODEL} and \
             {DIALECT}"
        );
        return None;
    };
    let dialect = match serde_json::from_value(serde_json::Value::String(raw)) {
        Ok(kind) => kind,
        Err(err) => panic!("{DIALECT} is not a dialect this city speaks: {err}"),
    };
    Some(Live {
        base_url,
        key,
        model,
        dialect,
    })
}

/// A city with the endpoint attached, the model chosen and one building
/// standing: exactly what the settings page leaves behind.
///
/// The key is a parameter rather than read from `live`, because the
/// broken-key case differs from the working one in this value alone.
///
/// Attaching is itself a call to the endpoint — an empty `admit` list
/// asks it which models it serves — so a key the endpoint rejects is
/// refused here rather than at the dispatch. Every caller therefore
/// chains this into [`dispatch`] and judges the one `Result`.
fn settled(live: &Live, key: &str, city_root: &Path) -> Result<RunWorker, AxError> {
    let secret = kernel::SecretRef::parse(SECRET_LOCATOR)?;
    let endpoint = channels::ProviderName::parse(ENDPOINT)?;
    let mut worker = RunWorker::new(
        city_root,
        // The in-session vault: a gate that reached the platform
        // credential service would write a key into the machine running
        // it and leave it there.
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )?;
    worker.handle(channels::Command::PutSecret {
        realm: secret.realm().to_owned(),
        name: secret.name().to_owned(),
        value: kernel::Sealed::new(Box::new(key.to_owned())),
    })?;
    worker.handle(channels::Command::AttachEndpoint {
        name: endpoint.clone(),
        base_url: live.base_url.clone(),
        dialect: live.dialect,
        secret: Some(SECRET_LOCATOR.to_owned()),
        auth_header: None,
        admit: Vec::new(),
        tuning: channels::EndpointTuning {
            timeout_ms: Some(REQUEST_TIMEOUT_MS),
            request_max_retries: Some(NO_RETRY),
            ..channels::EndpointTuning::default()
        },
        idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, b"e2e-attach"),
    })?;
    worker.handle(channels::Command::SelectModel {
        endpoint,
        model: live.model.clone(),
        tag: kernel::ModelTag::Main,
        context_tokens: 32_768,
        max_output_tokens: kernel::Ceiling::new(1_024),
        idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, b"e2e-select"),
    })?;
    worker.handle(channels::Command::CreateBuilding {
        addr: Address::parse(BUILDING)?,
        template: channels::TemplateName::parse("minimal")?,
        idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, b"e2e-create"),
    })?;
    Ok(worker)
}

/// One dispatch, asking for the shortest answer a model can give.
fn dispatch(worker: &mut RunWorker) -> Result<(), AxError> {
    worker.handle(channels::Command::Dispatch {
        addr: Address::parse(BUILDING)?,
        task: "Reply with one word: ready.".to_owned(),
        goal: "one answer from the endpoint this city was pointed at".to_owned(),
        mode: channels::ModeTag::parse("plan")?,
        idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, b"e2e-dispatch"),
        session: Some(kernel::SessionName::parse(SESSION)?),
        effort: None,
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
    let Some(live) = live() else { return };
    let dir = tempfile::tempdir().unwrap();
    let raised = assembly::init_city(dir.path()).unwrap();
    settled(&live, &live.key, dir.path())
        .and_then(|mut worker| dispatch(&mut worker))
        .expect("the dispatch reached the endpoint and came home");

    let records = history(&raised.ledger_dir);
    assert!(
        records
            .iter()
            .any(|record| record.kind() == EventKind::ModelCalled),
        "the run never called the model"
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
        "the call went out but the city and the endpoint disagree: {refused:?}"
    );
}

#[test]
fn the_history_states_where_the_output_ceiling_came_from() {
    let Some(live) = live() else { return };
    let dir = tempfile::tempdir().unwrap();
    let raised = assembly::init_city(dir.path()).unwrap();
    // The registration alone, without a dispatch: the rung is decided
    // when the model is chosen, so a gate that also required a call to
    // come home would report an endpoint outage as a ceiling nobody
    // stated. What a call does with the figure is the test above.
    settled(&live, &live.key, dir.path()).expect("the endpoint took this key and this model");

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
        "`model_selected` does not name the person's figure as the ceiling this city registered"
    );
}

#[test]
fn a_wrong_key_is_refused_in_words_a_person_can_act_on() {
    let Some(live) = live() else { return };
    let dir = tempfile::tempdir().unwrap();
    assembly::init_city(dir.path()).unwrap();

    let refused = settled(&live, WRONG_KEY, dir.path())
        .and_then(|mut worker| dispatch(&mut worker))
        .expect_err("a wrong key reached the endpoint and the city called it a success");
    // A refusal is a promise in three parts, and the third is the one a
    // person acts on; a gate that accepted a bare code would pass on the
    // day this became an unreadable failure.
    assert!(
        !refused.recovery().is_empty(),
        "the refusal names no way forward: {refused}"
    );
    assert_ne!(
        *refused.code(),
        AxCode::ConfigInvalid,
        "a rejected credential is the endpoint's answer, not a malformed configuration: {refused}"
    );
}
