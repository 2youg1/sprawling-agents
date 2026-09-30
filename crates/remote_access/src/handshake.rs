// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The handshake a paired device and its city make on every connection
//! (remote_access-SPEC.md §8-4).
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

use aws_lc_rs::agreement::{self, PrivateKey, UnparsedPublicKey, X25519};
use aws_lc_rs::digest::{SHA256, digest};
use aws_lc_rs::hkdf::{HKDF_SHA256, Salt};
use aws_lc_rs::kem::{Ciphertext, DecapsulationKey, EncapsulationKey, ML_KEM_768};
use kernel::{AxCode, AxError};

use crate::door::DeviceId;
use crate::keys::{Length, SIGNATURE_BYTES, Signature, SigningKey, VerifyingKey, crypto_failure};
use crate::seal::{Direction, Opener, Sealer};

#[cfg(test)]
mod tests;

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
    x25519: PrivateKey,
    ml_kem: DecapsulationKey,
}

/// The city's side after it sent [`Reply`].
pub struct CityWaiting {
    device: DeviceId,
    transcript: [u8; 32],
    keys: Keys,
}

struct Keys {
    device_to_city: [u8; 32],
    city_to_device: [u8; 32],
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
    let mut bytes = Vec::with_capacity(HELLO_BYTES);
    bytes.extend_from_slice(device.as_bytes());
    bytes.extend_from_slice(x_public.as_ref());
    bytes.extend_from_slice(kem_public.as_ref());
    bytes.extend_from_slice(&nonce);
    Ok(DeviceWaiting {
        hello: Hello::from_bytes(&bytes)?,
        x25519,
        ml_kem,
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
        let transcript = transcript(&self.hello, reply);
        let (x_peer, ciphertext, signature) = reply.parts();
        city.verify(
            &signed(CITY_SIGNS, &transcript),
            &Signature::from_bytes(signature)?,
        )?;
        let x_secret = agree(&self.x25519, x_peer)?;
        let kem_secret = self
            .ml_kem
            .decapsulate(Ciphertext::from(ciphertext))
            .map_err(|_| refused("open the city's ML-KEM ciphertext"))?;
        let keys = derive(&transcript, &x_secret, kem_secret.as_ref())?;
        let finish = Finish(device_key.sign(&signed(DEVICE_SIGNS, &transcript))?);
        let session = Session {
            sealer: Sealer::new(&keys.device_to_city, Direction::DeviceToCity)?,
            opener: Opener::new(&keys.city_to_device, Direction::CityToDevice)?,
        };
        Ok((finish, session))
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
    let mut bytes = Vec::with_capacity(REPLY_BYTES);
    bytes.extend_from_slice(x_public.as_ref());
    bytes.extend_from_slice(ciphertext.as_ref());
    bytes.extend_from_slice(&nonce);
    bytes.resize(REPLY_BYTES, 0);
    let unsigned = Reply::from_bytes(&bytes)?;
    let transcript = transcript(hello, &unsigned);
    let signature = city.sign(&signed(CITY_SIGNS, &transcript))?;
    bytes.truncate(UNSIGNED_REPLY_BYTES);
    bytes.extend_from_slice(signature.as_bytes());
    let keys = derive(&transcript, &x_secret, kem_secret.as_ref())?;
    Ok((
        Reply::from_bytes(&bytes)?,
        CityWaiting {
            device: hello.device(),
            transcript,
            keys,
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
        device_key.verify(&signed(DEVICE_SIGNS, &self.transcript), &finish.0)?;
        Ok(Session {
            sealer: Sealer::new(&self.keys.city_to_device, Direction::CityToDevice)?,
            opener: Opener::new(&self.keys.device_to_city, Direction::DeviceToCity)?,
        })
    }
}

/// SHA-256 of the protocol label, the hello and the reply without its
/// signature. SHA-256 rather than the city's BLAKE3, because the other end
/// is a browser, whose WebCrypto has SHA-256 and HKDF and no BLAKE3.
fn transcript(hello: &Hello, reply: &Reply) -> [u8; 32] {
    let input = [PROTOCOL, hello.as_bytes().as_slice(), reply.unsigned()].concat();
    let mut out = [0u8; 32];
    out.copy_from_slice(digest(&SHA256, &input).as_ref());
    out
}

fn signed(role: &[u8], transcript: &[u8; 32]) -> Vec<u8> {
    [PROTOCOL, role, transcript].concat()
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

/// Both session keys from both secrets, under the transcript as salt.
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

fn refused(action: &str) -> AxError {
    AxError::failure(AxCode::GateDenied, action, "the other side's key material")
        .with_recovery("start the connection again; if it repeats, pair the device again")
}
