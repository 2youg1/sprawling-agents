// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::net::SocketAddr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Keying {

    Adopt,

    MintUnshown,

    Mint,
}

impl Keying {

    pub(crate) fn decide(bind: SocketAddr, configured: bool) -> Self {
        match (configured, bind.ip().is_loopback()) {
            (true, _) => Self::Adopt,
            (false, true) => Self::MintUnshown,
            (false, false) => Self::Mint,
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::Keying;

    fn at(raw: &str) -> std::net::SocketAddr {
        raw.parse().unwrap()
    }

    #[test]
    fn the_four_cells_are_these_four() {
        assert_eq!(
            Keying::decide(at("127.0.0.1:2501"), false),
            Keying::MintUnshown
        );
        assert_eq!(Keying::decide(at("127.0.0.1:2501"), true), Keying::Adopt);
        assert_eq!(Keying::decide(at("0.0.0.0:2501"), true), Keying::Adopt);
        assert_eq!(Keying::decide(at("0.0.0.0:2501"), false), Keying::Mint);
    }

    #[test]
    fn an_address_beyond_this_machine_never_ends_up_without_a_key() {
        for raw in ["0.0.0.0:2501", "192.168.1.10:2501", "[::]:2501"] {
            assert_eq!(
                Keying::decide(at(raw), false),
                Keying::Mint,
                "{raw} reaches past this machine"
            );
        }
    }

    #[test]
    fn the_other_spelling_of_loopback_is_also_loopback() {
        assert_eq!(Keying::decide(at("[::1]:2501"), false), Keying::MintUnshown);
    }
}
