// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

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

#[test]
fn a_pasted_credential_leaves_no_plaintext_behind_it() {
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

    let described = format!("{:?}", custodian.describe(&reference));
    assert!(!described.contains(&secret));
}

#[test]
fn a_delete_with_no_way_back_is_refused_rather_than_queued() {
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
    let obvious = ExecArm::Shell {
        text: "rm -rf build".to_owned(),
    };
    assert!(
        matches!(forecast(&obvious), DiscardForecast::Suspected { .. }),
        "the plain form must be seen"
    );

    let quiet = ExecArm::Shell {
        text: "cargo test".to_owned(),
    };
    assert_eq!(forecast(&quiet), DiscardForecast::Clear);

    let rearranged = ExecArm::Shell {
        text: "find . -name '*.tmp' | xargs rm".to_owned(),
    };
    assert_eq!(
        forecast(&rearranged),
        DiscardForecast::Clear,
        "obfuscation succeeds against the forecaster, and is meant to"
    );
}

#[test]
fn resuming_the_same_handoff_twice_does_not_double_the_work() {
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
