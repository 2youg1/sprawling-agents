// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Every frame of a remote session, sealed end to end (crates/remote_access/Spec.lean §8-5).
//!
//! AES-256-GCM under the session key of one direction. The nonce is the
//! direction and a counter, and the counter is not sent: the socket
//! delivers frames in order, so each side knows the next number, and a
//! frame replayed, dropped or reordered on the way fails to open. A
//! failure ends the session; there is no frame worth skipping.

use aws_lc_rs::aead::{AES_256_GCM, Aad, LessSafeKey, NONCE_LEN, Nonce, UnboundKey};
use kernel::{AxCode, AxError};

use crate::keys::crypto_failure;

/// Which way a frame travels. It is part of the nonce, so a frame sealed
/// one way cannot be opened the other way even under a mistaken key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    DeviceToCity,
    CityToDevice,
}

impl Direction {
    fn tag(self) -> [u8; 4] {
        match self {
            Direction::DeviceToCity => *b"d->c",
            Direction::CityToDevice => *b"c->d",
        }
    }
}

/// What one sealed frame of a session carries, told apart by its first
/// byte (crates/remote_access/Spec.lean §8-5, remote_access D13): the door's own verb, the
/// lock, never travels on the wire protocol, so it is spoken here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Payload {
    /// One wire-protocol text frame, carried in both directions and
    /// forwarded as it arrived rather than parsed and written again.
    Frame(String),
    /// A device locking the door as it leaves; any authority may send it.
    Lock,
}

/// The first byte of a [`Payload::Frame`].
const FRAME: u8 = 0;
/// The one byte of a [`Payload::Lock`].
const LOCK: u8 = 1;

impl Payload {
    /// The bytes to seal.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Payload::Frame(text) => {
                let mut bytes = Vec::with_capacity(text.len().saturating_add(1));
                bytes.push(FRAME);
                bytes.extend_from_slice(text.as_bytes());
                bytes
            }
            Payload::Lock => vec![LOCK],
        }
    }

    /// Reads an opened frame back.
    ///
    /// # Errors
    /// Refuses an empty payload, an unknown first byte, a lock followed by
    /// more bytes, and a frame that is not UTF-8; the caller ends the
    /// connection.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, AxError> {
        match bytes.split_first() {
            Some((&FRAME, text)) => std::str::from_utf8(text)
                .map(|frame| Payload::Frame(frame.to_owned()))
                .map_err(|_| unreadable("a frame that is not UTF-8".to_owned())),
            Some((&LOCK, [])) => Ok(Payload::Lock),
            Some((&LOCK, [_, ..])) => Err(unreadable("a lock followed by more bytes".to_owned())),
            Some((other, _)) => Err(unreadable(format!(
                "a payload that opens with byte {other}"
            ))),
            None => Err(unreadable("an empty payload".to_owned())),
        }
    }
}

fn unreadable(subject: String) -> AxError {
    AxError::failure(AxCode::WireMismatch, "read a remote payload", subject)
        .with_recovery("the device and the city are on different versions; reload the page")
}

/// Seals the frames one side sends.
pub struct Sealer {
    key: LessSafeKey,
    direction: Direction,
    next: u64,
}

/// Opens the frames one side receives, in the order they were sealed.
pub struct Opener {
    key: LessSafeKey,
    direction: Direction,
    next: u64,
}

impl Sealer {
    pub(crate) fn new(key: &[u8; 32], direction: Direction) -> Result<Self, AxError> {
        Ok(Self {
            key: aes(key)?,
            direction,
            next: 0,
        })
    }

    /// Seals one frame and advances the counter.
    ///
    /// # Errors
    /// Refuses once the counter is spent, which a session reaches only
    /// after 2^64 frames.
    pub fn seal(&mut self, frame: &[u8]) -> Result<Vec<u8>, AxError> {
        let nonce = nonce(self.direction, self.next);
        self.next = advance(self.next)?;
        let mut sealed = frame.to_vec();
        self.key
            .seal_in_place_append_tag(nonce, Aad::empty(), &mut sealed)
            .map_err(|_| crypto_failure("seal a remote frame"))?;
        Ok(sealed)
    }
}

impl Opener {
    pub(crate) fn new(key: &[u8; 32], direction: Direction) -> Result<Self, AxError> {
        Ok(Self {
            key: aes(key)?,
            direction,
            next: 0,
        })
    }

    /// Opens the next frame.
    ///
    /// # Errors
    /// Refuses a frame that was changed, replayed, dropped before this one,
    /// or sealed for the other direction. The caller ends the session.
    pub fn open(&mut self, sealed: &[u8]) -> Result<Vec<u8>, AxError> {
        let nonce = nonce(self.direction, self.next);
        let mut buffer = sealed.to_vec();
        let length = self
            .key
            .open_in_place(nonce, Aad::empty(), &mut buffer)
            .map_err(|_| {
                AxError::failure(AxCode::GateDenied, "open a remote frame", "a sealed frame")
                    .with_recovery("reconnect: a frame was changed, replayed or lost on the way")
            })?
            .len();
        self.next = advance(self.next)?;
        buffer.truncate(length);
        Ok(buffer)
    }
}

fn aes(key: &[u8; 32]) -> Result<LessSafeKey, AxError> {
    UnboundKey::new(&AES_256_GCM, key)
        .map(LessSafeKey::new)
        .map_err(|_| crypto_failure("load a session key"))
}

/// Four bytes of direction, then the counter in big-endian order.
fn nonce(direction: Direction, counter: u64) -> Nonce {
    let mut bytes = [0u8; NONCE_LEN];
    let (tag, count) = bytes.split_at_mut(4);
    tag.copy_from_slice(&direction.tag());
    count.copy_from_slice(&counter.to_be_bytes());
    Nonce::assume_unique_for_key(bytes)
}

fn advance(counter: u64) -> Result<u64, AxError> {
    counter.checked_add(1).ok_or_else(|| {
        AxError::failure(
            AxCode::BudgetExhausted,
            "seal a remote frame",
            "the session's frame counter",
        )
        .with_recovery("reconnect: a new session starts a new counter under new keys")
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]
mod tests {
    use super::*;

    fn pair() -> (Sealer, Opener) {
        (
            Sealer::new(&[7; 32], Direction::DeviceToCity).unwrap(),
            Opener::new(&[7; 32], Direction::DeviceToCity).unwrap(),
        )
    }

    #[test]
    fn frames_open_in_the_order_they_were_sealed() {
        let (mut sealer, mut opener) = pair();
        let first = sealer.seal(b"one").unwrap();
        let second = sealer.seal(b"two").unwrap();
        assert_eq!(
            [opener.open(&first).unwrap(), opener.open(&second).unwrap()],
            [b"one".to_vec(), b"two".to_vec()]
        );
    }

    #[test]
    fn a_replayed_dropped_changed_or_turned_frame_does_not_open() {
        let (mut sealer, mut opener) = pair();
        let first = sealer.seal(b"one").unwrap();
        opener.open(&first).unwrap();
        let replayed = opener.open(&first).is_err();

        let (mut sealer, mut opener) = pair();
        let _dropped = sealer.seal(b"one").unwrap();
        let skipped = opener.open(&sealer.seal(b"two").unwrap()).is_err();

        let (mut sealer, mut opener) = pair();
        let mut changed = sealer.seal(b"one").unwrap();
        changed[0] ^= 1;
        let tampered = opener.open(&changed).is_err();

        let (mut sealer, _) = pair();
        let mut turned = Opener::new(&[7; 32], Direction::CityToDevice).unwrap();
        let wrong_way = turned.open(&sealer.seal(b"one").unwrap()).is_err();

        assert_eq!(
            [replayed, skipped, tampered, wrong_way],
            [true, true, true, true]
        );
    }

    #[test]
    fn a_payload_reads_back_as_it_was_written() {
        let frame = Payload::Frame(r#"{"frame":"ask"}"#.to_owned());
        assert_eq!(
            [
                Payload::from_bytes(&frame.to_bytes()),
                Payload::from_bytes(&Payload::Lock.to_bytes()),
            ],
            [Ok(frame.clone()), Ok(Payload::Lock)]
        );
    }

    #[test]
    fn the_first_byte_of_a_payload_says_a_frame_or_the_lock() {
        assert_eq!(
            [
                Payload::Frame("ab".to_owned()).to_bytes(),
                Payload::Lock.to_bytes()
            ],
            [vec![0, b'a', b'b'], vec![1]]
        );
    }

    #[test]
    fn a_payload_that_is_neither_is_refused() {
        let empty: &[u8] = &[];
        let unknown: &[u8] = &[2, b'a'];
        let lock_and_more: &[u8] = &[1, 0];
        let not_utf8: &[u8] = &[0, 0xff];
        assert_eq!(
            [empty, unknown, lock_and_more, not_utf8]
                .map(|bytes| Payload::from_bytes(bytes).map_err(|refusal| *refusal.code())),
            [const { Err(AxCode::WireMismatch) }; 4]
        );
    }
}
