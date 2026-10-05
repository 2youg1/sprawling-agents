// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

#[test]
fn an_identical_digest_outside_locked_remains_a_complete_finding() {
    let root = std::env::temp_dir().join(format!("secret-nix-location-{}", std::process::id()));
    crate::root::fixture::write(
        &root,
        "Cargo.toml",
        "[workspace]\nmembers = []\nresolver = \"3\"\n",
    );
    let hash = [
        "sha256-",
        "Ab1Cd2Ef3G",
        "h4Ij5Kl6Mn",
        "7Op8Qr9St0",
        "Uv1Wx2Yz3Abc",
        "A=",
    ]
    .concat();
    assert_eq!(hash.len(), 51);
    let lock = serde_json::json!({
        "narHash": hash,
        "extra": {"narHash": hash, "zz": 0},
        "nodes": {"input": {"locked": {
            "extra": {"narHash": hash}, "extra": {"narHash": hash}, "narHash": hash, "owner": "example", "repo": "input",
            "rev": "a".repeat(40), "type": "github"
        }}, "root": {}},
        "root": "root", "version": 7
    });
    let body = format!("{}\n", serde_json::to_string_pretty(&lock).unwrap());
    let starts: Vec<_> = body.match_indices(&hash).map(|(start, _)| start).collect();
    assert_eq!(starts.len(), 4);
    crate::root::fixture::write(&root, "flake.lock", &body);
    let found = check(&root).unwrap();
    let actual: Vec<_> = found.iter().map(|v| format!("{v:?}")).collect();
    let expected: Vec<_> = starts.iter().take(3).map(|start| {
        format!("{:?}", Violation {
            gate: "secret", location: format!("flake.lock:byte {start}"),
            rule: "no secret shape may live in the repository (C13)".to_owned(),
            violation: "high-entropy token, 50 bytes".to_owned(),
            alternative: "replace the value with a secret:<realm>/<name> reference; if it is a scanner self-test sample, assemble it at runtime from short fragments".to_owned(),
        })
    }).collect();
    assert_eq!(actual, expected);
    std::fs::remove_dir_all(root).unwrap();
}
