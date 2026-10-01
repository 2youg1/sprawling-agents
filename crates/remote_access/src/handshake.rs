// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The handshake a paired device and its city make on every connection
//! (crates/remote_access/Spec.lean §8-4).
//!
//! Three messages. The device sends [`Hello`]: its id, an ephemeral
//! X25519 key, an ephemeral ML-KEM-768 encapsulation key and a nonce. The
//! city answers [`Reply`]: its own ephemeral X25519 key, an ML-KEM
//! ciphertext to the device's key, a nonce, and its hybrid signature over
//! the transcript. The device checks that signature against the city key
//! it pinned at pairing and answers [`Finish`]: its own hybrid signature
//! over the same transcript, which the city checks against the key it
//! paired. Both sides then hold the same two session keys, one per
//! direction, derived from both shared secrets under the transcript.
//!
//! A route between the two sees the three messages and nothing it can
//! use: it cannot sign as either side, and the session keys need both an
//! X25519 and an ML-KEM secret it never held.

use kernel::{AxCode, AxError};

use crate::door::DeviceId;
use crate::keys::{SIGNATURE_BYTES, Signature, SigningKey, VerifyingKey};
use crate::seal::{Opener, Sealer};
use agreement::{Answer, Ephemeral, Keys, record, signed};

mod agreement;
mod pairing;
#[cfg(test)]
mod tests;

pub use pairing::{CLAIM_BYTES, CODE_TEXT_BYTES, FINGERPRINT_BYTES};
pub use pairing::{CityFingerprint, CityPairing, Claim, Claimed, DevicePairing, Invitation};
pub use pairing::{PAIR_HELLO_BYTES, PAIR_REPLY_BYTES, PairHello, PairReply};
pub use pairing::{city_pair_reply, device_pair_hello};

/// Bytes in an X25519 public key.
const X25519_BYTES: usize = 32;
/// Bytes in an ML-KEM-768 encapsulation key (FIPS 203, table 3).
const ML_KEM_KEY_BYTES: usize = 1184;
/// Bytes in an ML-KEM-768 ciphertext (FIPS 203, table 3).
const ML_KEM_CIPHERTEXT_BYTES: usize = 1088;
/// Bytes in each side's nonce.
pub const NONCE_BYTES: usize = 32;
/// Bytes in a device id.
const DEVICE_ID_BYTES: usize = 16;

/// Bytes in [`Hello`]'s wire form.
pub const HELLO_BYTES: usize = DEVICE_ID_BYTES + X25519_BYTES + ML_KEM_KEY_BYTES + NONCE_BYTES;
/// Bytes in [`Reply`]'s wire form.
pub const REPLY_BYTES: usize = UNSIGNED_REPLY_BYTES + SIGNATURE_BYTES;
/// Bytes of [`Reply`] the city's signature covers through the transcript.
const UNSIGNED_REPLY_BYTES: usize = X25519_BYTES + ML_KEM_CIPHERTEXT_BYTES + NONCE_BYTES;

/// Opens the transcript, so a transcript of any other protocol hashes to
/// something else.
const PROTOCOL: &[u8] = b"sprawling remote handshake v1";
/// What each side signs ahead of the transcript hash, so the city's
/// signature can never be replayed as a device's, or the reverse.
const CITY_SIGNS: &[u8] = b"city";
const DEVICE_SIGNS: &[u8] = b"device";

/// The device's opening message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hello {
    bytes: Box<[u8; HELLO_BYTES]>,
    device: DeviceId,
}

/// The city's answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply(Box<[u8; REPLY_BYTES]>);

/// The device's closing message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finish(Signature);

/// The two halves of a session: what this side sends with, and what it
/// opens with.
pub struct Session {
    pub sealer: Sealer,
    pub opener: Opener,
}

/// A device's side after it sent [`Hello`].
pub struct DeviceWaiting {
    hello: Hello,
    ephemeral: Ephemeral,
}

/// The city's side after it sent [`Reply`].
pub struct CityWaiting {
    device: DeviceId,
    transcript: [u8; 32],
    keys: Keys,
}

impl Hello {
    /// # Errors
    /// Refuses bytes of any other length.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, AxError> {
        let array: [u8; HELLO_BYTES] = fixed(bytes, "read a handshake hello")?;
        let (id, _) = array
            .split_first_chunk::<DEVICE_ID_BYTES>()
            .ok_or_else(|| {
                AxError::failure(
                    AxCode::WireMismatch,
                    "read a handshake hello",
                    "its device id",
                )
                .with_recovery("the device and the city are on different versions")
            })?;
        let device = DeviceId::from_entropy(*id);
        Ok(Self {
            bytes: Box::new(array),
            device,
        })
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8; HELLO_BYTES] {
        &self.bytes
    }

    /// The device this hello says it comes from; the city looks its key up
    /// by this id and the signature in [`Finish`] proves the claim.
    #[must_use]
    pub fn device(&self) -> DeviceId {
        self.device
    }

    fn parts(&self) -> (&[u8], &[u8]) {
        let (_, rest) = self.bytes.split_at(DEVICE_ID_BYTES);
        let (x25519, rest) = rest.split_at(X25519_BYTES);
        let (ml_kem, _) = rest.split_at(ML_KEM_KEY_BYTES);
        (x25519, ml_kem)
    }
}

impl Reply {
    /// # Errors
    /// Refuses bytes of any other length.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, AxError> {
        fixed(bytes, "read a handshake reply").map(|array| Self(Box::new(array)))
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8; REPLY_BYTES] {
        &self.0
    }

    /// Everything but the signature, which is what the transcript covers.
    fn unsigned(&self) -> &[u8] {
        self.0.split_at(UNSIGNED_REPLY_BYTES).0
    }

    fn parts(&self) -> (&[u8], &[u8], &[u8]) {
        let (x25519, rest) = self.0.split_at(X25519_BYTES);
        let (ciphertext, rest) = rest.split_at(ML_KEM_CIPHERTEXT_BYTES);
        let (_, signature) = rest.split_at(NONCE_BYTES);
        (x25519, ciphertext, signature)
    }
}

impl Finish {
    /// # Errors
    /// Refuses bytes of any other length.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, AxError> {
        Signature::from_bytes(bytes).map(Self)
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8; SIGNATURE_BYTES] {
        self.0.as_bytes()
    }
}

/// The device opens: fresh ephemeral keys, and the hello to send.
///
/// # Errors
/// Fails only when the cryptographic library refuses to generate a key.
pub fn device_hello(device: DeviceId, nonce: [u8; NONCE_BYTES]) -> Result<DeviceWaiting, AxError> {
    let (ephemeral, public) = Ephemeral::generate()?;
    let bytes = [device.as_bytes().as_slice(), &public, &nonce].concat();
    Ok(DeviceWaiting {
        hello: Hello::from_bytes(&bytes)?,
        ephemeral,
    })
}

impl DeviceWaiting {
    #[must_use]
    pub fn hello(&self) -> &Hello {
        &self.hello
    }

    /// Checks the city's reply against the key pinned at pairing, and
    /// signs the transcript with the device's own key.
    ///
    /// # Errors
    /// Refuses a reply the pinned city key did not sign, and a reply whose
    /// key material the library rejects.
    pub fn finish(
        self,
        reply: &Reply,
        city: &VerifyingKey,
        device_key: &SigningKey,
    ) -> Result<(Finish, Session), AxError> {
        let transcript = record(PROTOCOL, self.hello.as_bytes(), reply.unsigned());
        let (x_peer, ciphertext, signature) = reply.parts();
        city.verify(
            &signed(PROTOCOL, CITY_SIGNS, &transcript),
            &Signature::from_bytes(signature)?,
        )?;
        let keys = self.ephemeral.keys(&transcript, x_peer, ciphertext)?;
        let finish = Finish(device_key.sign(&signed(PROTOCOL, DEVICE_SIGNS, &transcript))?);
        Ok((finish, keys.device_session()?))
    }
}

/// The city answers a hello: fresh ephemeral X25519, an ML-KEM ciphertext
/// to the device's key, and the city's signature over the transcript.
///
/// # Errors
/// Refuses key material the library rejects.
pub fn city_reply(
    hello: &Hello,
    city: &SigningKey,
    nonce: [u8; NONCE_BYTES],
) -> Result<(Reply, CityWaiting), AxError> {
    let (x_peer, kem_peer) = hello.parts();
    let answer = Answer::new(x_peer, kem_peer)?;
    let unsigned = [answer.public.as_slice(), &nonce].concat();
    let transcript = record(PROTOCOL, hello.as_bytes(), &unsigned);
    let signature = city.sign(&signed(PROTOCOL, CITY_SIGNS, &transcript))?;
    Ok((
        Reply::from_bytes(&[unsigned.as_slice(), signature.as_bytes()].concat())?,
        CityWaiting {
            device: hello.device(),
            transcript,
            keys: answer.keys(&transcript)?,
        },
    ))
}

impl CityWaiting {
    /// The device the hello claimed to be; [`Self::accept`] proves it.
    #[must_use]
    pub fn device(&self) -> DeviceId {
        self.device
    }

    /// Checks the device's signature against the key the city paired.
    ///
    /// # Errors
    /// Refuses a finish that key did not sign.
    pub fn accept(self, finish: &Finish, device_key: &VerifyingKey) -> Result<Session, AxError> {
        device_key.verify(&signed(PROTOCOL, DEVICE_SIGNS, &self.transcript), &finish.0)?;
        self.keys.city_session()
    }
}

fn fixed<const N: usize>(bytes: &[u8], action: &str) -> Result<[u8; N], AxError> {
    bytes.try_into().map_err(|_| {
        AxError::failure(
            AxCode::WireMismatch,
            action,
            format!("{} bytes", bytes.len()),
        )
        .with_recovery(format!(
            "this message is {N} bytes; the device and the city are on different versions"
        ))
    })
}
