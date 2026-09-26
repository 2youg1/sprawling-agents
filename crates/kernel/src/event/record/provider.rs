// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! `provider_degraded`: something this city leans on answered less than
//! it should, and the line says which of two things that was.

use serde::{Deserialize, Serialize};

use crate::error::AxError;

/// `provider_degraded` has two writers with two shapes.
///
/// The carrier table in `kernel::error` sends `E_PROVIDER` here, written
/// as the error itself; the vault's startup probe writes its fallback to
/// session memory. A reader tells them apart by reading, not by guessing
/// from a key, and a line that is neither does not read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum ProviderDegraded {
    /// A provider refused a call; this is the refusal, flat.
    Refused(AxError),
    /// The platform credential service failed its startup round trip.
    VaultFellBack(VaultFellBack),
}

/// Tries the refusal, then the fallback, and when neither reads names
/// what each shape missed: a derived untagged enum reports only that no
/// variant matched, which leaves the reader of an unreadable line without
/// the field to look at.
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
                            "neither a provider refusal ({as_refusal}) nor a vault fallback                              ({as_fallback})"
                        ))
                    })
            })
    }
}

/// The vault's fallback, as the startup probe writes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct VaultFellBack {
    /// Which part degraded; the probe writes `vault`.
    pub component: String,
    /// Where credentials rest instead; the probe writes `session-memory`.
    pub fallback: String,
    /// `gateway::Persistence` in its own spelling.
    pub persistence: String,
    /// Why the platform service was not used.
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
            err.contains("missing field `action`") && err.contains("missing field `fallback`"),
            "{err}"
        );
    }
}
