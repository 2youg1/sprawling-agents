// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The city's remote signing key as the vault keeps it
//! (`crates/sprawling/spec/Outside/Conduit.lean` §8-139,
//! crates/remote_access/Spec.lean D23): read back at every start, written
//! the first time, replaced when the User asks.
//!
//! The seed never becomes bytes here. A fresh seed is drawn into a buffer,
//! written to the vault in the form `keys::written` gives it, and read
//! back through the same path every later start takes, so the first start
//! and the hundredth derive the key in one place: `SigningKey::from_sealed`.

use std::sync::{Arc, Mutex, MutexGuard};

use gateway::{Custodian, Persistence};
use kernel::{AxCode, AxError, B3Hash, SecretRef};
use remote_access::keys::{self, SEED_BYTES, SigningKey};
use zeroize::Zeroizing;

use remote_access::door::Device;

use super::{Doorway, Entropy};

/// The realm every city key is kept under.
const REALM: &str = "remote";
/// What precedes the city's genesis id in the reference's name.
const NAME_PREFIX: &str = "city-key.";

/// Where this city's key lives, and the vault that keeps it.
pub(crate) struct CityKey {
    vault: Arc<Mutex<Custodian>>,
    reference: SecretRef,
}

impl CityKey {
    /// The key of the city whose ledger begins with `epoch`, kept in
    /// `vault`. Two cities never share a reference, because no two share
    /// a genesis line.
    ///
    /// # Errors
    /// `E_CONFIG_INVALID` when the ledger has no genesis line yet.
    pub(crate) fn of(
        vault: Arc<Mutex<Custodian>>,
        epoch: Option<B3Hash>,
    ) -> Result<CityKey, AxError> {
        let epoch = epoch.ok_or_else(|| {
            AxError::failure(
                AxCode::ConfigInvalid,
                "name the city key",
                "a ledger with no genesis line",
            )
            .with_recovery("restart the city once its ledger has begun")
        })?;
        Ok(CityKey {
            vault,
            reference: SecretRef::new(REALM, &format!("{NAME_PREFIX}{epoch}"))?,
        })
    }

    /// The key the vault keeps, written the first time this city asks.
    ///
    /// # Errors
    /// A vault that refuses for any reason other than holding nothing; a
    /// random source that refuses; a seed that does not read back.
    pub(super) fn held(&self, entropy: &Entropy) -> Result<SigningKey, AxError> {
        match self.read() {
            Err(missing) if *missing.code() == AxCode::CredentialMissing => self.replaced(entropy),
            read => read,
        }
    }

    /// A key from a fresh seed, written over whatever the vault kept.
    ///
    /// # Errors
    /// A random source or a vault that refuses; nothing has changed then.
    pub(super) fn replaced(&self, entropy: &Entropy) -> Result<SigningKey, AxError> {
        let mut seed = Zeroizing::new([0u8; SEED_BYTES]);
        entropy(seed.as_mut_slice())?;
        self.vault()?.set(&self.reference, keys::written(&seed))?;
        self.read()
    }

    /// How long this machine's vault keeps the key, and so how long a
    /// paired device stays paired.
    ///
    /// # Errors
    /// The vault's lock was poisoned.
    pub(crate) fn lasting(&self) -> Result<Persistence, AxError> {
        Ok(self.vault()?.persistence())
    }

    fn read(&self) -> Result<SigningKey, AxError> {
        SigningKey::from_sealed(&self.vault()?.resolve(&self.reference)?)
    }

    fn vault(&self) -> Result<MutexGuard<'_, Custodian>, AxError> {
        self.vault.lock().map_err(|_| {
            AxError::failure(
                AxCode::StorageFatal,
                "reach the vault for the city key",
                "a thread ended in a panic while holding it",
            )
            .with_recovery("restart the city; the key the vault keeps is read again")
        })
    }
}

impl Doorway {
    /// Replaces the city key with one from a fresh seed and revokes every
    /// paired device, since each pins the old key: the devices answered
    /// are the ones that pair again.
    ///
    /// # Errors
    /// `E_BUSY` while the door is open, because its sessions were
    /// shaken hands with the old key; a vault or random source that
    /// refuses, and nothing has changed then; a table or a line that
    /// cannot be written.
    pub(crate) fn replace_key(&self) -> Result<Vec<Device>, AxError> {
        let now = (self.senses.clock)()?;
        let mut kept = self.kept()?;
        if let Some(standing) = &kept.standing {
            return Err(AxError::failure(
                AxCode::Busy,
                "replace the city key",
                format!(
                    "the remote door is open at {}",
                    standing.opened.url.as_str()
                ),
            )
            .with_recovery("close it first with `/remote close`, then replace the key"));
        }
        kept.city = Ok(kept.key.replaced(&self.senses.entropy)?);
        let paired = kept.door.devices().to_vec();
        kept.revoked(now, paired)
    }

    /// How long this machine keeps the city key.
    ///
    /// # Errors
    /// A lock that was poisoned.
    pub(crate) fn key_lasting(&self) -> Result<Persistence, AxError> {
        self.kept()?.key.lasting()
    }
}
