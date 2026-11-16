// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! How a credential enters this city: enrolment.

use crate::worker::*;

#[test]
fn an_enrolled_credential_leaves_only_a_reference_in_the_history() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();
    // Assembled at runtime: a credential-shaped literal is what the
    // secret gate keeps out of the repository.
    let token = ["sk-live-", "9f2c4a7e1b8d"].concat();
    worker
        .handle(wire::Command::PutSecret {
            realm: "house".to_owned(),
            name: "key".to_owned(),
            value: kernel::Sealed::new(Box::new(token.clone())),
        })
        .unwrap();

    let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
    let history: String = verified
        .raw_lines()
        .iter()
        .map(|line| String::from_utf8_lossy(line).into_owned())
        .collect::<Vec<String>>()
        .join("\n");
    assert!(history.contains("secret:house/key"));
    assert!(
        !history.contains(&token),
        "the ledger records where a credential lives, never what it is"
    );
    // And it is redeemable afterwards, which is the other half: a
    // vault that records the act without keeping the value would
    // fail later, far from here.
    let resolver = worker.resolver();
    let reference = kernel::SecretRef::parse("secret:house/key").unwrap();
    let redeemed = resolver(&reference).unwrap().into_vault_value();
    assert_eq!(redeemed.as_str(), token);
}
