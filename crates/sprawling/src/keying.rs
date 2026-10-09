// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Where the key one serve demands comes from (`crates/sprawling/spec/Keying.lean`
//! section 8-22).
//!
//! Pure, and pure on purpose: the entropy a minted key is made of is
//! drawn in `bin::serving::door`, which is where this crate draws
//! randomness, and `wire::auth` documents that arrangement from the
//! other side ("Entropy arrives as a parameter. This module samples
//! nothing"). What is left here is the policy, which is four cells over
//! two facts and therefore something a test can state in full.
//!
//! Every cell has a key: the loopback port is no credential (wire D54),
//! so `wire::decide_bind`'s refusal of a keyless face is never reached
//! from here.

use std::net::SocketAddr;

/// Where this serve's key comes from.
///
/// Exhaustive over the two facts that decide it - whether the address
/// reaches beyond this machine, and whether the operator configured a
/// token - because a key nobody may see and a key a person must be
/// shown carry different obligations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Keying {
    /// The operator configured one. It is adopted as it stands and never
    /// displayed: we did not mint it, so printing it would only copy a
    /// standing secret into one more place that keeps text.
    Adopt,
    /// A loopback listener with nothing configured. A key is minted for
    /// this serve and shown to nobody: native clients read it from the key
    /// file, and browsers pair for a session token instead.
    MintUnshown,
    /// Nothing was configured and the address reaches past this machine.
    /// One key is minted for this serve, shown once, and forgotten when
    /// the process ends - a browser on another machine cannot pair
    /// without `crypto.subtle`, so it presents this key instead.
    Mint,
}

impl Keying {
    /// The four cells, with the configured value taking precedence
    /// wherever it exists.
    ///
    /// A configured token is honoured even on loopback so that a person
    /// can rehearse the exposed setup locally; the alternative silently
    /// ignores what they configured and then behaves differently the one
    /// time it matters.
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

    /// The whole policy, stated as the table it is. Written out cell by
    /// cell rather than as two branches, because the value of an
    /// exhaustive enum is that the reader can see every case at once.
    #[test]
    fn the_four_cells_are_these_four() {
        assert_eq!(
            Keying::decide(at("127.0.0.1:8787"), false),
            Keying::MintUnshown
        );
        assert_eq!(Keying::decide(at("127.0.0.1:8787"), true), Keying::Adopt);
        assert_eq!(Keying::decide(at("0.0.0.0:8787"), true), Keying::Adopt);
        assert_eq!(Keying::decide(at("0.0.0.0:8787"), false), Keying::Mint);
    }

    /// An address reachable from elsewhere with nothing configured grows
    /// a key that is shown, so a browser on another machine can present it.
    #[test]
    fn an_address_beyond_this_machine_never_ends_up_without_a_key() {
        for raw in ["0.0.0.0:8787", "192.168.1.10:8787", "[::]:8787"] {
            assert_eq!(
                Keying::decide(at(raw), false),
                Keying::Mint,
                "{raw} reaches past this machine"
            );
        }
    }

    /// IPv6 loopback is loopback. Spelling the check as `is_loopback`
    /// rather than comparing against `127.0.0.1` is what makes this true
    /// without a second branch.
    #[test]
    fn the_other_spelling_of_loopback_is_also_loopback() {
        assert_eq!(Keying::decide(at("[::1]:8787"), false), Keying::MintUnshown);
    }
}
