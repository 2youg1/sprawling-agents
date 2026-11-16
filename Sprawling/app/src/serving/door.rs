// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError, Payload};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Keyed {
    Unshown(String),

    Adopted(String),

    Minted(String),
}

impl Keyed {
    #[must_use]
    pub fn code(&self) -> &str {
        match self {
            Self::Unshown(code) | Self::Adopted(code) | Self::Minted(code) => code,
        }
    }

    #[must_use]
    pub fn shown(&self) -> Option<&str> {
        match self {
            Self::Minted(code) => Some(code),
            Self::Unshown(_) | Self::Adopted(_) => None,
        }
    }
}

pub fn key_for(bind: std::net::SocketAddr, configured: Option<String>) -> Result<Keyed, AxError> {
    match crate::keying::Keying::decide(bind, configured.is_some()) {
        crate::keying::Keying::Adopt => configured.map(Keyed::Adopted).ok_or_else(|| {
            AxError::failure(
                AxCode::ConfigInvalid,
                "adopt the configured pairing token",
                "a token was decided on and then was not there",
            )
            .with_recovery("report this: keying::Keying::decide and key_for disagree")
        }),
        crate::keying::Keying::MintUnshown => minted().map(Keyed::Unshown),
        crate::keying::Keying::Mint => minted().map(Keyed::Minted),
    }
}

fn minted() -> Result<String, AxError> {
    let mut entropy = [0u8; 32];
    getrandom::fill(&mut entropy).map_err(|err| {
        AxError::failure(
            AxCode::ConfigInvalid,
            "draw randomness for the city's key",
            err.to_string(),
        )
        .with_recovery("this machine's entropy source refused; set SPRAWLING_PAIRING_TOKEN")
    })?;

    let (_token, code) = wire::PairingToken::mint(entropy);
    Ok(code)
}

#[must_use]
pub fn open_vault() -> (gateway::Custodian, Option<Payload>) {
    gateway::Custodian::probe()
}
