// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! **These tests are this crate's statement of what it believes the
//! broker sends.** The field names in `broker.rs` are a reading of
//! somebody else's JSON, and a reading that nothing pins is a guess
//! that fails in front of a person. The fake server answers the shapes
//! that reading expects; when the broker changes its answers, these are
//! what say so.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "test code"
)]

use kernel::Sealed;

use super::{Broker, Connection};
#[cfg(test)]
use crate::endpoint::fakes::fake_provider;

/// The fake's origin, without the path it was built for.
fn origin(url: &str) -> String {
    let parsed = reqwest::Url::parse(url).unwrap();
    format!(
        "http://{}:{}/",
        parsed.host_str().unwrap(),
        parsed.port().unwrap()
    )
}

fn key() -> Sealed<String> {
    Sealed::new(Box::new("ak_test".to_owned()))
}

#[test]
fn a_shelf_joins_the_directory_with_this_person_s_standings() {
    let directory = serde_json::json!({
        "items": [
            { "slug": "github", "name": "GitHub", "auth_schemes": ["OAUTH2"] },
            { "slug": "linear", "name": "Linear", "auth_schemes": ["OAUTH2"] },
        ]
    })
    .to_string();
    let standings = serde_json::json!({
        "items": [{ "toolkit_slug": "github", "status": "ACTIVE", "alias": "octocat" }]
    })
    .to_string();
    let (url, handle) = fake_provider(vec![(200, directory), (200, standings)], false);
    let broker = Broker::at(&origin(&url), key()).unwrap();

    let shelf = broker.shelf("acme").unwrap();

    assert_eq!(shelf.len(), 2);
    assert_eq!(shelf[0].name, "GitHub");
    assert_eq!(shelf[0].auth, "oauth2", "the scheme is carried lowercased");
    assert_eq!(
        shelf[0].standing,
        Connection::Connected {
            alias: "octocat".to_owned()
        }
    );
    // A row the directory names and the standings do not is nobody's
    // connection, which is what `Absent` says.
    assert_eq!(shelf[1].standing, Connection::Absent);

    let seen = handle.join().unwrap();
    assert!(seen[0].contains("GET /"), "the directory is a read");
    assert!(
        seen[0].contains("x-api-key: ak_test"),
        "every call carries the project key"
    );
    assert!(
        seen[1].contains("user_ids=acme"),
        "standings are asked for one person, not for the project"
    );
}

#[test]
fn connecting_reuses_an_auth_config_rather_than_making_a_second() {
    let held = serde_json::json!({ "items": [{ "id": "ac_kept" }] }).to_string();
    let opened = serde_json::json!({ "redirect_url": "https://consent.example/abc" }).to_string();
    let (url, handle) = fake_provider(vec![(200, held), (200, opened)], false);
    let broker = Broker::at(&origin(&url), key()).unwrap();

    let page = broker.connect("github", "acme").unwrap();

    assert_eq!(page, "https://consent.example/abc");
    let seen = handle.join().unwrap();
    assert_eq!(
        seen.len(),
        2,
        "an existing config is not made a second time"
    );
    assert!(seen[1].contains("\"auth_config_id\":\"ac_kept\""));
    assert!(
        seen[1].contains("\"user_id\":\"acme\""),
        "the city connects under its own name"
    );
}

#[test]
fn connecting_creates_a_managed_config_when_the_project_has_none() {
    let none = serde_json::json!({ "items": [] }).to_string();
    let made = serde_json::json!({ "id": "ac_new" }).to_string();
    let opened = serde_json::json!({ "redirect_url": "https://consent.example/x" }).to_string();
    let (url, handle) = fake_provider(vec![(200, none), (200, made), (200, opened)], false);
    let broker = Broker::at(&origin(&url), key()).unwrap();

    assert!(broker.connect("github", "acme").is_ok());

    let seen = handle.join().unwrap();
    assert_eq!(seen.len(), 3);
    assert!(
        seen[1].contains("use_composio_managed_auth"),
        "a managed config is what makes one press enough: no dashboard visit, \
         and no OAuth application of the person's own"
    );
}

#[test]
fn a_slug_reaches_the_query_string_percent_encoded() {
    let none = serde_json::json!({ "items": [] }).to_string();
    let made = serde_json::json!({ "id": "ac_new" }).to_string();
    let opened = serde_json::json!({ "redirect_url": "https://consent.example/x" }).to_string();
    let (url, handle) = fake_provider(vec![(200, none), (200, made), (200, opened)], false);
    let broker = Broker::at(&origin(&url), key()).unwrap();

    let _ = broker.connect("git hub/../admin", "acme");

    let seen = handle.join().unwrap();
    assert!(
        !seen[0].contains("git hub/../admin"),
        "a slug from a client frame never reaches the path unescaped"
    );
    assert!(seen[0].contains("git+hub%2F..%2Fadmin"));
}

#[test]
fn a_refused_key_reads_as_a_missing_credential_rather_than_a_provider_fault() {
    let (url, handle) = fake_provider(vec![(401, "{}".to_owned())], false);
    let broker = Broker::at(&origin(&url), key()).unwrap();

    let refusal = broker.shelf("acme").unwrap_err();

    assert_eq!(*refusal.code(), kernel::AxCode::CredentialMissing);
    assert!(
        !refusal.recovery().is_empty(),
        "a refusal a person reads names what to do about it"
    );
    drop(handle.join());
}

#[test]
fn a_failed_connection_keeps_the_reason_the_broker_gave() {
    let directory =
        serde_json::json!({ "items": [{ "slug": "github", "name": "GitHub" }] }).to_string();
    let standings = serde_json::json!({
        "items": [{
            "toolkit_slug": "github",
            "status": "FAILED",
            "status_reason": "the person closed the consent page"
        }]
    })
    .to_string();
    let (url, handle) = fake_provider(vec![(200, directory), (200, standings)], false);
    let broker = Broker::at(&origin(&url), key()).unwrap();

    let shelf = broker.shelf("acme").unwrap();

    match &shelf[0].standing {
        Connection::Refused { refusal } => {
            assert_eq!(refusal.subject(), "the person closed the consent page");
        }
        other => panic!("a failed connection must carry its reason, got {other:?}"),
    }
    drop(handle.join());
}

#[test]
fn a_status_this_build_has_not_heard_of_offers_the_button_again() {
    let directory =
        serde_json::json!({ "items": [{ "slug": "github", "name": "GitHub" }] }).to_string();
    let standings = serde_json::json!({
        "items": [{ "toolkit_slug": "github", "status": "SOMETHING_NEW" }]
    })
    .to_string();
    let (url, handle) = fake_provider(vec![(200, directory), (200, standings)], false);
    let broker = Broker::at(&origin(&url), key()).unwrap();

    let shelf = broker.shelf("acme").unwrap();

    // Offering to connect is the one action that cannot make an unknown
    // state worse, and a row drawn as failed would be a lie about a
    // state this build simply does not know.
    assert_eq!(shelf[0].standing, Connection::Absent);
    drop(handle.join());
}
