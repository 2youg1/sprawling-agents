// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The repair segment inside a call's account round: its resend is one
//! more send on the round's current account
//! (`crates/runtime/spec/Turn/Recovery.lean` §8-49).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use super::super::*;
use super::helpers::*;
use super::recovery::{model_return, provider_wobble, wire_mismatch};

/// The repair is one more send on the call's account: when the account
/// round the watchdog holds has spent that account's resends, the repair
/// stands aside and the failure goes to the watchdog as it came, with no
/// second attempt behind it.
#[test]
fn a_repair_the_account_round_has_no_budget_for_is_not_sent() {
    struct StreamCut {
        blocked: u32,
    }
    impl Model for StreamCut {
        fn call(&mut self, _req: &ModelRequest) -> Result<ModelReturn, AxError> {
            self.blocked = self.blocked.saturating_add(1);
            Ok(model_return())
        }
        fn call_streaming(
            &mut self,
            _req: &ModelRequest,
            _onto: kernel::Increments<'_>,
        ) -> Result<ModelReturn, AxError> {
            Err(wire_mismatch())
        }
    }
    let label = |name: &str| kernel::ServerLabel::parse(name).unwrap();
    let mut watchdog = crate::Watchdog::new(
        kernel::Retries::UntilHalted,
        kernel::RunId::from_bytes([3; 16]),
    );
    watchdog
        .open_round(Some(kernel::model::AccountRoster {
            provider: "house".to_owned(),
            accounts: ["a", "b"]
                .map(|name| kernel::model::RosterEntry {
                    account: label(name),
                    usable: true,
                })
                .to_vec(),
            current: label("a"),
            retries: kernel::account_recovery::AccountRetries::One,
        }))
        .unwrap();
    let busy = provider_wobble();
    assert!(matches!(
        watchdog.on_provider_failure(&busy, TimeMs::new(0)),
        crate::Disposal::BackOff { .. }
    ));
    let mut ledger = TestLedger::new();
    let mut model = StreamCut { blocked: 0 };
    let turn = opened::<1>();
    let turn = advance(
        turn.assemble(
            Interrupt::None,
            &mut ledger,
            RunPrompt::new(&prefix(), &mut PromptRecord::default()),
            blank_conversation(),
            &[],
            &shape(),
        )
        .unwrap(),
    );
    let mut sink = |_inc: &kernel::Increment| {};
    let err = turn
        .under(&mut watchdog)
        .call(
            Interrupt::None,
            &mut ledger,
            &mut model,
            &BuildingPolicy::default(),
            Generating::Watched(&mut sink),
        )
        .unwrap_err();
    assert_eq!(err, wire_mismatch());
    assert_eq!(model.blocked, 0, "no blocking resend went out");
}
