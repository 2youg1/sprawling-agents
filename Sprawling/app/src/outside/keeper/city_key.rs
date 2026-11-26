// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::{Arc, Mutex, MutexGuard};

use gateway::{Custodian, Persistence};
use kernel::{AxCode, AxError, B3Hash, SecretRef};
use remote_access::keys::{self, SEED_BYTES, SigningKey};
use zeroize::Zeroizing;

use remote_access::door::Device;

use super::{Doorway, Entropy};

const REALM: &str = "remote";

const NAME_PREFIX: &str = "city-key.";

pub(crate) struct CityKey {
    vault: Arc<Mutex<Custodian>>,
    reference: SecretRef,
}

impl CityKey {
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

    pub(super) fn held(&self, entropy: &Entropy) -> Result<SigningKey, AxError> {
        match self.read() {
            Err(missing) if *missing.code() == AxCode::CredentialMissing => self.replaced(entropy),
            read => read,
        }
    }

    pub(super) fn replaced(&self, entropy: &Entropy) -> Result<SigningKey, AxError> {
        let mut seed = Zeroizing::new([0u8; SEED_BYTES]);
        entropy(seed.as_mut_slice())?;
        self.vault()?.set(&self.reference, keys::written(&seed))?;
        self.read()
    }

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
    pub(crate) fn replace_key(&self) -> Result<Vec<Device>, AxError> {
        let now = (self.senses.clock)()?;
        let mut kept = self.kept()?;
        if let Some(standing) = &kept.standing {
            return Err(AxError::failure(
                AxCode::Busy,
                "replace the city key",
                format!("the remote door is open at {}", standing.origin.as_str()),
            )
            .with_recovery("close it first with `/remote close`, then replace the key"));
        }
        kept.city = Ok(kept.key.replaced(&self.senses.entropy)?);
        let paired = kept.door.devices().to_vec();
        kept.revoked(now, paired)
    }

    pub(crate) fn key_lasting(&self) -> Result<Persistence, AxError> {
        self.kept()?.key.lasting()
    }
}
