// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeSet;
use std::sync::Arc;

use kernel::Address;
use serde_json::{Map, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleCandidate {
    pub room: Address,
    pub text: String,
    pub dropped: u64,
}

impl RuleCandidate {
    #[must_use]
    pub fn into_json(self) -> Value {
        let mut rule = Map::new();
        rule.insert(
            "room".to_owned(),
            Value::String(self.room.as_str().to_owned()),
        );
        rule.insert("text".to_owned(), Value::String(self.text));
        Value::Object(rule)
    }

    #[must_use]
    pub fn from_json(value: &Value) -> Option<RuleCandidate> {
        let room = value.get("room")?.as_str()?;
        let room = match Address::parse(room) {
            Ok(room) => room,
            Err(_) => return None,
        };
        let text = value.get("text")?.as_str()?.to_owned();
        let dropped = value.get("dropped").and_then(Value::as_u64).unwrap_or(0);
        Some(RuleCandidate {
            room,
            text,
            dropped,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleContext {
    pub admitted: Address,
    pub landed: Address,
}

impl RuleContext {
    pub(super) fn into_json(self) -> Value {
        let mut context = Map::new();
        context.insert(
            "admitted".to_owned(),
            Value::String(self.admitted.as_str().to_owned()),
        );
        context.insert(
            "landed".to_owned(),
            Value::String(self.landed.as_str().to_owned()),
        );
        Value::Object(context)
    }

    #[must_use]
    pub fn from_json(value: &Value) -> Option<RuleContext> {
        let admitted = value.get("admitted")?.as_str()?;
        let landed = value.get("landed")?.as_str()?;
        match (Address::parse(admitted), Address::parse(landed)) {
            (Ok(admitted), Ok(landed)) => Some(RuleContext { admitted, landed }),
            _ => None,
        }
    }
}

pub type RuleCandidates = Arc<
    dyn Fn(&Address, &Address, &BTreeSet<Address>, &mut dyn FnMut() -> bool) -> Vec<RuleCandidate>
        + Send
        + Sync,
>;
