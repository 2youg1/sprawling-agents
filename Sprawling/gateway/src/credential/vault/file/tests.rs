// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::let_underscore_untyped,
    reason = "test code"
)]

use super::*;

fn passphrase(text: &str) -> Zeroizing<String> {
    Zeroizing::new(text.to_owned())
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("vault-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("credentials.vault")
}

#[test]
fn a_stored_credential_returns_and_a_wrong_passphrase_is_refused() {
    let path = scratch("roundtrip");
    let _ = std::fs::remove_file(&path);
    let reference = SecretRef::parse("secret:anthropic/key").unwrap();

    let value = ["sk", "-vault", "-sample", "-value"].concat();

    let mut vault = FileVault::open(path.clone(), &passphrase("open sesame")).unwrap();
    vault
        .put(&reference, Zeroizing::new(value.clone()))
        .unwrap();
    assert_eq!(
        vault.get(&reference).unwrap().map(|v| v.to_string()),
        Some(value.clone())
    );

    let on_disk = String::from_utf8(std::fs::read(&path).unwrap()).unwrap();
    assert!(
        !on_disk.contains(&value),
        "the file a grep reaches holds ciphertext"
    );

    let reopened = FileVault::open(path.clone(), &passphrase("open sesame")).unwrap();
    assert_eq!(
        reopened.get(&reference).unwrap().map(|v| v.to_string()),
        Some(value)
    );

    let refused = FileVault::open(path.clone(), &passphrase("wrong"))
        .err()
        .expect("a wrong passphrase is refused rather than opening an empty vault");
    assert_eq!(refused.code(), &AxCode::ConfigInvalid);
    assert!(refused.recovery().contains("passphrase"), "{refused:?}");

    assert!(!refused.recovery().contains("  "), "{refused:?}");
    assert_eq!(
        String::from_utf8(std::fs::read(&path).unwrap()).unwrap(),
        on_disk
    );

    let mut vault = FileVault::open(path.clone(), &passphrase("open sesame")).unwrap();
    vault.delete(&reference).unwrap();
    assert!(vault.get(&reference).unwrap().is_none());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn a_record_moved_to_another_name_does_not_open() {
    let path = scratch("aad");
    let _ = std::fs::remove_file(&path);
    let here = SecretRef::parse("secret:anthropic/key").unwrap();
    let there = SecretRef::parse("secret:openai/key").unwrap();
    let mut vault = FileVault::open(path.clone(), &passphrase("pass")).unwrap();
    vault
        .put(&here, Zeroizing::new("value-here".to_owned()))
        .unwrap();
    let record = vault.entries.get(&here.to_string()).unwrap().clone();
    vault.entries.insert(there.to_string(), record);
    let moved = vault.get(&there).unwrap_err();
    assert_eq!(moved.code(), &AxCode::ConfigInvalid);
    let _ = std::fs::remove_file(&path);
}
