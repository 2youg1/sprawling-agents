// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A key a resident writes into a file goes to the vault, and the file
//! holds its `secret:` reference instead (sprawling-SPEC.md 8-87).

use std::sync::{Arc, Mutex};

use kernel::{
    AxCode, AxError, GateSubject, Payload, SecretRef, Tool, ToolCall, ToolMeta, ToolOutcome,
};
use serde_json::Value;
use zeroize::Zeroizing;

use super::super::super::dispatching::custody::kept_text;
use crate::held_vault::poisoned_vault;

#[cfg(test)]
mod tests;

/// The realm every key a resident writes is kept under.
const REALM: &str = "written";

/// The `edit` tool with custody in front of it: the `new` text reaches
/// the file with every provider-shaped key replaced by its reference.
pub(in crate::assembly) struct KeptEdit {
    edit: runtime::EditTool,
    vault: Arc<Mutex<gateway::Custodian>>,
    /// The ledger position when this run's bench was laid out: no two
    /// runs share it, because each writes before its bench is laid out.
    run: u64,
    /// How many keys this run has written so far.
    written: u64,
}

impl KeptEdit {
    pub(in crate::assembly) fn new(
        edit: runtime::EditTool,
        vault: Arc<Mutex<gateway::Custodian>>,
        run: u64,
    ) -> KeptEdit {
        KeptEdit {
            edit,
            vault,
            run,
            written: 0,
        }
    }

    /// Puts one written key in the vault under a name no earlier key of
    /// this run or of any other run holds.
    fn keep(&mut self, provider: &str, key: &str) -> Result<SecretRef, AxError> {
        self.written = self.written.checked_add(1).ok_or_else(|| {
            AxError::failure(
                AxCode::InvalidArgs,
                "keep a written key",
                "this run has written more keys than a counter holds".to_owned(),
            )
            .with_recovery("start a new run and write the key there")
        })?;
        let reference =
            SecretRef::new(REALM, &format!("{provider}-{}-{}", self.run, self.written))?;
        self.vault
            .lock()
            .map_err(|_| poisoned_vault())?
            .set(&reference, Zeroizing::new(key.to_owned()))?;
        Ok(reference)
    }
}

impl Tool for KeptEdit {
    fn meta(&self) -> &ToolMeta {
        self.edit.meta()
    }

    /// # Errors
    /// Everything `edit` refuses, and the vault refusing a key: then the
    /// file is left as it was, because writing the key is the leak.
    fn invoke(&mut self, call: &ToolCall) -> Result<ToolOutcome, AxError> {
        let Some(Value::String(new)) = call.args.as_map().get("new") else {
            return self.edit.invoke(call);
        };
        let Some(kept) = kept_text(new, |provider, key| self.keep(provider, key))? else {
            return self.edit.invoke(call);
        };
        let mut args = call.args.as_map().clone();
        args.insert("new".to_owned(), Value::String(kept));
        self.edit.invoke(&ToolCall {
            id: call.id.clone(),
            name: call.name.clone(),
            args: Payload::new(args)?,
        })
    }

    fn subject(&self, call: &ToolCall) -> Result<GateSubject, AxError> {
        self.edit.subject(call)
    }
}
