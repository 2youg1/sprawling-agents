// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Forgetting a key through `ForgetSecret`: kept while anything the
//! city reads still names the reference, deleted once nothing does
//! (`crates/accounting/spec/Worker.lean` §8-37).

use kernel::{AxCode, AxError};

use super::*;

/// Sends `ForgetSecret` for `reference` and hands back the city's answer.
fn forget(rig: &mut Rig, reference: &str) -> Result<(), AxError> {
    rig.sent = rig.sent.saturating_add(1);
    let idem = IdemKey::derive(&RunId::CITY, Seq::FIRST, &rig.sent.to_be_bytes());
    rig.worker
        .handle(wire::Command::ForgetSecret(wire::SecretForgetting {
            reference: reference.to_owned(),
            idem,
        }))
        .map(|_| ())
}

/// Whether the vault still redeems `reference`.
fn held(rig: &Rig, reference: &str) -> bool {
    let reference = kernel::SecretRef::parse(reference).unwrap();
    (rig.worker.resolver())(&reference).is_ok()
}

fn refusal(error: &AxError) -> (AxCode, String, String) {
    (
        *error.code(),
        error.action().to_owned(),
        error.subject().to_owned(),
    )
}

#[test]
fn a_key_an_endpoint_names_is_kept_until_its_list_no_longer_names_it() {
    let mut rig = Rig::new(Vec::new(), Vec::new(), unpaced());
    let refused = forget(&mut rig, "secret:fixture/b").unwrap_err();
    assert_eq!(
        (refusal(&refused), held(&rig, "secret:fixture/b")),
        (
            (
                AxCode::ConfigInvalid,
                "forget credential".to_owned(),
                "secret:fixture/b is still named by provider `house`".to_owned()
            ),
            true
        )
    );

    rig.attach(&["a"], None);
    forget(&mut rig, "secret:fixture/b").unwrap();
    assert_eq!(
        (
            held(&rig, "secret:fixture/b"),
            held(&rig, "secret:fixture/a")
        ),
        (false, true)
    );
}

#[test]
fn a_key_the_city_search_or_a_building_names_is_kept() {
    let mut rig = Rig::new(Vec::new(), Vec::new(), unpaced());
    rig.worker
        .handle(wire::Command::PutSecret {
            realm: "search".to_owned(),
            name: "brave.main".to_owned(),
            value: kernel::Sealed::new(Box::new("fixture-search-key".to_owned())),
        })
        .unwrap();
    let supplier = |reference: &str| kernel::config::SearchSupplier {
        id: kernel::ServerLabel::parse("brave").unwrap(),
        url: "https://mcp.brave.invalid/mcp".to_owned(),
        remote: "brave_web_search".to_owned(),
        query_field: "query".to_owned(),
        objective_field: None,
        count_field: None,
        accounts: vec![kernel::event::record::ProviderAccount {
            id: kernel::ServerLabel::parse("main").unwrap(),
            reference: Some(kernel::SecretRef::parse(reference).unwrap()),
            header: Some("x-subscription-token".to_owned()),
        }],
    };
    let custom = |reference: &str| kernel::config::SearchConfiguration::Custom {
        selected: kernel::ServerLabel::parse("brave").unwrap(),
        suppliers: vec![supplier(reference)],
    };
    let root = rig.dir.path().to_owned();
    city::write_search(&root, &custom("secret:search/brave.main")).unwrap();
    let by_the_city = forget(&mut rig, "secret:search/brave.main").unwrap_err();

    // The city now names another key, and the hall's own file the first.
    city::write_search(&root, &custom("secret:search/brave.spare")).unwrap();
    let hall = Address::parse("hall").unwrap();
    let file = city::config_path(&root, &hall, city::Layer::Building).unwrap();
    let mut text = std::fs::read_to_string(&file).unwrap_or_default();
    text.push_str(
        "\n[[mcp]]\nlabel = \"brave\"\nurl = \"https://mcp.brave.invalid/mcp\"\nheaders = { x-subscription-token = \"secret:search/brave.main\" }\n",
    );
    std::fs::write(&file, text).unwrap();
    let by_the_hall = forget(&mut rig, "secret:search/brave.main").unwrap_err();

    assert_eq!(
        (
            refusal(&by_the_city),
            refusal(&by_the_hall),
            held(&rig, "secret:search/brave.main")
        ),
        (
            (
                AxCode::ConfigInvalid,
                "forget credential".to_owned(),
                "secret:search/brave.main is still named by the city's [search] supplier `brave`"
                    .to_owned()
            ),
            (
                AxCode::ConfigInvalid,
                "forget credential".to_owned(),
                "secret:search/brave.main is still named by [[mcp]] `brave` at `hall`".to_owned()
            ),
            true
        )
    );
}

#[test]
fn a_reference_that_does_not_parse_is_refused_by_the_reference_grammar() {
    let mut rig = Rig::new(Vec::new(), Vec::new(), unpaced());
    let refused = forget(&mut rig, "fixture/b").unwrap_err();
    let parsed = kernel::SecretRef::parse("fixture/b")
        .map(|_| ())
        .unwrap_err();
    assert_eq!(refusal(&refused), refusal(&parsed));
}
