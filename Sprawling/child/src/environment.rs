// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::ffi::OsString;
use std::process::Command;

pub const SECRET_PREFIX: &str = "SPRAWLING_SECRET_";

pub const PAIRING_TOKEN: &str = "SPRAWLING_PAIRING_TOKEN";

pub(crate) fn scrubbed(mut command: Command, inherited: impl Iterator<Item = OsString>) -> Command {
    for key in inherited.filter(|key| is_secret(key.as_encoded_bytes())) {
        command.env_remove(key);
    }
    command
}

fn is_secret(key: &[u8]) -> bool {
    same_name(key, PAIRING_TOKEN.as_bytes())
        || key
            .get(..SECRET_PREFIX.len())
            .is_some_and(|head| same_name(head, SECRET_PREFIX.as_bytes()))
}

#[cfg(windows)]
fn same_name(a: &[u8], b: &[u8]) -> bool {
    a.eq_ignore_ascii_case(b)
}

#[cfg(not(windows))]
fn same_name(a: &[u8], b: &[u8]) -> bool {
    a == b
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::disallowed_methods,
    reason = "test code"
)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn told(command: &Command) -> BTreeMap<String, Option<String>> {
        command
            .get_envs()
            .map(|(key, value)| {
                (
                    key.to_string_lossy().into_owned(),
                    value.map(|v| v.to_string_lossy().into_owned()),
                )
            })
            .collect()
    }

    #[test]
    fn every_secret_key_is_removed_and_nothing_else() {
        let inherited = [
            "PATH",
            "SPRAWLING_SECRET_ACME_KEY",
            "SPRAWLING_SECRET_",
            "SPRAWLING_SECRETS",
            "SPRAWLING_PAIRING_TOKEN",
            "SPRAWLING_PROVIDER",
            "sprawling_secret_acme_other",
        ]
        .map(OsString::from);
        let command = scrubbed(Command::new("program"), inherited.into_iter());
        let mut removed: BTreeMap<String, Option<String>> = [
            "SPRAWLING_SECRET_ACME_KEY",
            "SPRAWLING_SECRET_",
            "SPRAWLING_PAIRING_TOKEN",
        ]
        .into_iter()
        .map(|key| (key.to_owned(), None))
        .collect();

        if cfg!(windows) {
            removed.insert("sprawling_secret_acme_other".to_owned(), None);
        }
        assert_eq!(told(&command), removed);
    }
}
