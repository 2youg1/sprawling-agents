// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The handshake an unpaired device makes to pair (remote_access-SPEC.md
//! §8-6). Its properties are stated by `crates/remote_access/spec/Handshake.lean`.
//!
//! The city has no key for the device yet, so only the city signs. The
//! device checks the key the city presents against the fingerprint its
//! invitation pinned, and only then sends its claim - the pairing code,
//! its own public key and its signature - sealed under the keys this
//! handshake agreed. A route that terminates TLS sees the hello, the
//! reply and ciphertext, and never the code.

use kernel::{AxCode, AxError};

use super::agreement::Answer;
use super::agreement::{Ephemeral, Keys, record, sha256, signed};
use super::{CITY_SIGNS, DEVICE_SIGNS, ML_KEM_CIPHERTEXT_BYTES, ML_KEM_KEY_BYTES, NONCE_BYTES};
use super::{Session, X25519_BYTES, fixed};
use crate::keys::{PUBLIC_BYTES, SIGNATURE_BYTES, Signature, SigningKey, VerifyingKey};
use crate::pairing::{CODE_BYTES, PairingCode, canonical, decode, encode, is_symbol};

#[cfg(test)]
mod tests;

/// Bytes in a city's fingerprint: the SHA-256 of its public key.
pub const FINGERPRINT_BYTES: usize = 32;
/// Symbols in a pairing code's canonical text: base32 carries five bits a
/// symbol, rounded up.
pub const CODE_TEXT_BYTES: usize = (CODE_BYTES * 8).div_ceil(5);
/// Bytes in [`PairHello`]: X25519, the ML-KEM-768 encapsulation key and a
/// nonce. No device id, because the city knows no device yet.
pub const PAIR_HELLO_BYTES: usize = X25519_BYTES + ML_KEM_KEY_BYTES + NONCE_BYTES;
/// Bytes of [`PairReply`] the city's signature covers through the transcript.
const UNSIGNED_PAIR_REPLY_BYTES: usize =
    PUBLIC_BYTES + X25519_BYTES + ML_KEM_CIPHERTEXT_BYTES + NONCE_BYTES;
/// Bytes in [`PairReply`]: the city's public key, its X25519 key, an
/// ML-KEM ciphertext, a nonce, and its signature.
pub const PAIR_REPLY_BYTES: usize = UNSIGNED_PAIR_REPLY_BYTES + SIGNATURE_BYTES;
/// Bytes in a claim before it is sealed: the code's canonical text, the
/// device's public key, and the device's signature.
pub const CLAIM_BYTES: usize = CODE_TEXT_BYTES + PUBLIC_BYTES + SIGNATURE_BYTES;

/// Opens the pairing transcript. It differs from the session handshake's
/// label, so a signature from one can never complete the other.
const PAIRING: &[u8] = b"sprawling remote pairing v1";

/// What a device is told when the answer it got is not from the city its
/// invitation named.
const SCAN_AGAIN: &str = "scan the code on the city's console again; if this repeats, the route is changing what it forwards";

/// The SHA-256 of a city's public key: what the invitation carries in
/// place of the key itself, which is too long for a QR code to stay easy
/// to scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CityFingerprint([u8; FINGERPRINT_BYTES]);

/// What the QR code on the city's console hands the device, beside the
/// address: the city's fingerprint and the pairing code's text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invitation {
    pub city: CityFingerprint,
    pub code: String,
}

/// The device's opening message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairHello(Box<[u8; PAIR_HELLO_BYTES]>);

/// The city's answer: its public key, its half of the key agreement, and
/// its signature over the transcript.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairReply(Box<[u8; PAIR_REPLY_BYTES]>);

/// A claim the city opened and whose signature held: the code to hand the
/// door, and the key the device proved it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Claim {
    code: PairingCode,
    device_key: VerifyingKey,
}

/// What the device holds once it sent its claim: the sealed claim to send,
/// the city's key to pin for every later connection, and the session the
/// city's receipt arrives on.
pub struct Claimed {
    pub sealed: Vec<u8>,
    pub city: VerifyingKey,
    pub session: Session,
}

/// A device's side after it sent [`PairHello`].
pub struct DevicePairing {
    hello: PairHello,
    ephemeral: Ephemeral,
}

/// The city's side after it sent [`PairReply`].
pub struct CityPairing {
    transcript: [u8; 32],
    keys: Keys,
}

impl CityFingerprint {
    /// The one place a fingerprint is computed.
    #[must_use]
    pub fn of(city: &VerifyingKey) -> Self {
        Self([0; FINGERPRINT_BYTES])
    }

    /// Base32 in lower case, the alphabet of the pairing code: 52 symbols.
    #[must_use]
    pub fn text(&self) -> String {
        String::new()
    }

    /// Reads the text [`Self::text`] wrote, in either case.
    ///
    /// # Errors
    /// Refuses anything but the one canonical text of 32 bytes, so two
    /// fingerprints are equal exactly when their texts are.
    pub fn read(text: &str) -> Result<Self, AxError> {
        Err(not_built(text))
    }
}

impl PairHello {
    /// # Errors
    /// Refuses bytes of any other length.
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
    /// # Errors
    /// Refuses bytes of any other length.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, AxError> {
        fixed(bytes, "read a pairing reply").map(|array| Self(Box::new(array)))
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8; PAIR_REPLY_BYTES] {
        &self.0
    }

    /// Everything but the signature, which is what the transcript covers.
    fn unsigned(&self) -> &[u8] {
        self.0.split_at(UNSIGNED_PAIR_REPLY_BYTES).0
    }

    /// The presented key, the X25519 key, the ciphertext, the signature.
    fn parts(&self) -> [&[u8]; 4] {
        let (presented, rest) = self.0.split_at(PUBLIC_BYTES);
        let (x25519, rest) = rest.split_at(X25519_BYTES);
        let (ciphertext, rest) = rest.split_at(ML_KEM_CIPHERTEXT_BYTES);
        let (_, signature) = rest.split_at(NONCE_BYTES);
        [presented, x25519, ciphertext, signature]
    }
}

impl Claim {
    /// The code the door redeems (`Door::pair`).
    #[must_use]
    pub fn code(&self) -> PairingCode {
        self.code
    }

    /// The key the device signed its claim with, which the door keeps.
    #[must_use]
    pub fn device_key(&self) -> &VerifyingKey {
        &self.device_key
    }
}

/// The device opens: fresh ephemeral keys, and the hello to send.
///
/// # Errors
/// Fails only when the cryptographic library refuses to generate a key.
pub fn device_pair_hello(nonce: [u8; NONCE_BYTES]) -> Result<DevicePairing, AxError> {
    Err(not_built("a pairing hello"))
}

impl DevicePairing {
    #[must_use]
    pub fn hello(&self) -> &PairHello {
        &self.hello
    }

    /// Checks the city's answer against the invitation, and seals the
    /// claim under the keys this handshake agreed.
    ///
    /// # Errors
    /// Refuses an invitation whose code is not 26 base32 symbols, a reply
    /// whose key has another fingerprint or whose signature that key did
    /// not make, and key material the library rejects. Nothing is sent
    /// after a refusal.
    pub fn claim(
        self,
        reply: &PairReply,
        invitation: &Invitation,
        device_key: &SigningKey,
    ) -> Result<Claimed, AxError> {
        Err(not_built("a claim"))
    }
}

/// The city answers a pairing hello: its public key, a fresh X25519 key,
/// an ML-KEM ciphertext to the device's key, and its signature over the
/// transcript.
///
/// # Errors
/// Refuses key material the library rejects.
pub fn city_pair_reply(
    hello: &PairHello,
    city: &SigningKey,
    nonce: [u8; NONCE_BYTES],
) -> Result<(PairReply, CityPairing), AxError> {
    Err(not_built("a pairing reply"))
}

impl CityPairing {
    /// Opens the device's claim and checks that the device signed this
    /// pairing with the key it hands over. The caller then asks the door
    /// to redeem [`Claim::code`], seals the device id it gets back on the
    /// session, and ends the connection.
    ///
    /// # Errors
    /// Refuses a claim that does not open under this pairing's keys, one
    /// of any other length, and one whose signature its key did not make.
    /// The code stays in the door until it expires.
    pub fn open_claim(self, sealed: &[u8]) -> Result<(Claim, Session), AxError> {
        Err(not_built("a sealed claim"))
    }
}

/// The invitation's code in the form the claim carries it.
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

fn not_built(subject: &str) -> AxError {
    AxError::failure(AxCode::StorageFatal, "pair a device", subject)
        .with_recovery("the pairing handshake is not built yet")
}
