// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a running session refuses to send: a prefix whose bytes no
//! longer match the hash the session froze, a request whose system
//! blocks moved, and a call shape that changed under the session.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use super::super::*;
use super::helpers::*;
use crate::window::Opening;

/// The frozen prefix is a runtime invariant, not a laboratory one: the
/// bytes a request will carry are rehashed every turn and compared with
/// what the session froze, and a difference is a refusal with two ways
/// out rather than a warning.
#[test]
fn a_prefix_whose_bytes_moved_is_refused_before_anything_is_assembled() {
    let mut ledger = TestLedger::new();
    let mut window = Window::new();
    window.push_task_lines("probe the city", "one probe", Opening::FromJob);
    let drifted = crate::prefix::FrozenPrefix::assemble(
        crate::prefix::FrozenSegment::mislabelled(
            crate::prefix::SegmentSlot::City,
            b"c".to_vec(),
            B3Hash::digest(b"not the city"),
        ),
        crate::prefix::FrozenSegment::new(crate::prefix::SegmentSlot::Building, b"b".to_vec()),
        crate::prefix::FrozenSegment::new(crate::prefix::SegmentSlot::Resident, b"r".to_vec()),
        crate::prefix::FrozenSegment::new(crate::prefix::SegmentSlot::Run, b"j".to_vec()),
    )
    .unwrap();
    let turn = Turn::begin(run_id(), "resident@sim.1".into(), TimeMs::new(1));
    let Err(refusal) = turn.assemble(
        Interrupt::None,
        &mut ledger,
        &drifted,
        &window,
        &[],
        &shape(),
    ) else {
        panic!("a prefix whose recorded hash no longer describes its bytes is refused");
    };
    assert_eq!(refusal.code(), &kernel::AxCode::CasCorrupt);
    assert!(
        refusal.recovery().contains("fork this run"),
        "the recovery names the way out: {}",
        refusal.recovery()
    );
    assert!(
        ledger.kinds().is_empty(),
        "the refusal happens before any line is written: {:?}",
        ledger.kinds()
    );
}

/// The second per-turn digest: the four system blocks the request carries
/// must hash to the four the run froze, and each must keep its cache
/// breakpoint.
#[test]
fn the_system_blocks_a_request_carries_are_checked_against_the_frozen_hashes() {
    let frozen: [B3Hash; 4] = [
        B3Hash::digest(b"c"),
        B3Hash::digest(b"b"),
        B3Hash::digest(b"r"),
        B3Hash::digest(b"j"),
    ];
    let blocks = |run: &str, cached: bool| {
        vec![
            kernel::SystemBlock {
                text: "c".to_owned(),
                cache: true,
            },
            kernel::SystemBlock {
                text: "b".to_owned(),
                cache: true,
            },
            kernel::SystemBlock {
                text: "r".to_owned(),
                cache: true,
            },
            kernel::SystemBlock {
                text: run.to_owned(),
                cache: cached,
            },
        ]
    };
    assert_eq!(
        crate::prefix::verified_system_hashes(&blocks("j", true), &frozen).unwrap(),
        frozen
    );
    let Err(moved) = crate::prefix::verified_system_hashes(&blocks("k", true), &frozen) else {
        panic!("a block that moved is refused");
    };
    assert_eq!(moved.code(), &kernel::AxCode::CasCorrupt);
    assert!(moved.subject().contains("run"), "{}", moved);
    let Err(unmarked) = crate::prefix::verified_system_hashes(&blocks("j", false), &frozen) else {
        panic!("a block that lost its breakpoint is refused");
    };
    assert!(
        unmarked.subject().contains("cache breakpoint"),
        "{unmarked}"
    );
    assert!(
        crate::prefix::verified_system_hashes(&blocks("j", false)[..3], &frozen).is_err(),
        "four blocks, or the request is not this prefix"
    );
}

/// The model and the effort are the session's, not a form's. The runtime
/// check is exported for the dispatch face to call before it writes a
/// room's effort; `context_tokens` never reaches the wire and does not
/// enter the comparison.
#[test]
fn a_call_shape_that_changed_mid_session_is_refused() {
    let frozen = shape();
    assert!(shape().verified_against(&frozen).is_ok());

    let mut moved_model = shape();
    moved_model.model = "something-else".to_owned();
    let Err(refusal) = moved_model.verified_against(&frozen) else {
        panic!("a model changed mid-session is refused");
    };
    assert_eq!(refusal.code(), &kernel::AxCode::ConfigInvalid);
    assert!(
        refusal.recovery().contains("fork this run"),
        "{}",
        refusal.recovery()
    );

    let mut moved_effort = shape();
    moved_effort.effort = Some(kernel::Effort::High);
    assert!(moved_effort.verified_against(&frozen).is_err());

    let mut moved_ceiling = shape();
    moved_ceiling.max_tokens = kernel::Ceiling::new(4_096);
    assert!(moved_ceiling.verified_against(&frozen).is_err());

    let mut gauge = shape();
    gauge.context_tokens = 200_000;
    assert!(
        gauge.verified_against(&frozen).is_ok(),
        "the gauge sizes a local reminder and moves no request byte"
    );
}
