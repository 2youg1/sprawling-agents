// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! An honest pairing leaves both sides agreeing, and each step the
//! `Handshake.lean` model names refuses what it must: the device sends no
//! claim to a city its invitation did not pin, and the city hands the
//! door no code it did not open from a claim sealed to this pairing.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test code"
)]

use super::*;
use crate::keys::SEED_BYTES;

fn city() -> SigningKey {
    SigningKey::from_seed(&[1; SEED_BYTES]).unwrap()
}

fn device() -> SigningKey {
    SigningKey::from_seed(&[2; SEED_BYTES]).unwrap()
}

fn entropy() -> [u8; CODE_BYTES] {
    [7; CODE_BYTES]
}

/// The invitation the city's console shows: the code as a person reads
/// it, grouped with dashes, and the fingerprint of `city`.
fn invitation(city: &SigningKey) -> Invitation {
    Invitation {
        city: CityFingerprint::of(&city.public()),
        code: PairingCode::mint(entropy()).1,
    }
}

/// A device's hello and a city's answer to it.
fn opened(city: &SigningKey) -> Result<(DevicePairing, PairReply, CityPairing), AxError> {
    let device_side = device_pair_hello([4; NONCE_BYTES])?;
    let (reply, city_side) = city_pair_reply(device_side.hello(), city, [5; NONCE_BYTES])?;
    Ok((device_side, reply, city_side))
}

/// The whole pairing, ending with the city's receipt opened on the device.
fn pair() -> Result<(VerifyingKey, Claim, Vec<u8>), AxError> {
    let (device_side, reply, city_side) = opened(&city())?;
    let mut claimed = device_side.claim(&reply, &invitation(&city()), &device())?;
    let (claim, mut on_city) = city_side.open_claim(&claimed.sealed)?;
    let receipt = on_city.sealer.seal(&[3; 16])?;
    Ok((claimed.city, claim, claimed.session.opener.open(&receipt)?))
}

fn code_of<T>(outcome: Result<T, AxError>) -> Result<(), AxCode> {
    outcome.map(|_| ()).map_err(|refusal| *refusal.code())
}

#[test]
fn an_honest_pairing_leaves_both_sides_agreeing() {
    assert_eq!(
        pair(),
        Ok((
            city().public(),
            Claim {
                code: PairingCode::mint(entropy()).0,
                device_key: device().public(),
            },
            vec![3; 16],
        ))
    );
}

#[test]
fn a_device_sends_no_claim_to_a_city_its_invitation_did_not_pin() {
    let impostor = SigningKey::from_seed(&[9; SEED_BYTES]).unwrap();
    let wrong_key = opened(&impostor).and_then(|(device_side, reply, _)| {
        device_side.claim(&reply, &invitation(&city()), &device())
    });
    // The pinned key presented, with the impostor's signature under it.
    let borrowed_key = opened(&impostor).and_then(|(device_side, reply, _)| {
        let mut bytes = reply.as_bytes().to_vec();
        bytes[..PUBLIC_BYTES].copy_from_slice(city().public().as_bytes());
        device_side.claim(
            &PairReply::from_bytes(&bytes)?,
            &invitation(&city()),
            &device(),
        )
    });
    assert_eq!(
        [code_of(wrong_key), code_of(borrowed_key)],
        [Err(AxCode::GateDenied), Err(AxCode::GateDenied)]
    );
}

#[test]
fn a_city_opens_only_a_claim_sealed_to_this_pairing_and_signed_by_its_key() {
    let changed = opened(&city()).and_then(|(device_side, reply, city_side)| {
        let mut claimed = device_side.claim(&reply, &invitation(&city()), &device())?;
        claimed.sealed[0] ^= 1;
        city_side.open_claim(&claimed.sealed)
    });
    let foreign = opened(&city()).and_then(|(device_side, reply, _)| {
        let claimed = device_side.claim(&reply, &invitation(&city()), &device())?;
        let (_, _, other_city_side) = opened(&city())?;
        other_city_side.open_claim(&claimed.sealed)
    });
    // The device's own key handed over, signed by a key it does not hold.
    let unproven = opened(&city()).and_then(|(device_side, reply, city_side)| {
        let transcript = record(PAIRING, device_side.hello.as_bytes(), reply.unsigned());
        let [_, x_peer, ciphertext, _] = reply.parts();
        let keys = device_side
            .ephemeral
            .keys(&transcript, x_peer, ciphertext)?;
        let thief = SigningKey::from_seed(&[8; SEED_BYTES])?;
        let proof = thief.sign(&signed(PAIRING, DEVICE_SIGNS, &transcript))?;
        let claim = [
            code_text(&invitation(&city()).code)?.as_bytes(),
            device().public().as_bytes().as_slice(),
            proof.as_bytes().as_slice(),
        ]
        .concat();
        let sealed = keys.device_session()?.sealer.seal(&claim)?;
        city_side.open_claim(&sealed)
    });
    assert_eq!(
        [code_of(changed), code_of(foreign), code_of(unproven)],
        [const { Err(AxCode::GateDenied) }; 3]
    );
}

#[test]
fn an_invitation_whose_code_is_not_26_symbols_sends_nothing() {
    let claim_with = |code: &str| {
        opened(&city()).and_then(|(device_side, reply, _)| {
            let invitation = Invitation {
                code: code.to_owned(),
                ..invitation(&city())
            };
            device_side.claim(&reply, &invitation, &device())
        })
    };
    assert_eq!(
        [
            code_of(claim_with("abcde-fghij")),
            code_of(claim_with("abcde-fghij-klmno-pqrst-uvw1z-a")),
        ],
        [Err(AxCode::InvalidArgs), Err(AxCode::InvalidArgs)]
    );
}

#[test]
fn a_fingerprint_reads_back_from_its_own_text_and_from_nothing_else() {
    let fingerprint = CityFingerprint::of(&city().public());
    let text = fingerprint.text();
    let short: String = text.chars().take(51).collect();
    // The last symbol carries one bit and four zero bits; setting a zero
    // bit spells the same bytes in a second way.
    let second_spelling: String = text
        .chars()
        .take(51)
        .chain(std::iter::once(match text.chars().last() {
            Some('a') => 'b',
            _ => 'r',
        }))
        .collect();
    assert_eq!(
        [
            CityFingerprint::read(&text).map_err(|refusal| *refusal.code()),
            CityFingerprint::read(&format!(" {} ", text.to_uppercase()))
                .map_err(|refusal| *refusal.code()),
            CityFingerprint::read(&short).map_err(|refusal| *refusal.code()),
            CityFingerprint::read(&second_spelling).map_err(|refusal| *refusal.code()),
        ],
        [
            Ok(fingerprint),
            Ok(fingerprint),
            Err(AxCode::InvalidArgs),
            Err(AxCode::InvalidArgs),
        ]
    );
}
