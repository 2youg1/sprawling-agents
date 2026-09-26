// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Every tool on the bench with custody on both sides of it: a key in
//! the arguments reaches the tool as its `secret:` reference, and a key
//! in the result reaches the model the same way (sprawling-SPEC.md 8-87).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use kernel::{
    AxCode, AxError, GateSubject, Payload, SecretRef, Tool, ToolCall, ToolMeta, ToolOutcome,
};
use serde_json::{Map, Value};
use zeroize::Zeroizing;

use super::super::super::credentials::poisoned_vault;
use super::super::super::dispatching::custody::kept_text;

#[cfg(test)]
mod tests;

/// Which side of a tool a key was found on, and so the realm it is kept
/// under.
#[derive(Clone, Copy)]
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

/// The vault and the naming every tool of one run shares, so that no
/// two tools keep two keys under one name.
pub(in crate::assembly) struct Keeper {
    vault: Arc<Mutex<gateway::Custodian>>,
    /// The ledger position when this run's bench was laid out: no two
    /// runs share it, because each writes before its bench is laid out.
    run: u64,
    /// How many keys this run's tools have kept so far.
    kept: AtomicU64,
}

impl Keeper {
    pub(in crate::assembly) fn new(vault: Arc<Mutex<gateway::Custodian>>, run: u64) -> Keeper {
        Keeper {
            vault,
            run,
            kept: AtomicU64::new(0),
        }
    }

    /// Puts one key in the vault under a name no earlier key of this run
    /// or of any other run holds.
    fn keep(&self, side: Side, provider: &str, key: &str) -> Result<SecretRef, AxError> {
        let n = self
            .kept
            .fetch_add(1, Ordering::Relaxed)
            .checked_add(1)
            .ok_or_else(|| {
                AxError::failure(
                    AxCode::InvalidArgs,
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
        Ok(reference)
    }

    /// `value` with every key in every string kept, or `None` when no
    /// string held one, so an untouched value is handed on uncopied.
    fn kept_value(&self, side: Side, value: &Value) -> Result<Option<Value>, AxError> {
        match value {
            Value::String(text) => {
                Ok(
                    kept_text(text, |provider, key| self.keep(side, provider, key))?
                        .map(Value::String),
                )
            }
            Value::Array(items) => {
                let mut changed = None;
                for (at, item) in items.iter().enumerate() {
                    if let Some(kept) = self.kept_value(side, item)? {
                        if let Some(slot) = changed.get_or_insert_with(|| items.clone()).get_mut(at)
                        {
                            *slot = kept;
                        }
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
    /// Everything the tool refuses, and the vault refusing a key: a key
    /// in the arguments then stops the tool before it runs, and a key in
    /// the result stops the result before the model reads it.
    fn invoke(&mut self, call: &ToolCall) -> Result<ToolOutcome, AxError> {
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

    /// The file `edit` names: custody rewrites the text, not the path.
    fn writes(&self, call: &ToolCall) -> kernel::Writes {
        self.edit.writes(call)
    }
}
