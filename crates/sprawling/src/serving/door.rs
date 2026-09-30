// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a listener presents at its door, settled before a socket exists.
//!
//! Two questions are answered here and nothing else: which pairing key
//! this serve will carry ([`key_for`]), and what the credential vault on
//! this machine really is ([`open_vault`]). Both are answered while
//! nothing is bound yet, so there is no moment in which the port is open
//! and unauthenticated.
//!
//! [`Keyed`] is an enum rather than an `Option<String>` beside a flag
//! because its three cases carry different obligations, and only a key
//! minted for this serve may be shown to a person. Which case applies is
//! decided by `crate::keying`, which stays pure; this module is where
//! the entropy behind a minted key is drawn, so the binary samples
//! randomness in one place and a key a third party can predict is a door
//! a third party can open.
//!
//! The point a reader most often gets wrong: [`open_vault`] reports what
//! a probe of write, read and delete found, not what configuration
//! asked for: a vault that silently forgets across a restart is
//! disclosed in the ledger at startup instead of surfacing much later as
//! an egress failure.

use kernel::{AxCode, AxError, Payload};

/// What this serve will present at its door, and whether a person has
/// to be shown it.
///
/// The three cases carry different obligations, so they are an enum
/// rather than an `Option<String>` plus a flag: only [`Self::Minted`] is
/// shown, and only because nobody else has ever seen it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Keyed {
    /// A loopback listener. Nothing is presented and nothing is shown.
    NothingToPresent,
    /// What the operator configured, adopted as it stands.
    Adopted(String),
    /// Minted for this serve from this machine's entropy. Shown once,
    /// stored nowhere, and dead when the process ends.
    Minted(String),
}

impl Keyed {
    /// The code the listener and the console both carry, if there is one.
    #[must_use]
    pub fn code(&self) -> Option<&str> {
        match self {
            Self::NothingToPresent => None,
            Self::Adopted(code) | Self::Minted(code) => Some(code),
        }
    }
}

/// Settles this serve's pairing key before anything is bound.
///
/// [`crate::keying::Keying`] decides which of the three cases applies;
/// this performs the one that needs entropy. Minting happens here for
/// the reason `wire::auth` states from its own side — that module
/// samples nothing, and the assembly layer is where randomness in this
/// binary comes from.
///
/// The result is what `wire::decide_bind` will be asked about, so an
/// address reaching beyond this machine arrives at the socket already
/// carrying a key. The guard is satisfied, never relaxed: there is still
/// no moment in which the port is open and unauthenticated.
///
/// # Errors
/// When this machine's entropy source refuses. No key can be minted
/// safely then, and serving anyway would open the port without one.
pub fn key_for(bind: std::net::SocketAddr, configured: Option<String>) -> Result<Keyed, AxError> {
    match crate::keying::Keying::decide(bind, configured.is_some()) {
        crate::keying::Keying::NothingToPresent => Ok(Keyed::NothingToPresent),
        // Refused rather than unwrapped: `decide` answers `Adopt` only
        // when the value is there, and a mismatch between those two would
        // be a defect in this file worth naming out loud.
        crate::keying::Keying::Adopt => configured.map(Keyed::Adopted).ok_or_else(|| {
            AxError::failure(
                AxCode::ConfigInvalid,
                "adopt the configured pairing token",
                "a token was decided on and then was not there",
            )
            .with_recovery("report this: keying::Keying::decide and key_for disagree")
        }),
        crate::keying::Keying::Mint => {
            let mut entropy = [0u8; 32];
            getrandom::fill(&mut entropy).map_err(|err| {
                AxError::failure(
                    AxCode::ConfigInvalid,
                    "draw randomness for a pairing key",
                    err.to_string(),
                )
                .with_recovery(
                    "this machine's entropy source refused; set SPRAWLING_PAIRING_TOKEN \
                     or bind a loopback address",
                )
            })?;
            // The token half is dropped here on purpose. `serve` derives
            // the digest it compares against from this same code through
            // `from_configured`, so holding both would be two paths to
            // one digest and a second thing to keep in step.
            let (_token, code) = wire::PairingToken::mint(entropy);
            Ok(Keyed::Minted(code))
        }
    }
}

/// Opens the credential vault and says what it really is.
///
/// The probe writes, reads and deletes once; whatever backend survives
/// that is the one in use, and the notice it returns is the disclosure
/// that goes in the ledger. A vault that silently forgets across a
/// restart would turn one configuration act into a later egress failure,
/// far from its cause.
#[must_use]
pub fn open_vault() -> (gateway::Custodian, Option<Payload>) {
    gateway::Custodian::probe()
}
