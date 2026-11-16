// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

use crate::error::AxError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum ProviderDegraded {
    Refused(AxError),

    VaultFellBack(VaultFellBack),
}

impl<'de> Deserialize<'de> for ProviderDegraded {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let line = serde_json::Value::deserialize(deserializer)?;
        AxError::deserialize(&line)
            .map(Self::Refused)
            .or_else(|as_refusal| {
                VaultFellBack::deserialize(&line)
                    .map(Self::VaultFellBack)
                    .map_err(|as_fallback| {
                        serde::de::Error::custom(format!(
                            "neither a provider refusal ({as_refusal}) nor a vault fallback \
                             ({as_fallback})"
                        ))
                    })
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct VaultFellBack {
    pub component: String,

    pub fallback: String,

    pub persistence: String,

    pub reason: String,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;
    use crate::event::Payload;

    #[test]
    fn a_vault_line_keeps_the_bytes_its_ledger_already_holds() {
        let line = r#"{"component":"vault","fallback":"session-memory","persistence":"this-process","reason":"probe reference unparsable"}"#;
        let read: ProviderDegraded = serde_json::from_str::<Payload>(line)
            .unwrap()
            .read()
            .unwrap();
        assert!(matches!(read, ProviderDegraded::VaultFellBack(_)));
        assert_eq!(
            serde_json::to_string(&Payload::of(&read).unwrap()).unwrap(),
            line
        );
    }

    #[test]
    fn a_line_that_is_neither_shape_names_what_each_shape_missed() {
        let line = r#"{"code":"E_PROVIDER","component":"vault"}"#;
        let err = serde_json::from_str::<Payload>(line)
            .unwrap()
            .read::<ProviderDegraded>()
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("neither a provider refusal (missing field `action`)")
                && err.contains("nor a vault fallback (missing field `fallback`)"),
            "{err}"
        );
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ProviderAccount {
    pub id: crate::ServerLabel,
    #[serde(default)]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub reference: Option<crate::SecretRef>,
    #[serde(default)]
    pub header: Option<String>,
}

pub fn validate_provider_accounts(
    accounts: Option<&[ProviderAccount]>,
) -> Result<(), crate::AxError> {
    let Some(accounts) = accounts else {
        return Ok(());
    };
    let refuse = |subject: &str| {
        crate::AxError::failure(
        crate::AxCode::ConfigInvalid, "register provider accounts", subject,
    ).with_recovery("give each account a distinct id and vault reference; use an explicit anonymous account for an unkeyed provider")
    };
    if accounts.is_empty() {
        return Err(refuse("the account list is empty"));
    }
    let mut ids = std::collections::BTreeSet::new();
    let mut references = std::collections::BTreeSet::new();
    for account in accounts {
        if !ids.insert(account.id.as_str()) {
            return Err(refuse("an account id is repeated"));
        }
        if let Some(reference) = &account.reference {
            if !references.insert(reference) {
                return Err(refuse("a vault reference is repeated"));
            }
        } else if account.header.is_some() {
            return Err(refuse(
                "an anonymous account cannot carry a credential header",
            ));
        }
        if let Some(header) = &account.header
            && (header.is_empty()
                || !header
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte)))
        {
            return Err(refuse("an account header name is invalid"));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ProviderAccountBinding {
    pub provider: String,
    pub account: crate::ServerLabel,
}
