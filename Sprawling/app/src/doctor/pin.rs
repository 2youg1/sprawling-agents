// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Pin {

    Unpinned,

    RustToolchain,

    LeanToolchain,
}

include!(concat!(env!("OUT_DIR"), "/pins.rs"));

pub(crate) const LEAN_TOOLCHAIN: &str = LEAN_TOOLCHAIN_FILE.trim_ascii_end();

pub(crate) const ZIG_VERSION: &str = ZIG_VERSION_FILE.trim_ascii_end();

pub(crate) fn pinned(pin: Pin) -> Option<String> {
    match pin {
        Pin::Unpinned => None,
        Pin::RustToolchain => rust_channel(RUST_TOOLCHAIN_FILE),
        Pin::LeanToolchain => lean_version(LEAN_TOOLCHAIN),
    }
}

fn rust_channel(file: &str) -> Option<String> {
    file.lines()
        .filter_map(|line| line.trim().strip_prefix("channel"))
        .filter_map(|rest| rest.trim_start().strip_prefix('='))
        .map(|value| value.trim().trim_matches('"').to_owned())
        .find(|value| !value.is_empty())
}

fn lean_version(line: &str) -> Option<String> {
    line.split_once(':')
        .map(|(_, version)| version.trim_start_matches('v').to_owned())
        .filter(|version| !version.is_empty())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;

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
