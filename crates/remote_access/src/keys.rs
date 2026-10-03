// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Hybrid signing keys: ML-DSA-44 and Ed25519, both derived from one
//! 32-byte seed, and a signature that holds only when both halves hold
//! (crates/remote_access/Spec.lean §8-3).
//!
//! The city's own key and every device's key have this shape. A device
//! generates its key where it lives and sends the public half at pairing;
//! the city keeps its seed in the platform credential store.

use aws_lc_rs::hkdf::{HKDF_SHA256, KeyType, Salt};
use aws_lc_rs::signature::{
    ED25519, Ed25519KeyPair, KeyPair, ML_DSA_44, ML_DSA_44_SIGNING, PqdsaKeyPair, UnparsedPublicKey,
};
use kernel::{AxCode, AxError, Sealed};
use zeroize::Zeroizing;

/// Bytes in the seed a key is derived from, and in its written form a
/// person may keep: 52 base32 characters.
pub const SEED_BYTES: usize = 32;
/// Bytes in the Ed25519 half of a public key.
pub const ED25519_PUBLIC_BYTES: usize = 32;
/// Bytes in the ML-DSA-44 half of a public key (FIPS 204, table 2).
pub const ML_DSA_44_PUBLIC_BYTES: usize = 1312;
/// Bytes in the Ed25519 half of a signature.
pub const ED25519_SIGNATURE_BYTES: usize = 64;
/// Bytes in the ML-DSA-44 half of a signature (FIPS 204, table 2).
pub const ML_DSA_44_SIGNATURE_BYTES: usize = 2420;
/// Bytes in a whole public key: Ed25519 first, then ML-DSA-44.
pub const PUBLIC_BYTES: usize = ED25519_PUBLIC_BYTES + ML_DSA_44_PUBLIC_BYTES;
/// Bytes in a whole signature: Ed25519 first, then ML-DSA-44.
pub const SIGNATURE_BYTES: usize = ED25519_SIGNATURE_BYTES + ML_DSA_44_SIGNATURE_BYTES;

/// The salt both halves are derived under. A version in the label means a
/// later derivation cannot produce a key an earlier one produced.
const DERIVATION_SALT: &[u8] = b"sprawling remote key v1";

/// A hybrid signing key. It never leaves the process that derived it.
pub struct SigningKey {
    ed25519: Ed25519KeyPair,
    ml_dsa: PqdsaKeyPair,
}

impl SigningKey {
    /// Derives both halves from one seed, each under its own label, so the
    /// two halves share no key material.
    ///
    /// # Errors
    /// Fails only when the cryptographic library refuses a derived seed.
    pub fn from_seed(seed: &[u8; SEED_BYTES]) -> Result<Self, AxError> {
        let (ed_seed, ml_seed) = halves(seed)?;
        let ed25519 = Ed25519KeyPair::from_seed_unchecked(&ed_seed)
            .map_err(|_| crypto_failure("derive the Ed25519 half of a key"))?;
        let ml_dsa = PqdsaKeyPair::from_seed(&ML_DSA_44_SIGNING, &ml_seed)
            .map_err(|_| crypto_failure("derive the ML-DSA-44 half of a key"))?;
        Ok(Self { ed25519, ml_dsa })
    }

    /// Derives the city's key from the seed the vault keeps, in the form
    /// [`written`] gives it (crates/remote_access/Spec.lean D23).
    ///
    /// # Errors
    /// `E_CONFIG_INVALID` for text [`written`] cannot produce: a wrong
    /// length, a character outside the alphabet, or stray bits at the end.
    pub fn from_sealed(seed: &Sealed<String>) -> Result<Self, AxError> {
        let _ = seed;
        Err(unreadable_seed())
    }

    /// The public half, as a device or the city publishes it.
    #[must_use]
    pub fn public(&self) -> VerifyingKey {
        let mut bytes = [0u8; PUBLIC_BYTES];
        let (ed, ml) = bytes.split_at_mut(ED25519_PUBLIC_BYTES);
        ed.copy_from_slice(self.ed25519.public_key().as_ref());
        ml.copy_from_slice(self.ml_dsa.public_key().as_ref());
        VerifyingKey(Box::new(bytes))
    }

    /// Signs `message` with both halves.
    ///
    /// # Errors
    /// Fails only when the cryptographic library refuses to sign.
    pub fn sign(&self, message: &[u8]) -> Result<Signature, AxError> {
        let mut bytes = [0u8; SIGNATURE_BYTES];
        let (ed, ml) = bytes.split_at_mut(ED25519_SIGNATURE_BYTES);
        ed.copy_from_slice(self.ed25519.sign(message).as_ref());
        let written = self
            .ml_dsa
            .sign(message, ml)
            .map_err(|_| crypto_failure("sign with the ML-DSA-44 half of a key"))?;
        if written != ML_DSA_44_SIGNATURE_BYTES {
            return Err(crypto_failure("sign with the ML-DSA-44 half of a key"));
        }
        Ok(Signature(Box::new(bytes)))
    }
}

/// The text the vault keeps for a seed: lower-case base32 without
/// padding, 52 characters, cleared when the caller drops it.
#[must_use]
pub fn written(seed: &[u8; SEED_BYTES]) -> Zeroizing<String> {
    Zeroizing::new(crate::pairing::encode(seed))
}

fn unreadable_seed() -> AxError {
    AxError::failure(
        AxCode::ConfigInvalid,
        "read the city key",
        "the seed the vault keeps for it",
    )
    .with_recovery(
        "replace the city key with `/remote replace-key` on the city's console, \
         then pair every device again",
    )
}

/// The public half of a hybrid key, in its wire form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifyingKey(Box<[u8; PUBLIC_BYTES]>);

impl VerifyingKey {
    /// # Errors
    /// Refuses bytes of any other length; which curve point or lattice key
    /// they hold is judged when a signature is checked.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, AxError> {
        let fixed: [u8; PUBLIC_BYTES] = bytes.try_into().map_err(|_| {
            AxError::failure(
                AxCode::InvalidArgs,
                "read a public key",
                format!("{} bytes", bytes.len()),
            )
            .with_recovery(format!(
                "a public key is {PUBLIC_BYTES} bytes: Ed25519, then ML-DSA-44"
            ))
        })?;
        Ok(Self(Box::new(fixed)))
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8; PUBLIC_BYTES] {
        &self.0
    }

    /// Holds when both halves of `signature` verify `message`.
    ///
    /// # Errors
    /// Refuses a signature either half rejects, with one answer for both,
    /// so a caller cannot learn which half failed.
    pub fn verify(&self, message: &[u8], signature: &Signature) -> Result<(), AxError> {
        let (ed_key, ml_key) = self.0.split_at(ED25519_PUBLIC_BYTES);
        let (ed_sig, ml_sig) = signature.0.split_at(ED25519_SIGNATURE_BYTES);
        let ed = UnparsedPublicKey::new(&ED25519, ed_key).verify(message, ed_sig);
        let ml = UnparsedPublicKey::new(&ML_DSA_44, ml_key).verify(message, ml_sig);
        match (ed, ml) {
            (Ok(()), Ok(())) => Ok(()),
            (Ok(()) | Err(_), Err(_)) | (Err(_), Ok(())) => Err(AxError::failure(
                AxCode::GateDenied,
                "verify a signature",
                "a hybrid signature",
            )
            .with_recovery(
                "pair the device again; the key it holds is not the one this city paired",
            )),
        }
    }
}

/// A hybrid signature, in its wire form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature(Box<[u8; SIGNATURE_BYTES]>);

impl Signature {
    /// # Errors
    /// Refuses bytes of any other length.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, AxError> {
        let fixed: [u8; SIGNATURE_BYTES] = bytes.try_into().map_err(|_| {
            AxError::failure(
                AxCode::InvalidArgs,
                "read a signature",
                format!("{} bytes", bytes.len()),
            )
            .with_recovery(format!(
                "a signature is {SIGNATURE_BYTES} bytes: Ed25519, then ML-DSA-44"
            ))
        })?;
        Ok(Self(Box::new(fixed)))
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8; SIGNATURE_BYTES] {
        &self.0
    }
}

/// The two sub-seeds a seed derives, Ed25519's first and ML-DSA-44's
/// second. A browser derives the same two from the seed in
/// `tools/fixtures/remote-handshake/keys.txt` (crates/remote_access/Spec.lean
/// §8-12), which is why the pair is readable inside the crate.
pub(crate) fn halves(
    seed: &[u8; SEED_BYTES],
) -> Result<([u8; SEED_BYTES], [u8; SEED_BYTES]), AxError> {
    Ok((derive(seed, b"ed25519")?, derive(seed, b"ml-dsa-44")?))
}

/// One 32-byte sub-seed per half, by HKDF-SHA256 under its own label.
fn derive(seed: &[u8; SEED_BYTES], label: &'static [u8]) -> Result<[u8; SEED_BYTES], AxError> {
    let mut out = [0u8; SEED_BYTES];
    Salt::new(HKDF_SHA256, DERIVATION_SALT)
        .extract(seed)
        .expand(&[label], Length(SEED_BYTES))
        .and_then(|okm| okm.fill(&mut out))
        .map_err(|_| crypto_failure("derive a key half from a seed"))?;
    Ok(out)
}

/// The output length HKDF is asked for.
pub(crate) struct Length(pub(crate) usize);

impl KeyType for Length {
    fn len(&self) -> usize {
        self.0
    }
}

pub(crate) fn crypto_failure(action: &str) -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        action,
        "the cryptographic library refused",
    )
    .with_recovery(
        "restart the city; if it recurs, report the platform and the release to the project",
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]
mod tests {
    use super::*;

    #[test]
    fn one_seed_gives_one_key_and_two_seeds_give_two() {
        let a = SigningKey::from_seed(&[1; SEED_BYTES]).unwrap().public();
        let again = SigningKey::from_seed(&[1; SEED_BYTES]).unwrap().public();
        let b = SigningKey::from_seed(&[2; SEED_BYTES]).unwrap().public();
        assert_eq!((a == again, a == b), (true, false));
    }

    #[test]
    fn a_signature_holds_only_with_both_halves() {
        let key = SigningKey::from_seed(&[1; SEED_BYTES]).unwrap();
        let signed = key.sign(b"hello").unwrap();
        let other = key.sign(b"other").unwrap();
        let mut ed_swapped = *signed.0;
        ed_swapped[..ED25519_SIGNATURE_BYTES].copy_from_slice(&other.0[..ED25519_SIGNATURE_BYTES]);
        let mut ml_swapped = *signed.0;
        ml_swapped[ED25519_SIGNATURE_BYTES..].copy_from_slice(&other.0[ED25519_SIGNATURE_BYTES..]);
        let public = key.public();
        assert_eq!(
            [
                public.verify(b"hello", &signed).is_ok(),
                public
                    .verify(b"hello", &Signature(Box::new(ed_swapped)))
                    .is_ok(),
                public
                    .verify(b"hello", &Signature(Box::new(ml_swapped)))
                    .is_ok(),
                SigningKey::from_seed(&[2; SEED_BYTES])
                    .unwrap()
                    .public()
                    .verify(b"hello", &signed)
                    .is_ok(),
            ],
            [true, false, false, false]
        );
    }

    fn sealed(text: &str) -> Sealed<String> {
        Sealed::new(Box::new(text.to_owned()))
    }

    #[test]
    fn a_written_seed_reads_back_as_the_key_its_seed_derives() {
        let seed = [7; SEED_BYTES];
        let text = written(&seed);
        let read = SigningKey::from_sealed(&sealed(&text))
            .ok()
            .map(|key| key.public());
        assert_eq!(
            (text.len(), read),
            (52, Some(SigningKey::from_seed(&seed).unwrap().public()))
        );
    }

    #[test]
    fn a_seed_written_any_other_way_is_refused() {
        let text = written(&[7; SEED_BYTES]);
        let short = &text[..51];
        let long = format!("{}a", text.as_str());
        let foreign = format!("{short}1");
        let stray = format!("{short}{}", if text.ends_with('a') { "b" } else { "r" });
        let codes: Vec<Option<AxCode>> =
            [short, long.as_str(), foreign.as_str(), stray.as_str(), ""]
                .iter()
                .map(|each| {
                    SigningKey::from_sealed(&sealed(each))
                        .err()
                        .map(|e| *e.code())
                })
                .collect();
        assert_eq!(codes, vec![Some(AxCode::ConfigInvalid); 5]);
    }

    #[test]
    fn wire_forms_refuse_the_wrong_length() {
        assert!(VerifyingKey::from_bytes(&[0; PUBLIC_BYTES - 1]).is_err());
        assert!(Signature::from_bytes(&[0; SIGNATURE_BYTES + 1]).is_err());
    }
}
