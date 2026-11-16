// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The check derived from `crates/agent_protocols/spec/Harness/Consent.lean`
//! (`nothing_is_launched_before_consent`): the model leaves the digest a
//! parameter and admits a `given` consent exactly when the recorded digest
//! is the one recomputed from the spec. Here the real digest is fed, over
//! every launch the generator reaches and every change to it.

#![allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]

use proptest::prelude::*;

use super::*;

fn word() -> impl Strategy<Value = String> {
    "[a-z \"=@.-]{0,6}"
}

fn launch() -> impl Strategy<Value = Launch> {
    (
        word(),
        proptest::collection::vec(word(), 0..4),
        proptest::collection::vec((word(), word()), 0..3),
    )
        .prop_map(|(program, args, env)| Launch { program, args, env })
}

fn entry(launch: Launch) -> AgentEntry {
    AgentEntry {
        id: AgentId::parse("agent").unwrap(),
        name: "Agent".to_owned(),
        source: AgentSource::Pasted,
        launch,
        version: None,
        licence: None,
    }
}

/// One change to a launch: a word appended to the program, an argument
/// added, split or dropped, an environment value changed.
fn changed(launch: &Launch, how: u8, word: &str) -> Launch {
    let mut changed = launch.clone();
    match how % 4 {
        0 => changed.program.push_str(&format!("{word}x")),
        1 => changed.args.push(word.to_owned()),
        2 => match changed.args.pop() {
            Some(last) if last.len() > 1 => {
                let (a, b) = last.split_at(1);
                changed.args.extend([a.to_owned(), b.to_owned()]);
            }
            Some(_) | None => changed.env.push((word.to_owned(), "v".to_owned())),
        },
        _ => changed.env.push((format!("{word}N"), word.to_owned())),
    }
    changed
}

proptest! {
    /// The consent the card recorded starts exactly the spec it was
    /// recorded for, and no spec changed after it.
    #[test]
    fn a_consent_admits_its_own_spec_and_no_changed_one(
        launch in launch(),
        how in any::<u8>(),
        word in word(),
    ) {
        let recorded = launch.digest();
        prop_assert!(Consented::given(entry(launch.clone()), &recorded).is_ok());
        let other = changed(&launch, how, &word);
        prop_assume!(other != launch);
        prop_assert_eq!(
            Consented::given(entry(other), &recorded).map(|_| ()).map_err(|err| *err.code()),
            Err(AxCode::ConfigInvalid)
        );
    }
}
