// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Every tool on the bench with custody on both sides of it: a key in
//! the arguments reaches the tool as its `secret:` reference, and a key
//! in the result reaches the model the same way (sprawling-SPEC.md 8-87).

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use kernel::{
    AxCode, AxError, B3Hash, GateSubject, Payload, SecretRef, Tool, ToolCall, ToolMeta, ToolOutcome,
};
use serde_json::{Map, Value};
use zeroize::Zeroizing;

use super::super::super::dispatching::custody::kept_text;
use accounting::held_vault::poisoned_vault;

#[cfg(test)]
mod tests;

/// Which side of a tool a key was found on, and so the realm it is kept
/// under.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Side {
    /// The arguments the model wrote: what `edit` and `exec` would write.
    Written,
    /// The result a tool returned: what the model would read.
    Output,
}

impl Side {
    fn realm(self) -> &'static str {
        match self {
            Side::Written => "written",
            Side::Output => "output",
        }
    }
}

/// What a key found in a tool's arguments or result is replaced with.
enum InPlace {
    Kept(SecretRef),
    /// The vault refused a key in a result the tool has already produced:
    /// the model reads this marker, never the key, and the call stands.
    Withheld(AxCode),
}

impl std::fmt::Display for InPlace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InPlace::Kept(reference) => write!(f, "{reference}"),
            InPlace::Withheld(code) => write!(f, "[key withheld: {code}]"),
        }
    }
}

/// The vault and the naming every tool of one run shares, so that no
/// two tools keep two keys under one name.
pub(in crate::assembly) struct Keeper {
    vault: Arc<Mutex<gateway::Custodian>>,
    /// The ledger position when this run's bench was laid out: no two
    /// runs share it, because each writes before its bench is laid out.
    run: u64,
    /// The reference each distinct key this run's tools have kept holds,
    /// found by the key's digest so that no plaintext outlives the call.
    kept: Mutex<BTreeMap<(Side, B3Hash), SecretRef>>,
}

impl Keeper {
    pub(in crate::assembly) fn new(vault: Arc<Mutex<gateway::Custodian>>, run: u64) -> Keeper {
        Keeper {
            vault,
            run,
            kept: Mutex::new(BTreeMap::new()),
        }
    }

    /// The reference `key` is kept under: the one it already holds in
    /// this run, or a new vault entry under a name no other key of this
    /// run or of any other run holds.
    fn keep(&self, side: Side, provider: &str, key: &str) -> Result<SecretRef, AxError> {
        let mut kept = self.kept.lock().map_err(|_| poisoned_vault())?;
        let digest = (side, B3Hash::digest(key.as_bytes()));
        if let Some(reference) = kept.get(&digest) {
            return Ok(reference.clone());
        }
        let n = u64::try_from(kept.len())
            .ok()
            .and_then(|len| len.checked_add(1))
            .ok_or_else(|| {
                AxError::failure(
                    AxCode::BudgetExhausted,
                    "keep a key a tool carried",
                    "this run has kept more keys than a counter holds".to_owned(),
                )
                .with_recovery("start a new run and use the key there")
            })?;
        let reference = SecretRef::new(side.realm(), &format!("{provider}-{}-{n}", self.run))?;
        self.vault
            .lock()
            .map_err(|_| poisoned_vault())?
            .set(&reference, Zeroizing::new(key.to_owned()))?;
        kept.insert(digest, reference.clone());
        Ok(reference)
    }

    /// What stands in `key`'s place. A refused key in the arguments stops
    /// the call before the tool acts; a refused key in a result is
    /// withheld, because the tool has already acted and an error would
    /// invite the model to act again.
    fn in_place(&self, side: Side, provider: &str, key: &str) -> Result<InPlace, AxError> {
        match (side, self.keep(side, provider, key)) {
            (Side::Written | Side::Output, Ok(reference)) => Ok(InPlace::Kept(reference)),
            (Side::Written, Err(err)) => Err(err),
            (Side::Output, Err(err)) => Ok(InPlace::Withheld(*err.code())),
        }
    }

    /// `value` with every key in every string kept, or `None` when no
    /// string held one, so an untouched value is handed on uncopied.
    fn kept_value(&self, side: Side, value: &Value) -> Result<Option<Value>, AxError> {
        match value {
            Value::String(text) => {
                Ok(
                    kept_text(text, |provider, key| self.in_place(side, provider, key))?
                        .map(Value::String),
                )
            }
            Value::Array(items) => {
                let mut changed = None;
                for (at, item) in items.iter().enumerate() {
                    if let Some(kept) = self.kept_value(side, item)?
                        && let Some(slot) = changed.get_or_insert_with(|| items.clone()).get_mut(at)
                    {
                        *slot = kept;
                    }
                }
                Ok(changed.map(Value::Array))
            }
            Value::Object(map) => Ok(self.kept_map(side, map)?.map(Value::Object)),
            Value::Null | Value::Bool(_) | Value::Number(_) => Ok(None),
        }
    }

    fn kept_map(
        &self,
        side: Side,
        map: &Map<String, Value>,
    ) -> Result<Option<Map<String, Value>>, AxError> {
        let mut changed: Option<Map<String, Value>> = None;
        for (name, value) in map {
            if let Some(kept) = self.kept_value(side, value)? {
                changed
                    .get_or_insert_with(|| map.clone())
                    .insert(name.clone(), kept);
            }
        }
        Ok(changed)
    }

    /// `payload` with every key kept, or `None` when it held none.
    fn kept_payload(&self, side: Side, payload: &Payload) -> Result<Option<Payload>, AxError> {
        self.kept_map(side, payload.as_map())?
            .map(Payload::new)
            .transpose()
    }
}

/// One tool with custody in front of and behind it.
pub(in crate::assembly) struct Kept {
    tool: Box<dyn Tool>,
    keeper: Arc<Keeper>,
}

impl Kept {
    pub(in crate::assembly) fn new(tool: Box<dyn Tool>, keeper: Arc<Keeper>) -> Kept {
        Kept { tool, keeper }
    }
}

impl Tool for Kept {
    fn meta(&self) -> &ToolMeta {
        self.tool.meta()
    }

    /// # Errors
    /// Everything the tool refuses, and the vault refusing a key in the
    /// arguments, which stops the tool before it runs. A key the vault
    /// refuses in the result is withheld instead (`InPlace::Withheld`).
    fn invoke(&self, call: &ToolCall) -> Result<ToolOutcome, AxError> {
        let outcome = match self.keeper.kept_payload(Side::Written, &call.args)? {
            Some(args) => self.tool.invoke(&ToolCall {
                id: call.id.clone(),
                name: call.name.clone(),
                args,
            })?,
            None => self.tool.invoke(call)?,
        };
        match self.keeper.kept_payload(Side::Output, &outcome.result)? {
            Some(result) => Ok(ToolOutcome { result, ..outcome }),
            None => Ok(outcome),
        }
    }

    fn subject(&self, call: &ToolCall) -> Result<GateSubject, AxError> {
        self.tool.subject(call)
    }

    /// What the kept tool writes: custody rewrites the text, not the
    /// path.
    fn writes(&self, call: &ToolCall) -> kernel::Writes {
        self.tool.writes(call)
    }
}
