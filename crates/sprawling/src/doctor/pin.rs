// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The versions this repository pins, read from the files that pin
//! them (sprawling-SPEC.md section 8-120).
//!
//! Both files are compiled in, so changing a pinned version is an edit
//! to that one file and this binary reports the new pin on its next
//! build rather than a copy somebody forgot to update.

/// The file in this repository that pins an item's version.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Pin {
    /// Nothing in this repository states which version to use.
    Unpinned,
    /// `rust-toolchain.toml`, the `channel` rustup reads.
    RustToolchain,
    /// `lean-toolchain`, the toolchain elan reads.
    LeanToolchain,
}

const RUST_TOOLCHAIN: &str = include_str!("../../../../rust-toolchain.toml");

/// The whole line elan reads, `leanprover/lean4:v<version>`; the table
/// installs and detects by it.
pub(crate) const LEAN_TOOLCHAIN: &str = include_str!("../../../../lean-toolchain").trim_ascii_end();

/// The version `pin` names, as a dotted number; `None` for an unpinned
/// item, or for a file whose shape no longer carries one.
pub(crate) fn pinned(pin: Pin) -> Option<String> {
    match pin {
        Pin::Unpinned => None,
        Pin::RustToolchain => rust_channel(RUST_TOOLCHAIN),
        Pin::LeanToolchain => lean_version(LEAN_TOOLCHAIN),
    }
}

/// The value of the first `channel = "…"` line.
fn rust_channel(file: &str) -> Option<String> {
    file.lines()
        .filter_map(|line| line.trim().strip_prefix("channel"))
        .filter_map(|rest| rest.trim_start().strip_prefix('='))
        .map(|value| value.trim().trim_matches('"').to_owned())
        .find(|value| !value.is_empty())
}

/// What follows `:v` in `leanprover/lean4:v4.33.1`.
fn lean_version(line: &str) -> Option<String> {
    line.split_once(':')
        .map(|(_, version)| version.trim_start_matches('v').to_owned())
        .filter(|version| !version.is_empty())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;

    /// The pins are read out of the two files as they stand, whatever
    /// version those files name next.
    #[test]
    fn each_pin_is_the_version_its_file_names() {
        assert_eq!(
            (
                rust_channel("[toolchain]\nchannel = \"1.97.1\"\nprofile = \"minimal\"\n"),
                lean_version("leanprover/lean4:v4.33.1"),
                pinned(Pin::Unpinned),
            ),
            (Some("1.97.1".to_owned()), Some("4.33.1".to_owned()), None)
        );
        assert!(pinned(Pin::RustToolchain).is_some());
        assert!(LEAN_TOOLCHAIN.ends_with(&pinned(Pin::LeanToolchain).unwrap()));
    }
}
