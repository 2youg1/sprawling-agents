// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

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

const X25519_BYTES: usize = 32;

const ML_KEM_KEY_BYTES: usize = 1184;

const ML_KEM_CIPHERTEXT_BYTES: usize = 1088;

pub const NONCE_BYTES: usize = 32;

const DEVICE_ID_BYTES: usize = 16;

pub const HELLO_BYTES: usize = DEVICE_ID_BYTES + X25519_BYTES + ML_KEM_KEY_BYTES + NONCE_BYTES;

pub const REPLY_BYTES: usize = UNSIGNED_REPLY_BYTES + SIGNATURE_BYTES;

const UNSIGNED_REPLY_BYTES: usize = X25519_BYTES + ML_KEM_CIPHERTEXT_BYTES + NONCE_BYTES;

const PROTOCOL: &[u8] = b"sprawling remote handshake v1";

const CITY_SIGNS: &[u8] = b"city";
const DEVICE_SIGNS: &[u8] = b"device";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hello {
    bytes: Box<[u8; HELLO_BYTES]>,
    device: DeviceId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply(Box<[u8; REPLY_BYTES]>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finish(Signature);

pub struct Session {
    pub sealer: Sealer,
    pub opener: Opener,
}

pub struct DeviceWaiting {
    hello: Hello,
    ephemeral: Ephemeral,
}

pub struct CityWaiting {
    device: DeviceId,
    transcript: [u8; 32],
    keys: Keys,
}

impl Hello {
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
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, AxError> {
        fixed(bytes, "read a handshake reply").map(|array| Self(Box::new(array)))
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8; REPLY_BYTES] {
        &self.0
    }

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
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, AxError> {
        Signature::from_bytes(bytes).map(Self)
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8; SIGNATURE_BYTES] {
        self.0.as_bytes()
    }
}

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
    #[must_use]
    pub fn device(&self) -> DeviceId {
        self.device
    }

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
