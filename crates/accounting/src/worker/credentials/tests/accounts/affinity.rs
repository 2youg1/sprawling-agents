// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which account a Session keeps across a restart, and which one a new
//! or branched Session starts on (`crates/gateway/spec/Router.lean` D30).

use super::*;

/// The binding is a fact of the ledger, so a city reopened with its
/// snapshot gone still sends the Session's next request on the account
/// that answered last, though the first account answers again. A branch
/// and a new Session are new Sessions in the same room: each starts on
/// the first account again, and neither inherits the binding.
#[test]
fn the_binding_outlives_a_restart_and_a_new_or_branched_session_chooses_again() {
    let mut rig = Rig::new(
        vec![rejected(), answer(), rejected(), answer()],
        vec![answer()],
        unpaced(),
    );
    rig.dispatch();
    assert_eq!(rig.bound().as_deref(), Some("b"));

    let mut rig = rig.reopened_without_snapshot();
    rig.dispatch();
    assert_eq!(
        rig.wire_accounts(),
        ["a", "b", "b"],
        "the binding outlived the restart"
    );

    let branched_from = last_run(&rig);
    rig.command(|idem| wire::Command::OpenSession {
        addr: Address::parse(ROOM).unwrap(),
        carry: wire::Carry::Nothing,
        from: Some(branched_from),
        idem,
    });
    rig.dispatch();
    assert_eq!(
        rig.wire_accounts(),
        ["a", "b", "b", "a"],
        "a branch chooses again"
    );

    rig.dispatch();
    assert_eq!(rig.bound().as_deref(), Some("b"));
    rig.command(|idem| wire::Command::OpenSession {
        addr: Address::parse(ROOM).unwrap(),
        carry: wire::Carry::Nothing,
        from: None,
        idem,
    });
    rig.dispatch();
    assert_eq!(
        rig.wire_accounts(),
        ["a", "b", "b", "a", "a", "b", "a"],
        "a new Session chooses again"
    );
    assert_eq!(rig.bound().as_deref(), Some("a"));
    rig.assert_keys_stay_in_the_vault();
}

/// The last run the room started, and the last line of the ledger: the
/// point a branch continues from.
fn last_run(rig: &Rig) -> kernel::Origin {
    let started = rig.records("run_started");
    let run = started
        .last()
        .and_then(|line| line["run"].as_str())
        .map(|run| serde_json::from_value(serde_json::json!(run)).unwrap())
        .unwrap();
    let at_seq = rig
        .ledger()
        .last()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap()["seq"].clone())
        .map(|seq| serde_json::from_value(seq).unwrap())
        .unwrap();
    kernel::Origin { run, at_seq }
}
