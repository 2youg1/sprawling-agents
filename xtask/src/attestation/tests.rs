// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::findings;

const ATTESTED: &str = "jobs:
  archive:
    runs-on: ubuntu-latest
  publish:
    permissions:
      contents: write
      id-token: write
      attestations: write
    steps:
      - uses: actions/attest-build-provenance@v3
        with:
          subject-path: assets/*.zip
      - name: attach the archives to the tag
        run: |
          gh release create \"${GITHUB_REF_NAME}\" assets/*.zip \
            --prerelease
  channel:
    permissions:
      id-token: write
";

/// A job that attests the glob it attaches, with both permissions,
/// before it attaches it, is the shape every release must have.
#[test]
fn a_release_that_attests_what_it_attaches_is_accepted() {
    assert_eq!(findings(ATTESTED), Vec::<String>::new());
}

/// Each way the release can attach an archive nobody attested is named:
/// a permission another job holds does not count for this one, and an
/// attestation written after `gh release create` leaves a window.
#[test]
fn a_release_that_attaches_an_unattested_archive_is_reported() {
    let without_permission = ATTESTED.replacen("      attestations: write\n", "", 1);
    let wrong_glob = ATTESTED.replace("subject-path: assets/*.zip", "subject-path: dist/*.zip");
    let (head, tail) = ATTESTED.split_once("      - uses: actions/attest").unwrap();
    let (step, rest) = tail.split_once("      - name: attach").unwrap();
    let (attach, channel) = rest.split_once("  channel:").unwrap();
    let late = format!(
        "{head}      - name: attach{attach}      - uses: actions/attest{step}  channel:{channel}"
    );
    let missing = ATTESTED.replace("actions/attest-build-provenance@v3", "actions/checkout@v7");
    for text in [&without_permission, &wrong_glob, &late, &missing] {
        assert_eq!(findings(text).len(), 1, "{text}");
    }
    assert_eq!(findings("jobs:\n  build:\n    runs-on: x\n").len(), 1);
}
