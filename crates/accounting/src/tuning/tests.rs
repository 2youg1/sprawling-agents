// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The settings page reads an endpoint's tuning back and attaches it
//! again with only the account list changed, so reading back and
//! attaching must land the tuning the book already held
//! (`crates/wire/spec/Answer/Endpoints.lean` §8-85).

use kernel::account_recovery::AccountRetries;
use kernel::event::record::ProviderAccount;
use proptest::prelude::*;

use super::{tuning_as_attached, tuning_of};

/// An ordered account list the book accepts, or none: each account
/// keyed by a reference of its own or anonymous, and a named header only
/// beside a reference.
fn accounts() -> impl Strategy<Value = Option<Vec<ProviderAccount>>> {
    proptest::option::of(
        proptest::collection::vec((any::<bool>(), any::<bool>()), 1..4).prop_map(|rows| {
            rows.into_iter()
                .enumerate()
                .map(|(index, (keyed, named))| {
                    let id = format!("acct{index}");
                    ProviderAccount {
                        id: kernel::ServerLabel::parse(&id).unwrap(),
                        reference: keyed.then(|| kernel::SecretRef::new("fixture", &id).unwrap()),
                        header: (keyed && named).then(|| "x-api-key".to_owned()),
                    }
                })
                .collect()
        }),
    )
}

fn proxying() -> impl Strategy<Value = Option<kernel::Proxying>> {
    proptest::option::of(prop_oneof![
        Just(kernel::Proxying::ExceptLocal),
        Just(kernel::Proxying::Always),
        Just(kernel::Proxying::Never),
    ])
}

fn retries() -> impl Strategy<Value = Option<AccountRetries>> {
    proptest::option::of(prop_oneof![
        Just(AccountRetries::One),
        Just(AccountRetries::Two)
    ])
}

/// Header and override rows as a form sends them, blank names included;
/// a value is a short literal or a vault reference.
fn rows() -> impl Strategy<Value = Vec<(String, String)>> {
    proptest::collection::vec(
        (
            "[ /A-Za-z-]{0,6}",
            prop_oneof!["[a-z0-9.]{0,5}", Just("secret:fixture/header".to_owned())],
        ),
        0..3,
    )
}

prop_compose! {
    fn entered()(
        accounts in accounts(),
        label in proptest::option::of("[ a-z]{0,6}"),
        timeout_ms in proptest::option::of(0u64..200_000),
        request_max_retries in proptest::option::of(0u32..9),
        stream_idle_timeout_ms in proptest::option::of(0u64..200_000),
        headers in rows(),
        overrides in rows(),
        proxying in proxying(),
        max_in_flight in proptest::option::of(0u32..300),
        account_retries in retries(),
    ) -> wire::EndpointTuning {
        wire::EndpointTuning {
            accounts,
            label,
            timeout_ms,
            request_max_retries,
            stream_idle_timeout_ms,
            headers: headers
                .into_iter()
                .map(|(name, value)| wire::HeaderPair { name, value })
                .collect(),
            overrides: overrides
                .into_iter()
                .map(|(pointer, value)| wire::BodyOverride { pointer, value })
                .collect(),
            proxying,
            max_in_flight,
            account_retries,
        }
    }
}

proptest! {
    /// Whatever a form sent and the city accepted, the tuning read back
    /// from the book attaches to the same tuning.
    #[test]
    fn a_tuning_read_back_attaches_to_the_same_tuning(entered in entered()) {
        let Ok(held) = tuning_of(entered) else {
            return Ok(());
        };
        prop_assert_eq!(tuning_of(tuning_as_attached(&held)).unwrap(), held);
    }
}
