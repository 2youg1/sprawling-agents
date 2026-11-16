// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use aws_lc_rs::agreement::{self, PrivateKey, UnparsedPublicKey, X25519};
use aws_lc_rs::digest::{SHA256, digest};
use aws_lc_rs::hkdf::{HKDF_SHA256, Salt};
use aws_lc_rs::kem::{Ciphertext, DecapsulationKey, EncapsulationKey, ML_KEM_768, SharedSecret};
use kernel::{AxCode, AxError};

use super::Session;
use crate::keys::{Length, crypto_failure};
use crate::seal::{Direction, Opener, Sealer};

#[cfg(test)]
mod interop;

pub(super) struct Keys {
    device_to_city: [u8; 32],
    city_to_device: [u8; 32],
}

pub(super) struct Ephemeral {
    x25519: PrivateKey,
    ml_kem: DecapsulationKey,
}

pub(super) struct Answer {
    pub(super) public: Vec<u8>,
    x_secret: [u8; 32],
    kem_secret: SharedSecret,
}

impl Ephemeral {
    pub(super) fn generate() -> Result<(Self, Vec<u8>), AxError> {
        let x25519 =
            PrivateKey::generate(&X25519).map_err(|_| crypto_failure("generate an X25519 key"))?;
        let ml_kem = DecapsulationKey::generate(&ML_KEM_768)
            .map_err(|_| crypto_failure("generate an ML-KEM key"))?;
        let x_public = x25519
            .compute_public_key()
            .map_err(|_| crypto_failure("derive an X25519 public key"))?;
        let kem_public = ml_kem
            .encapsulation_key()
            .and_then(|key| key.key_bytes())
            .map_err(|_| crypto_failure("derive an ML-KEM encapsulation key"))?;
        let public = [x_public.as_ref(), kem_public.as_ref()].concat();
        Ok((Self { x25519, ml_kem }, public))
    }

    pub(super) fn keys(
        &self,
        transcript: &[u8; 32],
        x_peer: &[u8],
        ciphertext: &[u8],
    ) -> Result<Keys, AxError> {
        let x_secret = agree(&self.x25519, x_peer)?;
        let kem_secret = self
            .ml_kem
            .decapsulate(Ciphertext::from(ciphertext))
            .map_err(|_| refused("open the city's ML-KEM ciphertext"))?;
        derive(transcript, &x_secret, kem_secret.as_ref())
    }
}

impl Answer {
    pub(super) fn new(x_peer: &[u8], kem_peer: &[u8]) -> Result<Self, AxError> {
        let x25519 =
            PrivateKey::generate(&X25519).map_err(|_| crypto_failure("generate an X25519 key"))?;
        let x_public = x25519
            .compute_public_key()
            .map_err(|_| crypto_failure("derive an X25519 public key"))?;
        let x_secret = agree(&x25519, x_peer)?;
        let (ciphertext, kem_secret) = EncapsulationKey::new(&ML_KEM_768, kem_peer)
            .map_err(|_| refused("read the device's ML-KEM key"))?
            .encapsulate()
            .map_err(|_| crypto_failure("encapsulate to the device's ML-KEM key"))?;
        Ok(Self {
            public: [x_public.as_ref(), ciphertext.as_ref()].concat(),
            x_secret,
            kem_secret,
        })
    }

    pub(super) fn keys(&self, transcript: &[u8; 32]) -> Result<Keys, AxError> {
        derive(transcript, &self.x_secret, self.kem_secret.as_ref())
    }
}

impl Keys {
    pub(super) fn device_session(&self) -> Result<Session, AxError> {
        Ok(Session {
            sealer: Sealer::new(&self.device_to_city, Direction::DeviceToCity)?,
            opener: Opener::new(&self.city_to_device, Direction::CityToDevice)?,
        })
    }

    pub(super) fn city_session(&self) -> Result<Session, AxError> {
        Ok(Session {
            sealer: Sealer::new(&self.city_to_device, Direction::CityToDevice)?,
            opener: Opener::new(&self.device_to_city, Direction::DeviceToCity)?,
        })
    }
}

pub(super) fn record(label: &[u8], opening: &[u8], unsigned_answer: &[u8]) -> [u8; 32] {
    sha256(&[label, opening, unsigned_answer].concat())
}

pub(super) fn sha256(input: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    out.copy_from_slice(digest(&SHA256, input).as_ref());
    out
}

pub(super) fn signed(label: &[u8], role: &[u8], transcript: &[u8; 32]) -> Vec<u8> {
    [label, role, transcript].concat()
}

fn agree(mine: &PrivateKey, peer: &[u8]) -> Result<[u8; 32], AxError> {
    agreement::agree(mine, UnparsedPublicKey::new(&X25519, peer), (), |secret| {
        let mut out = [0u8; 32];
        if secret.len() != out.len() {
            return Err(());
        }
        out.copy_from_slice(secret);
        Ok(out)
    })
    .map_err(|()| refused("agree an X25519 secret"))
}

fn derive(transcript: &[u8; 32], x_secret: &[u8; 32], kem_secret: &[u8]) -> Result<Keys, AxError> {
    let prk =
        Salt::new(HKDF_SHA256, transcript).extract(&[x_secret.as_slice(), kem_secret].concat());
    let expand = |label: &'static [u8]| -> Result<[u8; 32], AxError> {
        let mut out = [0u8; 32];
        prk.expand(&[label], Length(32))
            .and_then(|okm| okm.fill(&mut out))
            .map_err(|_| crypto_failure("derive a session key"))?;
        Ok(out)
    };
    Ok(Keys {
        device_to_city: expand(b"device to city")?,
        city_to_device: expand(b"city to device")?,
    })
}

fn refused(action: &str) -> AxError {
    AxError::failure(AxCode::GateDenied, action, "the other side's key material")
        .with_recovery("start the connection again; if it repeats, pair the device again")
}
