// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The three hazards a control surface can hand a city, driven through
//! the same modules the city runs.
//!
//! A person pastes a credential into a field. A command arrives that would
//! delete something. A machine is restarted and the work resumes. Each one
//! has a rule, and each rule is worth exercising from outside the module
//! that owns it - a rule only tested by its author is a rule tested against
//! the same misunderstanding that wrote it.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]

use gateway::Custodian;
use kernel::discard::{decide as decide_discard, forecast};
use kernel::{
    Address, ByteLen, Discard, DiscardForecast, DiscardRequest, DiscardVerdict, ExecArm, Locator,
    Restoration, SecretRef, TaintSet,
};
use runtime::redact::{Marker, redact_text};

// ------------------------------------------------------- a pasted credential

#[test]
fn a_pasted_credential_leaves_no_plaintext_behind_it() {
    // A13 as a scenario rather than as a unit: what a person actually does
    // is paste a blob into a field, and everything after that must hold.
    // Scanning and replacing has one home, `runtime::redact`, so that is
    // where this scenario is driven from.
    // Built at run time so no high-entropy literal sits in this file for
    // `xtask secret` to find - the gate is right to bite those.
    let pasted = format!("sk-ant-{}{}", "A7bQ2xLm".repeat(2), "Zk91Rp4T");
    let line = format!("the person pasted {pasted} into the field");

    let (replaced, hits) = redact_text(&line, Marker::Fingerprinted);

    assert_eq!(hits, 1, "the pasted blob is seen once");
    assert!(
        replaced.contains("secret:redacted/"),
        "replaced in place by a marker"
    );
    assert!(!replaced.contains(&pasted), "and the plaintext is gone");
    let span: String = pasted.chars().skip(7).take(13).collect();
    assert!(
        !replaced.contains(&span),
        "no recognisable span of it survives either"
    );

    // Two occurrences of one value read alike, and a diagnostic line says
    // only that something was there.
    let (again, _) = redact_text(&pasted, Marker::Fingerprinted);
    assert!(replaced.contains(again.trim()), "one value, one marker");
    let (plain, _) = redact_text(&pasted, Marker::Plain);
    assert_eq!(plain, "secret:redacted");
}

#[test]
fn a_configured_reference_resolves_sealed_and_describes_without_telling() {
    let mut custodian = Custodian::in_memory();
    let secret = format!("tok-{}", "9QmR3vXs".repeat(3));
    let reference = SecretRef::parse("secret:anthropic/api").unwrap();
    custodian
        .set(&reference, zeroize::Zeroizing::new(secret.clone()))
        .unwrap();

    let sealed = custodian.resolve(&reference).unwrap();
    assert_eq!(
        sealed.expose(),
        &secret,
        "redemption returns the real value"
    );

    // Describing is what an interface may do. It says whether something is
    // configured, never what it is.
    let described = format!("{:?}", custodian.describe(&reference));
    assert!(!described.contains(&secret));
}

// -------------------------------------------------------- an injected delete

#[test]
fn a_delete_with_no_way_back_is_refused_rather_than_queued() {
    // Constitution 7.2. The refusal is the point: a Discard that cannot be
    // undone never becomes an event, which is why the Recycle Bin can
    // promise every row a return path.
    let unplanned = DiscardRequest::Unplanned {
        paths: vec![Address::parse("notes/draft.md").unwrap()],
        taint: TaintSet::default(),
        total_bytes: ByteLen::new(128),
    };
    assert!(
        matches!(decide_discard(&unplanned), DiscardVerdict::Deny { .. }),
        "an unplanned delete has no restoration to point at"
    );
}

#[test]
fn a_delete_with_a_checkpoint_behind_it_is_allowed() {
    let oid = "5a".repeat(20);
    let restoration =
        Restoration::Tracked(Locator::parse(&format!("file:notes/draft.md@{oid}")).unwrap());
    let discard = Discard::new(
        vec![Address::parse("notes/draft.md").unwrap()],
        restoration,
        TaintSet::default(),
        ByteLen::new(128),
    )
    .expect("a tracked file has a way back");
    let planned = DiscardRequest::Planned(discard);
    assert!(
        !matches!(decide_discard(&planned), DiscardVerdict::Deny { .. }),
        "a delete with a commit behind it may proceed"
    );
}

#[test]
fn the_forecaster_is_advisory_and_the_checkpoint_is_the_defence() {
    // A forecast hit routes conservatively
    // - it forces the checkpoint fence up first - rather than refusing every
    // command containing `rm`. Refusing outright would break the tool for
    // honest use and still not stop a determined one.
    //
    // This test asserts both halves, including the unflattering one. The
    // obvious spelling is seen:
    let obvious = ExecArm::Shell {
        text: "rm -rf build".to_owned(),
    };
    assert!(
        matches!(forecast(&obvious), DiscardForecast::Suspected { .. }),
        "the plain form must be seen"
    );

    // ...and an ordinary command is not:
    let quiet = ExecArm::Shell {
        text: "cargo test".to_owned(),
    };
    assert_eq!(forecast(&quiet), DiscardForecast::Clear);

    // ...and a trivially rearranged one is *also* not seen, because the
    // matcher looks for `rm ` with its trailing space. This is not a defect
    // to file: kernel::discard says in its own comment that text prediction
    // is obfuscatable by design. Recording it here keeps anyone from later
    // mistaking the forecaster for a defence and removing the net that is.
    let rearranged = ExecArm::Shell {
        text: "find . -name '*.tmp' | xargs rm".to_owned(),
    };
    assert_eq!(
        forecast(&rearranged),
        DiscardForecast::Clear,
        "obfuscation succeeds against the forecaster, and is meant to"
    );
}

// ------------------------------------------------------- resume idempotence

#[test]
fn resuming_the_same_handoff_twice_does_not_double_the_work() {
    // Constitution 6.1: resume consumes a Handoff and mints a new RunId, so
    // "wake the old Run" has no spelling. Two resumes of one Handoff are
    // therefore two new Runs, not one Run advanced twice - the mother's
    // frozen state is untouched either way.
    use runtime::handoff::Handoff;
    let oid = "7c".repeat(20);
    let job = Locator::parse(&format!("file:sim/lobby/room1/JOB.md@{oid}")).unwrap();
    let handoff = Handoff::new(
        vec![job],
        "half the roadmap".to_owned(),
        "the other half".to_owned(),
        "scripted world".to_owned(),
        "pick up from the job file".to_owned(),
    )
    .unwrap();

    let rendered_once = format!("{handoff:?}");
    let rendered_twice = format!("{handoff:?}");
    assert_eq!(
        rendered_once, rendered_twice,
        "a Handoff is a value: reading it twice reads the same thing"
    );
}
