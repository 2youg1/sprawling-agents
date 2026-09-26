// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a credential this city already keeps does when the form
//! sends an empty box (sprawling-SPEC.md section 8-81).

use crate::assembly::fixture::*;
use crate::assembly::*;

/// An empty key box leaves the credential this city keeps where it is
/// (sprawling-SPEC.md 8-81).
///
/// The form holds the vault's reference only while it is mounted, so a
/// person who opens the settings page again and presses "look" or
/// "attach" says `secret: None` about an endpoint that has a key.
/// Reading that as "no credential" asks the provider with no
/// `Authorization` header at all - which answers 401, and a person
/// reads that as their key being wrong - and then writes the endpoint
/// back with its key gone. Clearing one is `DetachEndpoint`.
#[test]
fn an_empty_key_keeps_the_credential_this_city_has_archived() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let (base_url, provider) = fake_openai(&["m-key"], Vec::new());
    let mut worker = RunWorker::new(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap();
    worker
        .handle(channels::Command::PutSecret {
            realm: "kept".to_owned(),
            name: "key".to_owned(),
            value: kernel::Sealed::new(Box::new("sk-archived".to_owned())),
        })
        .unwrap();
    let attach = |secret: Option<&str>, mark: &[u8]| channels::Command::AttachEndpoint {
        name: channels::ProviderName::parse("kept").unwrap(),
        base_url: base_url.clone(),
        dialect: kernel::DialectKind::OpenAi,
        secret: secret.map(str::to_owned),
        auth_header: None,
        admit: Vec::new(),
        tuning: channels::EndpointTuning::default(),
        idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, mark),
    };
    worker
        .handle(attach(Some("secret:kept/key"), b"attach"))
        .unwrap();
    // The page was closed and opened again: the form has no reference
    // to send, and both of its buttons say so.
    worker
        .handle(channels::Command::ProbeEndpoint {
            name: channels::ProviderName::parse("kept").unwrap(),
            base_url: base_url.clone(),
            dialect: kernel::DialectKind::OpenAi,
            secret: None,
            auth_header: None,
            tuning: channels::EndpointTuning::default(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"probe"),
        })
        .unwrap();
    worker.handle(attach(None, b"again")).unwrap();
    let asked: Vec<String> = provider
        .exchanges()
        .into_iter()
        .filter(|head| head.starts_with("GET ") && head.contains("models"))
        .collect();
    assert_eq!(asked.len(), 3, "one model list per attach and per probe");
    assert!(
        asked.iter().all(|head| head
            .to_ascii_lowercase()
            .contains("authorization: bearer sk-archived")),
        "every ask about a keyed endpoint carries its key; heads were:\n{asked:#?}"
    );
    let kept = worker
        .credentials
        .book
        .endpoints()
        .find(|endpoint| endpoint.name == "kept")
        .expect("the endpoint stayed attached");
    let gateway::AuthSpec::Bearer(reference) = &kept.auth else {
        panic!(
            "an empty key box is not a request to remove the key; the endpoint holds {:?}",
            kept.auth
        );
    };
    assert_eq!(
        reference,
        &kernel::SecretRef::parse("secret:kept/key").unwrap()
    );
}
