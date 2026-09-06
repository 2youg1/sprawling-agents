// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! How a city is stood up and served, as opposed to how one piece of
//! work is run.
//!
//! Four things happen here and nothing else: the key this listener will
//! present at its door is settled before a socket exists, the vault is
//! opened and asked what it really is, the one writer thread is started
//! with the ledger inside it, and the socket is handed the four sinks it
//! may reach the city through.
//!
//! **The writer thread is the city's one writer.** The ledger is opened
//! inside it and never leaves, so the type never has to cross a thread
//! boundary to prove that a city has one writer (ARCHITECTURE section
//! 10). Everything a socket does reaches it as a `Command` on a desk,
//! one at a time.
//!
//! Randomness is drawn here rather than in `bin::keying`, which is pure:
//! this crate draws entropy in one place, and a key a third party can
//! predict is a door a third party can open.

use kernel::{AxCode, AxError, Payload};

/// A URL-safe random string of `bytes` bytes of OS entropy.
///
/// Deliberately not the simulator's seeded randomness: a verifier a
/// third party can predict is a login a third party can finish. This is
/// the one place in the binary where reproducibility would be a defect.
pub(crate) fn random_token(bytes: usize) -> Result<String, AxError> {
    let mut raw = vec![0u8; bytes];
    getrandom::fill(&mut raw).map_err(|err| {
        AxError::failure(
            AxCode::ConfigInvalid,
            "draw randomness for a login",
            err.to_string(),
        )
        .with_recovery("this machine's entropy source refused; no login can be started safely")
    })?;
    // The alphabet belongs to the flow that consumes it, and a copy of
    // it here would be both a second authority and - being sixty-four
    // mixed characters at rest - exactly the shape the secret scanner
    // hunts for.
    Ok(gateway::oauth_random(&raw))
}

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
/// the reason `channels::auth` states from its own side — that module
/// samples nothing, and the assembly layer is where randomness in this
/// binary comes from.
///
/// The result is what `channels::decide_bind` will be asked about, so an
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
            let (_token, code) = channels::PairingToken::mint(entropy);
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
