// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Taking a setting back by sending the command that sets it again.
//!
//! **Undo is not a verb here.** A person who changes their mind inside
//! the undo window sends the same command with the value it had before,
//! and the ledger keeps both lines: what the city stands at is the last
//! line, and what happened is all of them. A city that erased the first
//! line would be a city whose history disagreed with the person's own
//! memory of it.
//!
//! What this holds is the property that makes such a reversal safe:
//! sending a setting twice leaves the city where sending it once did,
//! so a reversal that crossed a reconnection and arrived twice is still
//! one reversal (sprawling-SPEC.md 8-46-8).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use kernel::{IdemKey, RunId, Seq};
use sprawling::assembly;

/// Opens a city and the worker that runs it.
fn a_city(dir: &std::path::Path) -> assembly::RunWorker {
    assembly::init_city(dir).unwrap();
    assembly::RunWorker::new(
        dir,
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap()
}

/// Every command carries a key, and two commands a person sent are two
/// commands however alike they look.
fn key(of: &[u8]) -> IdemKey {
    IdemKey::derive(&RunId::CITY, Seq::FIRST, of)
}

/// Who answers is what a person is most likely to want back within
/// seconds of changing it: it decides whether the next question waits
/// for them or is answered by a resident.
#[test]
fn setting_autonomy_back_leaves_the_city_where_it_started() {
    let dir = tempfile::tempdir().unwrap();
    let mut worker = a_city(dir.path());
    let resident = kernel::ResidentId::new("lab/room1").unwrap();

    let delegated = channels::Command::SetAutonomy {
        scope: channels::HaltScope::City,
        autonomy: kernel::Autonomy::Delegate(resident),
        idem: key(b"delegate"),
    };
    let back = channels::Command::SetAutonomy {
        scope: channels::HaltScope::City,
        autonomy: kernel::Autonomy::Owner,
        idem: key(b"back"),
    };
    let again = channels::Command::SetAutonomy {
        scope: channels::HaltScope::City,
        autonomy: kernel::Autonomy::Owner,
        idem: key(b"back-again"),
    };

    worker.handle(delegated).unwrap();
    worker.handle(back).unwrap();
    let once = sprawling::ask(dir.path(), &channels::Query::Governance).unwrap();
    worker.handle(again).unwrap();
    let twice = sprawling::ask(dir.path(), &channels::Query::Governance).unwrap();

    assert_eq!(
        once, twice,
        "a reversal that arrived twice is still one reversal"
    );
}

/// A city with no endpoint attached refuses to choose a model, which is
/// what makes the reverse of `select_model` conditional: it exists only
/// while the endpoint the earlier choice named is still attached.
///
/// Recorded as a test rather than as a note, because the client's undo
/// button is built on the assumption and this is where it is true or
/// false.
#[test]
fn choosing_a_model_needs_the_endpoint_the_earlier_choice_named() {
    let dir = tempfile::tempdir().unwrap();
    let mut worker = a_city(dir.path());

    let err = worker
        .handle(channels::Command::SelectModel {
            endpoint: channels::ProviderName::parse("nowhere").unwrap(),
            model: "m-local".to_owned(),
            tag: kernel::ModelTag::Main,
            context_tokens: None,
            max_output_tokens: None,
            idem: key(b"choose"),
        })
        .unwrap_err();
    assert!(
        err.subject().contains("is not attached"),
        "the refusal names the endpoint: {err}"
    );
}
