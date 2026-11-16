// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::Arc;

use kernel::config::SearchConfiguration;
use kernel::event::record::SecretCaptured;
use kernel::{AxCode, AxError, EventKind, Payload, SecretRef};

use crate::held_vault::{poisoned_vault, resolving};

use super::super::RunWorker;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::worker) enum Arrival {
    Enrolment,

    Pasted,
}

impl Arrival {
    fn spelling(self) -> &'static str {
        match self {
            Arrival::Enrolment => "enrolment",
            Arrival::Pasted => "pasted",
        }
    }
}

impl RunWorker {
    pub(in crate::worker) fn put_secret(
        &mut self,
        reference: &kernel::SecretRef,
        value: kernel::Sealed<String>,
        arrival: Arrival,
    ) -> Result<(), AxError> {
        {
            let mut vault = self
                .credentials
                .vault
                .lock()
                .map_err(|_| poisoned_vault())?;
            vault.set(reference, value.into_vault_value())?;
        }

        self.connectors.invalidate(reference);
        let captured = SecretCaptured {
            reference: reference.clone(),
            origin: arrival.spelling().to_owned(),
        };
        self.record(EventKind::SecretCaptured, Payload::of(&captured)?)
    }

    pub(in crate::worker) fn forget_secret(&mut self, reference: &str) -> Result<(), AxError> {
        let reference = kernel::SecretRef::parse(reference)?;
        if let Some(holder) = self.named_by(&reference)? {
            return Err(AxError::failure(
                AxCode::ConfigInvalid,
                "forget credential",
                format!("{reference} is still named by {holder}"),
            )
            .with_recovery(format!(
                "remove {reference} from {holder} first, then forget the key"
            )));
        }
        self.credentials
            .vault
            .lock()
            .map_err(|_| poisoned_vault())?
            .forget(&reference)?;

        self.connectors.invalidate(&reference);
        Ok(())
    }

    fn named_by(&self, reference: &kernel::SecretRef) -> Result<Option<String>, AxError> {
        if let Some(endpoint) = self
            .credentials
            .book
            .endpoints()
            .find(|endpoint| endpoint.references().contains(&reference))
        {
            return Ok(Some(format!("provider `{}`", endpoint.name)));
        }
        if let Some(supplier) = city::city_search(&self.city_root)?
            .as_ref()
            .and_then(|search| supplier_naming(search, reference))
        {
            return Ok(Some(format!("the city's [search] supplier `{supplier}`")));
        }
        for building in city::buildings(&self.city_root)? {
            let frozen = city::load_config(&self.city_root, &building)?;
            let at = building.as_str();
            if let Some(server) = frozen
                .mcp
                .iter()
                .find(|server| super::super::mcp::names(server, reference))
            {
                return Ok(Some(format!(
                    "[[mcp]] `{}` at `{at}`",
                    server.label.as_str()
                )));
            }
            if let Some(supplier) = supplier_naming(&frozen.search, reference) {
                return Ok(Some(format!("[search] supplier `{supplier}` at `{at}`")));
            }
        }
        Ok(None)
    }

    pub(in crate::worker) fn resolver(&self) -> gateway::SecretResolver {
        resolving(Arc::clone(&self.credentials.vault))
    }

    pub fn vault_handle(&self) -> Arc<std::sync::Mutex<gateway::Custodian>> {
        Arc::clone(&self.credentials.vault)
    }
}

fn supplier_naming<'a>(search: &'a SearchConfiguration, reference: &SecretRef) -> Option<&'a str> {
    match search {
        SearchConfiguration::Custom { suppliers, .. } => suppliers
            .iter()
            .find(|supplier| {
                supplier
                    .accounts
                    .iter()
                    .any(|account| account.reference.as_ref() == Some(reference))
            })
            .map(|supplier| supplier.id.as_str()),
        SearchConfiguration::Default | SearchConfiguration::Off => None,
    }
}
