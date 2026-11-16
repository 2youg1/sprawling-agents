// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError};

use super::agreement::Answer;
use super::agreement::{Ephemeral, Keys, record, sha256, signed};
use super::{CITY_SIGNS, DEVICE_SIGNS, ML_KEM_CIPHERTEXT_BYTES, ML_KEM_KEY_BYTES, NONCE_BYTES};
use super::{Session, X25519_BYTES, fixed};
use crate::keys::{PUBLIC_BYTES, SIGNATURE_BYTES, Signature, SigningKey, VerifyingKey};
use crate::pairing::{CODE_BYTES, PairingCode, canonical, decode, encode, is_symbol};

#[cfg(test)]
mod tests;

pub const FINGERPRINT_BYTES: usize = 32;

pub const CODE_TEXT_BYTES: usize = (CODE_BYTES * 8).div_ceil(5);

pub const PAIR_HELLO_BYTES: usize = X25519_BYTES + ML_KEM_KEY_BYTES + NONCE_BYTES;

const UNSIGNED_PAIR_REPLY_BYTES: usize =
    PUBLIC_BYTES + X25519_BYTES + ML_KEM_CIPHERTEXT_BYTES + NONCE_BYTES;

pub const PAIR_REPLY_BYTES: usize = UNSIGNED_PAIR_REPLY_BYTES + SIGNATURE_BYTES;

pub const CLAIM_BYTES: usize = CODE_TEXT_BYTES + PUBLIC_BYTES + SIGNATURE_BYTES;

const PAIRING: &[u8] = b"sprawling remote pairing v1";

const SCAN_AGAIN: &str = "scan the code on the city's console again; if this repeats, the route is changing what it forwards";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CityFingerprint([u8; FINGERPRINT_BYTES]);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invitation {
    pub city: CityFingerprint,
    pub code: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairHello(Box<[u8; PAIR_HELLO_BYTES]>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairReply(Box<[u8; PAIR_REPLY_BYTES]>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Claim {
    code: PairingCode,
    device_key: VerifyingKey,
}

pub struct Claimed {
    pub sealed: Vec<u8>,
    pub city: VerifyingKey,
    pub session: Session,
}

pub struct DevicePairing {
    hello: PairHello,
    ephemeral: Ephemeral,
}

pub struct CityPairing {
    transcript: [u8; 32],
    keys: Keys,
}

impl CityFingerprint {

    #[must_use]
    pub fn of(city: &VerifyingKey) -> Self {
        Self(sha256(city.as_bytes()))
    }

    #[must_use]
    pub fn text(&self) -> String {
        encode(&self.0)
    }

    pub fn read(text: &str) -> Result<Self, AxError> {
        let lowered = text.trim().to_lowercase();
        match decode(&lowered).map(<[u8; FINGERPRINT_BYTES]>::try_from) {
            Some(Ok(bytes)) if encode(&bytes) == lowered => Ok(Self(bytes)),
            Some(Ok(_) | Err(_)) | None => Err(AxError::failure(
                AxCode::InvalidArgs,
                "read a city fingerprint",
                format!("{} characters", lowered.chars().count()),
            )
            .with_recovery("scan the code on the city's console again")),
        }
    }
}

impl PairHello {

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, AxError> {
        fixed(bytes, "read a pairing hello").map(|array| Self(Box::new(array)))
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8; PAIR_HELLO_BYTES] {
        &self.0
    }

    fn parts(&self) -> (&[u8], &[u8]) {
        let (x25519, rest) = self.0.split_at(X25519_BYTES);
        let (ml_kem, _) = rest.split_at(ML_KEM_KEY_BYTES);
        (x25519, ml_kem)
    }
}

impl PairReply {

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, AxError> {
        fixed(bytes, "read a pairing reply").map(|array| Self(Box::new(array)))
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8; PAIR_REPLY_BYTES] {
        &self.0
    }

    fn unsigned(&self) -> &[u8] {
        self.0.split_at(UNSIGNED_PAIR_REPLY_BYTES).0
    }

    fn parts(&self) -> [&[u8]; 4] {
        let (presented, rest) = self.0.split_at(PUBLIC_BYTES);
        let (x25519, rest) = rest.split_at(X25519_BYTES);
        let (ciphertext, rest) = rest.split_at(ML_KEM_CIPHERTEXT_BYTES);
        let (_, signature) = rest.split_at(NONCE_BYTES);
        [presented, x25519, ciphertext, signature]
    }
}

impl Claim {

    #[must_use]
    pub fn code(&self) -> PairingCode {
        self.code
    }

    #[must_use]
    pub fn device_key(&self) -> &VerifyingKey {
        &self.device_key
    }
}

pub fn device_pair_hello(nonce: [u8; NONCE_BYTES]) -> Result<DevicePairing, AxError> {
    let (ephemeral, public) = Ephemeral::generate()?;
    Ok(DevicePairing {
        hello: PairHello::from_bytes(&[public.as_slice(), &nonce].concat())?,
        ephemeral,
    })
}

impl DevicePairing {
    #[must_use]
    pub fn hello(&self) -> &PairHello {
        &self.hello
    }

    pub fn claim(
        self,
        reply: &PairReply,
        invitation: &Invitation,
        device_key: &SigningKey,
    ) -> Result<Claimed, AxError> {
        let code = code_text(&invitation.code)?;
        let [presented, x_peer, ciphertext, signature] = reply.parts();
        let city = VerifyingKey::from_bytes(presented)?;
        if CityFingerprint::of(&city) != invitation.city {
            return Err(unrecognised("the key the city presented"));
        }
        let transcript = record(PAIRING, self.hello.as_bytes(), reply.unsigned());
        city.verify(
            &signed(PAIRING, CITY_SIGNS, &transcript),
            &Signature::from_bytes(signature)?,
        )
        .map_err(|_| unrecognised("the city's signature"))?;
        let keys = self.ephemeral.keys(&transcript, x_peer, ciphertext)?;
        let proof = device_key.sign(&signed(PAIRING, DEVICE_SIGNS, &transcript))?;
        let claim = [
            code.as_bytes(),
            device_key.public().as_bytes().as_slice(),
            proof.as_bytes().as_slice(),
        ]
        .concat();
        let mut session = keys.device_session()?;
        let sealed = session.sealer.seal(&claim)?;
        Ok(Claimed {
            sealed,
            city,
            session,
        })
    }
}

pub fn city_pair_reply(
    hello: &PairHello,
    city: &SigningKey,
    nonce: [u8; NONCE_BYTES],
) -> Result<(PairReply, CityPairing), AxError> {
    let (x_peer, kem_peer) = hello.parts();
    let answer = Answer::new(x_peer, kem_peer)?;
    let unsigned = [city.public().as_bytes().as_slice(), &answer.public, &nonce].concat();
    let transcript = record(PAIRING, hello.as_bytes(), &unsigned);
    let signature = city.sign(&signed(PAIRING, CITY_SIGNS, &transcript))?;
    Ok((
        PairReply::from_bytes(&[unsigned.as_slice(), signature.as_bytes()].concat())?,
        CityPairing {
            transcript,
            keys: answer.keys(&transcript)?,
        },
    ))
}

impl CityPairing {

    pub fn open_claim(self, sealed: &[u8]) -> Result<(Claim, Session), AxError> {
        let mut session = self.keys.city_session()?;
        let opened = session
            .opener
            .open(sealed)
            .map_err(|_| unclaimed("a sealed claim"))?;
        let bytes: [u8; CLAIM_BYTES] = fixed(&opened, "read a pairing claim")?;
        let (code, rest) = bytes.split_at(CODE_TEXT_BYTES);
        let (key, proof) = rest.split_at(PUBLIC_BYTES);
        let device_key = VerifyingKey::from_bytes(key)?;
        device_key
            .verify(
                &signed(PAIRING, DEVICE_SIGNS, &self.transcript),
                &Signature::from_bytes(proof)?,
            )
            .map_err(|_| unclaimed("the device's signature"))?;
        let code = std::str::from_utf8(code)
            .map(PairingCode::read)
            .map_err(|_| {
                AxError::failure(
                    AxCode::WireMismatch,
                    "read a pairing claim",
                    "a code that is not text",
                )
                .with_recovery("the device and the city are on different versions; reload the page")
            })?;
        Ok((Claim { code, device_key }, session))
    }
}

fn code_text(typed: &str) -> Result<String, AxError> {
    let text = canonical(typed);
    if text.len() == CODE_TEXT_BYTES && text.bytes().all(is_symbol) {
        Ok(text)
    } else {
        Err(AxError::failure(
            AxCode::InvalidArgs,
            "read an invitation",
            format!("a pairing code of {} characters", text.chars().count()),
        )
        .with_recovery("scan the code on the city's console again"))
    }
}

fn unrecognised(subject: &str) -> AxError {
    AxError::failure(AxCode::GateDenied, "check the city's answer", subject)
        .with_recovery(SCAN_AGAIN)
}

fn unclaimed(subject: &str) -> AxError {
    AxError::failure(AxCode::GateDenied, "open a pairing claim", subject).with_recovery(
        "the connection ends and the code stays valid until it expires; scan it again",
    )
}
