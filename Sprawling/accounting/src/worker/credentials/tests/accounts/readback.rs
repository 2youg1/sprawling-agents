// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the settings page reads back about the two accounts and a third
//! whose key was never filed: the tuning the endpoint was attached with,
//! and each account's key, read through the vault the worker lends the
//! views (`crates/wire/spec/Answer/Endpoints.lean` §8-85).

use super::*;

/// The page is handed the list in the order it was filed, with the
/// per-account figure it was given and every other figure absent, and
/// each account's key in the same order: read only once a vault is lent.
#[test]
fn each_account_reads_its_key_state_and_the_tuning_reads_back_as_attached() {
    let mut rig = Rig::new(Vec::new(), Vec::new(), unpaced());
    rig.attach(&["a", "b", "c"], Some(AccountRetries::One));
    let mut views =
        crate::views::Views::rebuild(&kernel::layout::CityLayout::new(rig.dir.path()).ledger())
            .unwrap();
    let unlent = house(views.prepare(&wire::Query::EndpointView).finish());
    views.lend_the_vault(rig.worker.vault_handle());
    let lent = house(views.prepare(&wire::Query::EndpointView).finish());

    let filed = ["a", "b", "c"]
        .iter()
        .map(|id| kernel::event::record::ProviderAccount {
            id: kernel::ServerLabel::parse(id).unwrap(),
            reference: Some(kernel::SecretRef::new("fixture", id).unwrap()),
            header: None,
        })
        .collect();
    let keys = |states: [wire::KeyState; 3]| {
        ["a", "b", "c"]
            .iter()
            .zip(states)
            .map(|(id, key)| wire::AccountStatus {
                id: kernel::ServerLabel::parse(id).unwrap(),
                key,
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        (unlent.account_status, lent.tuning, lent.account_status),
        (
            keys([wire::KeyState::Unread; 3]),
            wire::EndpointTuning {
                accounts: Some(filed),
                proxying: Some(kernel::Proxying::ExceptLocal),
                account_retries: Some(AccountRetries::One),
                ..wire::EndpointTuning::default()
            },
            keys([
                wire::KeyState::Stored,
                wire::KeyState::Stored,
                wire::KeyState::Missing
            ]),
        )
    );
}

/// The one endpoint the rig attached, from the settings page's read.
fn house(answer: wire::Answer) -> wire::EndpointSummary {
    let wire::Answer::Endpoints(book) = answer else {
        panic!("the settings page reads the endpoint book: {answer:?}");
    };
    book.endpoints
        .into_iter()
        .find(|endpoint| endpoint.name == "house")
        .unwrap()
}
