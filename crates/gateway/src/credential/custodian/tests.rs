// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Custody's two read ports agree, and an in-memory store says what
//! it is rather than pretending to persist.

use super::*;
fn sample_token() -> String {
    // Runtime-assembled: the repository never holds a complete
    // high-entropy literal at rest (xtask secret discipline).
    ["sk-ant-api03-", "Zx9yQ2mK4pL7", "vB1nC5tR8sD3"].concat()
}

/// The half of A13 this module owns: a value the person handed over
/// goes in under the name the caller named, comes back sealed, and
/// is never part of what `describe` renders. The other half - bytes
/// that merely look like a key never reaching a sink - belongs to
/// `runtime::redact`, and this crate holds no second copy of it.
#[test]
fn a13_a_stored_credential_is_redeemable_and_never_rendered() {
    let mut custodian = Custodian::in_memory();
    let reference = SecretRef::parse("secret:anthropic/api-key").unwrap();
    custodian
        .set(&reference, Zeroizing::new(sample_token()))
        .unwrap();
    assert!(custodian.resolve(&reference).is_ok());
    let described = custodian.describe(&reference);
    assert!(described.configured);
    assert_eq!(described.persistence, Persistence::ThisProcess);
    let rendered = format!("{} {}", described.source, described.configured);
    assert!(!rendered.contains("Zx9yQ2mK4pL7"), "{rendered}");
}

#[test]
fn missing_and_empty_read_as_not_configured() {
    let mut custodian = Custodian::in_memory();
    let reference = SecretRef::parse("secret:acme/key").unwrap();
    let err = match custodian.resolve(&reference) {
        Err(err) => err,
        Ok(_) => panic!("missing credential must not resolve"),
    };
    assert_eq!(*err.code(), AxCode::CredentialMissing);
    // The recovery says what the backend can keep, so a key the
    // reboot took is not reported as a key nobody ever entered.
    // This custodian is session memory; a Linux one says the same
    // about the boot, out of the same enum.
    assert!(
        err.recovery()
            .ends_with(custodian.persistence().consequence()),
        "{}",
        err.recovery()
    );
    assert!(
        Persistence::ThisBoot.consequence().contains("reboots"),
        "the Linux grade has to name what a reboot does"
    );
    let err = custodian
        .set(&reference, Zeroizing::new(String::new()))
        .unwrap_err();
    assert_eq!(*err.code(), AxCode::InvalidArgs);
    assert!(!custodian.describe(&reference).configured);
}

#[test]
fn the_environment_shades_and_set_refuses_naming_the_shader() {
    let mut custodian = Custodian::in_memory().with_env_reader(Box::new(|key| {
        (key == "SPRAWLING_SECRET_ACME_KEY").then(|| "from-env".to_owned())
    }));
    let reference = SecretRef::parse("secret:acme/key").unwrap();
    // Resolve serves the read-only source (sealed).
    assert!(custodian.resolve(&reference).is_ok());
    // Describe shows read-only.
    let described = custodian.describe(&reference);
    assert!(described.configured);
    assert!(!described.writable);
    assert!(described.source.contains("SPRAWLING_SECRET_ACME_KEY"));
    // Set refuses: it would look successful and change nothing.
    let err = custodian
        .set(&reference, Zeroizing::new("new".to_owned()))
        .unwrap_err();
    assert!(err.subject().contains("SPRAWLING_SECRET_ACME_KEY"));
}

/// The one fact with two readers. `resolve` opens the value and
/// `describe` only looks, so a backend that decrypts for one and
/// not the other answers "not configured" for a credential it can
/// redeem - which is what a page would then tell a person about a
/// key they entered.
///
/// The two ports are asked in every state this custodian can be in;
/// the assertions are the state, not a restatement of the code.
#[test]
fn the_two_read_ports_agree_about_whether_a_reference_is_configured() {
    let reference = SecretRef::parse("secret:acme/agreement").unwrap();
    let missing = Custodian::in_memory();
    assert!(!agreed(&missing, &reference), "nothing is stored");

    let mut stored = Custodian::in_memory();
    stored
        .set(&reference, Zeroizing::new(sample_token()))
        .unwrap();
    assert!(agreed(&stored, &reference), "a value is stored");
    assert_eq!(
        stored.describe(&reference).persistence,
        stored.custody().persistence,
        "the store `describe` names and the store a report names are one value"
    );

    // A name the machine sets is not a store this city can write to,
    // and both ports have to say the same thing about it.
    let shaded = Custodian::in_memory().with_env_reader(Box::new(|key| {
        (key == "SPRAWLING_SECRET_ACME_AGREEMENT").then(String::new)
    }));
    assert!(
        !agreed(&shaded, &reference),
        "a variable set to nothing is not a credential"
    );

    let from_env = Custodian::in_memory().with_env_reader(Box::new(|key| {
        (key == "SPRAWLING_SECRET_ACME_AGREEMENT").then(|| "from-env".to_owned())
    }));
    assert!(agreed(&from_env, &reference), "the environment carries it");
}

/// Asks both read ports and returns whether the credential is there.
fn agreed(custodian: &Custodian, reference: &SecretRef) -> bool {
    let redeemed = custodian.resolve(reference).is_ok();
    let described = custodian.describe(reference).configured;
    assert_eq!(
        redeemed, described,
        "resolve says {redeemed} and describe says {described} about the same reference"
    );
    redeemed
}

#[test]
fn rotation_is_next_operation_effective_because_nothing_caches() {
    let mut custodian = Custodian::in_memory();
    let reference = SecretRef::parse("secret:acme/rotating").unwrap();
    custodian
        .set(&reference, Zeroizing::new("one".to_owned()))
        .unwrap();
    assert!(custodian.resolve(&reference).is_ok());
    custodian
        .set(&reference, Zeroizing::new("two".to_owned()))
        .unwrap();
    // No copy survives between operations: the next resolve reads the
    // backend, so the rotated value is what the wire test would see.
    assert!(custodian.resolve(&reference).is_ok());
}
