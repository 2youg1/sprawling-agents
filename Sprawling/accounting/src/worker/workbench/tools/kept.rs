// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use kernel::{
    AxCode, AxError, B3Hash, GateSubject, Payload, SecretRef, Tool, ToolCall, ToolMeta, ToolOutcome,
};
use serde_json::{Map, Value};
use zeroize::Zeroizing;

use super::super::super::dispatching::custody::kept_text;
use crate::held_vault::poisoned_vault;

#[cfg(test)]
mod tests;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Side {
    Written,

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

enum InPlace {
    Kept(SecretRef),

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

pub(in crate::worker) struct Keeper {
    vault: Arc<Mutex<gateway::Custodian>>,

    run: u64,

    kept: Mutex<BTreeMap<(Side, B3Hash), SecretRef>>,
}

impl Keeper {
    pub(in crate::worker) fn new(vault: Arc<Mutex<gateway::Custodian>>, run: u64) -> Keeper {
        Keeper {
            vault,
            run,
            kept: Mutex::new(BTreeMap::new()),
        }
    }

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

    fn in_place(&self, side: Side, provider: &str, key: &str) -> Result<InPlace, AxError> {
        match (side, self.keep(side, provider, key)) {
            (Side::Written | Side::Output, Ok(reference)) => Ok(InPlace::Kept(reference)),
            (Side::Written, Err(err)) => Err(err),
            (Side::Output, Err(err)) => Ok(InPlace::Withheld(*err.code())),
        }
    }

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

    fn kept_payload(&self, side: Side, payload: &Payload) -> Result<Option<Payload>, AxError> {
        self.kept_map(side, payload.as_map())?
            .map(Payload::new)
            .transpose()
    }
}

pub(in crate::worker) struct Kept {
    tool: Box<dyn Tool>,
    keeper: Arc<Keeper>,
}

impl Kept {
    pub(in crate::worker) fn new(tool: Box<dyn Tool>, keeper: Arc<Keeper>) -> Kept {
        Kept { tool, keeper }
    }
}

impl Tool for Kept {
    fn meta(&self) -> &ToolMeta {
        self.tool.meta()
    }

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

    fn writes(&self, call: &ToolCall) -> kernel::Writes {
        self.tool.writes(call)
    }
}
