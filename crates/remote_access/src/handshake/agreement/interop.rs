// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The vectors and the one-way fixture a browser must reproduce
//! (crates/remote_access/Spec.lean §8-12, remote_access D11), written into
//! `tools/fixtures/remote-handshake/` by this module and read back by it.
//!
//! Without `GOLDEN_WRITE=1` every test only reads and compares. With it,
//! the three files with no randomness in them are written again, byte for
//! byte the same while the protocol stands, and the signature and the
//! fixture, which carry randomness, are written again only when the
//! committed copy no longer verifies here.
//!
//! The module sits under `agreement` because the fixture's device holds
//! two fixed ephemeral private keys, and the fields of [`Ephemeral`] are
//! private to `agreement`: a child reaches them, and the release binary
//! keeps `Ephemeral::generate` as the only way a device's ephemeral keys
//! come to be.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use aws_lc_rs::aead::{AES_256_GCM, Aad, LessSafeKey, Nonce, UnboundKey};
use aws_lc_rs::agreement::{PrivateKey, X25519};
use aws_lc_rs::kem::{DecapsulationKey, ML_KEM_768};

use super::{Ephemeral, derive};
use crate::handshake::{DeviceWaiting, Hello, NONCE_BYTES, Reply, city_reply};
use crate::keys::{SEED_BYTES, Signature, SigningKey, VerifyingKey, halves};
use crate::seal::{Direction, Opener, Payload, Sealer};
use file::{Fields, dir, kept, known, read, text};

mod file;

/// The seed of `keys.txt` and `signature-rust.txt`. Distinct bytes, so a
/// reader that reverses or shifts them gets another answer.
const SEED: [u8; SEED_BYTES] = *b"sprawling remote vector seed 001";
/// The message `signature-rust.txt` signs.
const MESSAGE: &[u8] = b"sprawling remote signature vector";

/// The three inputs of `session-keys.txt`; no handshake drew them.
const TRANSCRIPT: [u8; 32] = *b"session keys vector: transcript.";
const X25519_SECRET: [u8; 32] = *b"session keys vector: x25519 key.";
const ML_KEM_SECRET: [u8; 32] = *b"session keys vector: ml-kem key.";

/// The city-to-device key of `seal.txt`, and the frame sealed under it.
const SEAL_KEY: [u8; 32] = *b"seal vector: city to device key.";
const SEAL_FRAME: &str = "a frame the city seals first";

/// The fixture's city, its device, and what the device holds fixed.
const CITY_SEED: [u8; SEED_BYTES] = *b"sprawling remote fixture city 01";
const DEVICE_SEED: [u8; SEED_BYTES] = *b"sprawling remote fixture device1";
const DEVICE_ID: [u8; 16] = *b"fixture device 1";
const X25519_PRIVATE: [u8; 32] = *b"fixture device x25519 ephemeral!";
const DEVICE_NONCE: [u8; NONCE_BYTES] = *b"fixture device nonce, 32 bytes..";
const CITY_NONCE: [u8; NONCE_BYTES] = *b"fixture city nonce, 32 bytes....";
/// The agreed plaintext: the text of the first frame the city seals.
const FIRST_FRAME: &str = "sprawling remote fixture: the city speaks first";

/// The field names of each file, in the order §8-12 gives them.
const KEYS: [&str; 4] = ["seed", "ed25519_seed", "ml_dsa_44_seed", "public"];
const SESSION_KEYS: [&str; 5] = [
    "transcript",
    "x25519_secret",
    "ml_kem_secret",
    "device_to_city",
    "city_to_device",
];
const SEAL: [&str; 7] = [
    "key",
    "frame",
    "frame_nonce",
    "frame_sealed",
    "lock",
    "lock_nonce",
    "lock_sealed",
];
const SIGNATURE: [&str; 3] = ["public", "message", "signature"];
const FIXTURE: [&str; 7] = [
    "city_public",
    "hello",
    "x25519_private",
    "ml_kem_private",
    "reply",
    "first_frame",
    "plaintext",
];

#[test]
fn keys_vector_is_the_derivation() {
    let (ed25519_seed, ml_dsa_44_seed) = halves(&SEED).unwrap();
    let public = SigningKey::from_seed(&SEED).unwrap().public();
    known(
        "keys.txt",
        &Fields::of(
            KEYS,
            [
                SEED.to_vec(),
                ed25519_seed.to_vec(),
                ml_dsa_44_seed.to_vec(),
                public.as_bytes().to_vec(),
            ],
        ),
    );
}

#[test]
fn session_keys_vector_is_the_derivation() {
    let keys = derive(&TRANSCRIPT, &X25519_SECRET, &ML_KEM_SECRET).unwrap();
    known(
        "session-keys.txt",
        &Fields::of(
            SESSION_KEYS,
            [
                TRANSCRIPT.to_vec(),
                X25519_SECRET.to_vec(),
                ML_KEM_SECRET.to_vec(),
                keys.device_to_city.to_vec(),
                keys.city_to_device.to_vec(),
            ],
        ),
    );
}

#[test]
fn seal_vector_is_what_the_sealer_writes() {
    let frame = Payload::Frame(SEAL_FRAME.to_owned()).to_bytes();
    let lock = Payload::Lock.to_bytes();
    let mut sealer = Sealer::new(&SEAL_KEY, Direction::CityToDevice).unwrap();
    let frame_sealed = sealer.seal(&frame).unwrap();
    let lock_sealed = sealer.seal(&lock).unwrap();
    let fields = known(
        "seal.txt",
        &Fields::of(
            SEAL,
            [
                SEAL_KEY.to_vec(),
                frame.clone(),
                [b"c->d".as_slice(), &0u64.to_be_bytes()].concat(),
                frame_sealed,
                lock.clone(),
                [b"c->d".as_slice(), &1u64.to_be_bytes()].concat(),
                lock_sealed,
            ],
        ),
    );

    // The nonce a browser reads is the one the sealer used: plain
    // AES-256-GCM under it gives the sealed bytes the file holds.
    let by_hand = |payload: &str, nonce: &str| {
        let key = LessSafeKey::new(UnboundKey::new(&AES_256_GCM, fields.get("key")).unwrap());
        let nonce = Nonce::try_assume_unique_for_key(fields.get(nonce)).unwrap();
        let mut sealed = fields.get(payload).to_vec();
        key.seal_in_place_append_tag(nonce, Aad::empty(), &mut sealed)
            .unwrap();
        sealed
    };
    let mut opener = Opener::new(&SEAL_KEY, Direction::CityToDevice).unwrap();
    let opened = [
        opener.open(fields.get("frame_sealed")).unwrap(),
        opener.open(fields.get("lock_sealed")).unwrap(),
    ];
    assert_eq!(
        (
            by_hand("frame", "frame_nonce"),
            by_hand("lock", "lock_nonce"),
            opened.map(|bytes| Payload::from_bytes(&bytes).unwrap()),
        ),
        (
            fields.get("frame_sealed").to_vec(),
            fields.get("lock_sealed").to_vec(),
            [Payload::Frame(SEAL_FRAME.to_owned()), Payload::Lock],
        )
    );
}

#[test]
fn every_signature_vector_verifies() {
    kept("signature-rust.txt", signature, signed_by_the_seed);
    let mut names: Vec<String> = std::fs::read_dir(dir())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter(|name| name.starts_with("signature-") && name.ends_with(".txt"))
        .collect();
    names.sort();
    let verdicts: Vec<(String, Result<(), String>)> = names
        .into_iter()
        .map(|name| {
            let verdict = read(&name).and_then(|fields| verifies(&fields));
            (name, verdict)
        })
        .collect();
    assert!(
        verdicts.iter().all(|(_, verdict)| verdict.is_ok()),
        "{verdicts:?}"
    );
}

#[test]
fn fixture_opens_on_a_device_with_its_fixed_ephemeral_keys() {
    let fields = kept("fixture.txt", fixture, opens_to_the_agreed_plaintext);
    assert_eq!(
        (
            opens(&fields),
            Payload::from_bytes(fields.get("plaintext")).unwrap()
        ),
        (
            Ok(fields.get("plaintext").to_vec()),
            Payload::Frame(FIRST_FRAME.to_owned())
        )
    );
}

/// The Rust signature: the key of `keys.txt` signs [`MESSAGE`].
fn signature() -> Fields {
    let key = SigningKey::from_seed(&SEED).unwrap();
    Fields::of(
        SIGNATURE,
        [
            key.public().as_bytes().to_vec(),
            MESSAGE.to_vec(),
            key.sign(MESSAGE).unwrap().as_bytes().to_vec(),
        ],
    )
}

fn signed_by_the_seed(fields: &Fields) -> Result<(), String> {
    let expected = SigningKey::from_seed(&SEED).unwrap().public();
    if fields.try_get("public")? != expected.as_bytes() || fields.try_get("message")? != MESSAGE {
        return Err("signed by another key or over another message".to_owned());
    }
    verifies(fields)
}

fn verifies(fields: &Fields) -> Result<(), String> {
    fields.named(SIGNATURE)?;
    let public = VerifyingKey::from_bytes(fields.try_get("public")?).map_err(text)?;
    let signature = Signature::from_bytes(fields.try_get("signature")?).map_err(text)?;
    public
        .verify(fields.try_get("message")?, &signature)
        .map_err(text)
}

/// One handshake against the fixed device: a fresh ML-KEM key, whose
/// private half goes into the file, then the city's reply, the device's
/// finish, and the first frame the city seals.
fn fixture() -> Fields {
    let ml_kem = DecapsulationKey::generate(&ML_KEM_768).unwrap();
    let ml_kem_private = ml_kem.key_bytes().unwrap().as_ref().to_vec();
    let ml_kem_public = ml_kem
        .encapsulation_key()
        .and_then(|key| key.key_bytes())
        .unwrap()
        .as_ref()
        .to_vec();
    let x25519_public = PrivateKey::from_private_key(&X25519, &X25519_PRIVATE)
        .unwrap()
        .compute_public_key()
        .unwrap()
        .as_ref()
        .to_vec();
    let hello = Hello::from_bytes(
        &[
            DEVICE_ID.as_slice(),
            x25519_public.as_slice(),
            ml_kem_public.as_slice(),
            DEVICE_NONCE.as_slice(),
        ]
        .concat(),
    )
    .unwrap();
    let city = SigningKey::from_seed(&CITY_SEED).unwrap();
    let device = SigningKey::from_seed(&DEVICE_SEED).unwrap();
    let (reply, city_waiting) = city_reply(&hello, &city, CITY_NONCE).unwrap();
    let waiting = fixed_device(&hello, &X25519_PRIVATE, &ml_kem_private).unwrap();
    let (finish, _) = waiting.finish(&reply, &city.public(), &device).unwrap();
    let mut session = city_waiting.accept(&finish, &device.public()).unwrap();
    let plaintext = Payload::Frame(FIRST_FRAME.to_owned()).to_bytes();
    let first_frame = session.sealer.seal(&plaintext).unwrap();
    Fields::of(
        FIXTURE,
        [
            city.public().as_bytes().to_vec(),
            hello.as_bytes().to_vec(),
            X25519_PRIVATE.to_vec(),
            ml_kem_private,
            reply.as_bytes().to_vec(),
            first_frame,
            plaintext,
        ],
    )
}

fn opens_to_the_agreed_plaintext(fields: &Fields) -> Result<(), String> {
    fields.named(FIXTURE)?;
    let agreed = Payload::Frame(FIRST_FRAME.to_owned()).to_bytes();
    if fields.try_get("plaintext")? != agreed.as_slice() {
        return Err("the plaintext is not the agreed one".to_owned());
    }
    if opens(fields)? != agreed {
        return Err("the first frame opens to something else".to_owned());
    }
    Ok(())
}

/// What a device holding the fixture's two ephemeral private keys reads
/// from its first frame, going through the device's half of §8-4.
fn opens(fields: &Fields) -> Result<Vec<u8>, String> {
    let hello = Hello::from_bytes(fields.try_get("hello")?).map_err(text)?;
    let waiting = fixed_device(
        &hello,
        fields.try_get("x25519_private")?,
        fields.try_get("ml_kem_private")?,
    )?;
    let city = VerifyingKey::from_bytes(fields.try_get("city_public")?).map_err(text)?;
    let reply = Reply::from_bytes(fields.try_get("reply")?).map_err(text)?;
    let device = SigningKey::from_seed(&DEVICE_SEED).map_err(text)?;
    let (_, mut session) = waiting.finish(&reply, &city, &device).map_err(text)?;
    session
        .opener
        .open(fields.try_get("first_frame")?)
        .map_err(text)
}

/// The test seam of §8-12: a device whose ephemeral keys are the two
/// private keys given, not ones `Ephemeral::generate` drew.
fn fixed_device(hello: &Hello, x25519: &[u8], ml_kem: &[u8]) -> Result<DeviceWaiting, String> {
    Ok(DeviceWaiting {
        hello: hello.clone(),
        ephemeral: Ephemeral {
            x25519: PrivateKey::from_private_key(&X25519, x25519).map_err(text)?,
            ml_kem: DecapsulationKey::new(&ML_KEM_768, ml_kem).map_err(text)?,
        },
    })
}
