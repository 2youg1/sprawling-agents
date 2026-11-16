// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::account_recovery::AccountRetries;

use super::*;

#[test]
fn web_search_switches_to_the_next_account_at_once_when_a_key_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let worker = worker_holding(dir.path(), &["alpha", "beta"]);
    let (url, seen) = hosted("vendor_search", vendor_schema(), |sent| {
        match sent.key.as_deref() {
            Some("key-alpha") => Reply::Status(401),
            _ => Reply::Answer,
        }
    });
    let tool = offered(&worker, dir.path(), &vendor(&url, &["alpha", "beta"]));

    search(&tool, serde_json::json!({"query": "anything"})).unwrap();

    assert_eq!(
        calls_from(&seen, "key-alpha").len(),
        1,
        "alpha is asked once"
    );
    assert_eq!(
        calls_from(&seen, "key-beta")
            .iter()
            .filter(|s| s.method() == "tools/call")
            .count(),
        1,
        "and beta answers the call"
    );
}

#[test]
fn web_search_reports_every_account_rejected_as_one_exhausted_failure() {
    let dir = tempfile::tempdir().unwrap();
    let worker = worker_holding(dir.path(), &["alpha", "beta"]);
    let (url, seen) = hosted("vendor_search", vendor_schema(), |_| Reply::Status(401));
    let tool = offered(&worker, dir.path(), &vendor(&url, &["alpha", "beta"]));

    let failed = search(&tool, serde_json::json!({"query": "anything"})).unwrap_err();

    assert_eq!(failed.code(), &AxCode::ProviderAccountsExhausted);
    for named in ["vendor", "alpha", "beta"] {
        assert!(failed.subject().contains(named), "{}", failed.subject());
    }
    assert!(!failed.subject().contains("key-"), "{}", failed.subject());
    assert!(
        !failed.subject().contains("secret:"),
        "{}",
        failed.subject()
    );
    assert_eq!(seen.lock().unwrap().len(), 2, "each account is asked once");
}

#[test]
fn web_search_sends_a_lost_answer_again_only_on_the_same_account() {
    let dir = tempfile::tempdir().unwrap();
    let worker = worker_holding(dir.path(), &["alpha", "beta"]);
    let (url, seen) = hosted("vendor_search", vendor_schema(), |sent| {
        if sent.method() == "tools/call" {
            Reply::Unread
        } else {
            Reply::Answer
        }
    });
    let tool = offered(&worker, dir.path(), &vendor(&url, &["alpha", "beta"]));

    let failed = search(&tool, serde_json::json!({"query": "anything"})).unwrap_err();

    assert_eq!(failed.retry(), kernel::Retry::Unknown);
    let budget = match gateway::EndpointTuning::DEFAULTS.account_retries {
        AccountRetries::One => 1,
        AccountRetries::Two => 2,
    };
    assert_eq!(
        calls_from(&seen, "key-alpha")
            .iter()
            .filter(|s| s.method() == "tools/call")
            .count(),
        1 + budget
    );
    assert!(
        calls_from(&seen, "key-beta").is_empty(),
        "beta is never asked"
    );
}
