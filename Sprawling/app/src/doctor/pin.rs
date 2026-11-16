// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The versions this repository pins, read from the files that pin
//! them (`crates/sprawling/spec/Doctor.lean` §8-120 and §8-157).
//!
//! The build script reads the files from the checkout this binary is
//! built in, so changing a pinned version is an edit to that one file
//! and this binary reports the new pin on its next build rather than a
//! copy somebody forgot to update. None of the files is inside this
//! package, so a build from its crates.io archive finds none of them:
//! each then reads as empty, and an empty file pins nothing.

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

// RUST_TOOLCHAIN_FILE, LEAN_TOOLCHAIN_FILE and ZIG_VERSION_FILE: each
// pin file's whole text as the build script found it, or empty.
include!(concat!(env!("OUT_DIR"), "/pins.rs"));

/// The whole line elan reads, `leanprover/lean4:v<version>`; the table
/// installs and detects by it. Empty in a build that found no pin.
pub(crate) const LEAN_TOOLCHAIN: &str = LEAN_TOOLCHAIN_FILE.trim_ascii_end();

/// The Zig version `crates/desktop/ffi/zig-version` pins, the one file
/// the leaf's build script and CI's install step read too
/// (`crates/sprawling/spec/Doctor.lean` §8-146). Empty in a build that found no pin.
pub(crate) const ZIG_VERSION: &str = ZIG_VERSION_FILE.trim_ascii_end();

/// The version `pin` names, as a dotted number; `None` for an unpinned
/// item, for a file this build did not find, or for a file whose shape
/// no longer carries one.
pub(crate) fn pinned(pin: Pin) -> Option<String> {
    match pin {
        Pin::Unpinned => None,
        Pin::RustToolchain => rust_channel(RUST_TOOLCHAIN_FILE),
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

    /// The pins are read out of the files as they stand, whatever
    /// version those files name next; a file this build did not find is
    /// empty, and an empty file pins nothing.
    #[test]
    fn each_pin_is_the_version_its_file_names() {
        assert_eq!(
            (
                rust_channel("[toolchain]\nchannel = \"1.97.1\"\nprofile = \"minimal\"\n"),
                lean_version("leanprover/lean4:v4.33.1"),
                pinned(Pin::Unpinned),
                (rust_channel(""), lean_version("")),
            ),
            (
                Some("1.97.1".to_owned()),
                Some("4.33.1".to_owned()),
                None,
                (None, None)
            )
        );
        assert!(pinned(Pin::RustToolchain).is_some());
        assert!(LEAN_TOOLCHAIN.ends_with(&pinned(Pin::LeanToolchain).unwrap()));
        assert!(!ZIG_VERSION.is_empty() && !ZIG_VERSION.ends_with('\n'));
    }
}
