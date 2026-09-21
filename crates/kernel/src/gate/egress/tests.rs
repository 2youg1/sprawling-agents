// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
#[test]
fn an_allowlist_matches_on_label_boundaries_not_on_text() {
    let list = EgressAllowlist::new(vec![".Example.com".to_owned(), "docs.rs".to_owned()]);
    assert!(list.admits("example.com"));
    assert!(list.admits("api.example.com"));
    assert!(list.admits("EXAMPLE.COM"), "hosts are case-insensitive");
    assert!(
        !list.admits("notexample.com"),
        "a suffix test would allow this"
    );
    assert!(!list.admits("example.com.evil.test"));
}

#[test]
fn a_host_outside_the_list_is_refused_in_three_parts_that_name_the_way_out() {
    let list = EgressAllowlist::new(vec!["example.com".to_owned()]);
    let target = EgressTarget::Public {
        host: "pastebin.test".to_owned(),
    };
    let EgressOutcome::Deny { refusal } = egress_target(&list, &target) else {
        panic!("an unlisted host is refused");
    };
    assert!(refusal.to_string().contains("pastebin.test"));
    let gate = refusal.gate().unwrap();
    assert!(gate.alternative().contains("example.com"));
}

#[test]
fn an_empty_list_reaches_nothing_public_and_says_what_to_do_instead() {
    let list = EgressAllowlist::default();
    let EgressOutcome::Deny { refusal } = egress_target(
        &list,
        &EgressTarget::Public {
            host: "example.com".to_owned(),
        },
    ) else {
        panic!("a building with no list reaches nothing public");
    };
    let gate = refusal.gate().unwrap();
    assert!(gate.alternative().contains("building that has one"));
}

#[test]
fn local_and_private_targets_are_not_egress_at_all() {
    let list = EgressAllowlist::default();
    assert!(matches!(
        egress_target(&list, &EgressTarget::Loopback),
        EgressOutcome::Allow { .. }
    ));
    assert!(
        matches!(
            egress_target(&list, &EgressTarget::Private),
            EgressOutcome::Allow { .. }
        ),
        "a confidential building must still reach its own inference server"
    );
}

#[test]
fn the_egress_door_denies_spans_and_flags_the_first_public_hop() {
    let span = SecretSpan {
        start: 10,
        len: 40,
        provider: Some("anthropic"),
    };
    let outcome = egress(
        std::slice::from_ref(&span),
        &EgressTarget::Public {
            host: "api.example.com".into(),
        },
        false,
    );
    match outcome {
        EgressOutcome::Deny { refusal } => {
            assert_eq!(refusal.code(), &AxCode::SecretEgress);
            assert!(refusal.subject().contains("10+40"));
            assert!(!refusal.subject().contains("sk-ant"), "never echo bytes");
            assert!(refusal.gate().unwrap().alternative().contains("rotation"));
        }
        EgressOutcome::Allow { .. } => panic!("spans must deny"),
    }
    match egress(
        &[],
        &EgressTarget::Public {
            host: "x.dev".into(),
        },
        false,
    ) {
        EgressOutcome::Allow {
            first_public_egress,
        } => assert!(first_public_egress),
        EgressOutcome::Deny { .. } => panic!("clean egress allows"),
    }
    match egress(
        &[],
        &EgressTarget::Public {
            host: "x.dev".into(),
        },
        true,
    ) {
        EgressOutcome::Allow {
            first_public_egress,
        } => assert!(!first_public_egress),
        EgressOutcome::Deny { .. } => panic!(),
    }
    match egress(&[], &EgressTarget::Loopback, false) {
        EgressOutcome::Allow {
            first_public_egress,
        } => assert!(!first_public_egress, "localhost is not the internet"),
        EgressOutcome::Deny { .. } => panic!(),
    }
}

#[test]
fn a_loopback_or_private_host_is_not_the_internet() {
    for host in [
        "127.0.0.1",
        "127.9.9.9",
        "localhost",
        "[::1]",
        "::1",
        "127.0.0.1.",
    ] {
        assert_eq!(target_of(host), EgressTarget::Loopback, "{host}");
    }
    for host in [
        "10.1.2.3",
        "192.168.0.9",
        "172.16.0.1",
        "172.31.255.1",
        "169.254.1.1",
        "fc00::1",
        "fe80::1",
    ] {
        assert_eq!(target_of(host), EgressTarget::Private, "{host}");
    }
    assert_eq!(
        target_of("172.32.0.1"),
        EgressTarget::Public {
            host: "172.32.0.1".to_owned()
        },
        "just outside the private range"
    );
    assert_eq!(
        target_of("example.com"),
        EgressTarget::Public {
            host: "example.com".to_owned()
        }
    );
    assert_eq!(
        target_of("1270.0.0.1"),
        EgressTarget::Public {
            host: "1270.0.0.1".to_owned()
        },
        "a name that looks numerical is not an address"
    );
}

#[test]
fn host_of_reads_the_authority_and_nothing_else() {
    assert_eq!(
        host_of("https://example.com/a?b#c").unwrap(),
        Some("example.com".to_owned())
    );
    assert_eq!(
        host_of("http://user:pw@EXAMPLE.com:8080/x").unwrap(),
        Some("example.com".to_owned()),
        "userinfo and port are not the host"
    );
    assert_eq!(
        host_of("ws://[::1]:9222/session").unwrap(),
        Some("::1".to_owned())
    );
    assert_eq!(host_of("about:blank").unwrap(), None);
    assert_eq!(
        host_of("file:///tmp/page.html").unwrap(),
        None,
        "an empty authority names no host"
    );
    assert!(host_of("http://[::1").is_err(), "an unclosed bracket");
    assert!(host_of("http://host:not-a-port/").is_err());
}
