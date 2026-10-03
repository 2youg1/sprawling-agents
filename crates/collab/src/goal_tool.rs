// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The goal tool: the face `kernel::goal` and `collab::arbiter` show a
//! model.
//!
//! Three layers, none of them duplicated here. Detection answers whether
//! two goals clash; arbitration answers who settles it; this module only
//! joins them into something a resident can call, and refuses to
//! register anything that clashed.
//!
//! The register is the city's, not the desk's: two runs dispatched side
//! by side read the same register, so the desk hands each entry to the
//! authority every run asks ([`GoalBooking`]) and keeps no copy.
//!
//! A refusal is where the design lives. "No" alone teaches a model to
//! rephrase and try again, so the refusal carries the level that decides
//! the clash — wait for the other goal, or go and agree with its owner
//! (collab D1 leaves no third level) — and the model can act on exactly
//! the one it is given.

use std::sync::{Arc, Mutex};

use kernel::{
    Address, AxCode, AxError, CostTier, Effect, GoalEntry, GoalId, GoalResource, Payload,
    RenderIntent, RunId, Temporal, Tool, ToolCall, ToolMeta, ToolName, ToolOutcome,
};
use serde_json::{Map, Value};

use crate::arbiter::Level;

/// The authority that decides a registration at the call: it arbitrates
/// the entry against the whole register, records the outcome, and
/// answers. In the city that authority is the accounting thread, reached
/// through the relay.
pub struct GoalBooking(Box<Ask>);

/// Registers one entry, or refuses it with [`conflict_refusal`] because
/// the ground is held. Handed the whole entry, because the authority
/// records it as it registers it.
type Ask = dyn FnMut(&GoalEntry) -> Result<(), AxError> + Send;

impl GoalBooking {
    /// `ask`'s refusal reaches the model unchanged.
    #[must_use]
    pub fn new(ask: impl FnMut(&GoalEntry) -> Result<(), AxError> + Send + 'static) -> GoalBooking {
        GoalBooking(Box::new(ask))
    }
}

impl std::fmt::Debug for GoalBooking {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("GoalBooking")
    }
}

/// The run's side of the goal register: who registers, the ids it mints,
/// and the authority that decides each registration.
#[derive(Debug)]
pub struct GoalDesk {
    run: RunId,
    owner: String,
    minted: u32,
    booking: GoalBooking,
}

impl GoalDesk {
    #[must_use]
    pub fn new(run: RunId, owner: String, booking: GoalBooking) -> GoalDesk {
        GoalDesk {
            run,
            owner,
            minted: 0,
            booking,
        }
    }

    fn mint(&mut self) -> Result<GoalId, AxError> {
        self.minted = self.minted.checked_add(1).ok_or_else(|| {
            AxError::failure(
                AxCode::InvalidArgs,
                "mint a goal id",
                "this run has registered as many goals as one run can",
            )
            .with_recovery("freeze the run and dispatch again")
        })?;
        GoalId::new(format!("{}-g{}", self.run, self.minted)).ok_or_else(|| {
            AxError::failure(AxCode::InvalidArgs, "mint a goal id", "empty id")
                .with_recovery("this cannot happen with a run id in hand; report it")
        })
    }

    fn register(&mut self, args: &Map<String, Value>) -> Result<Payload, AxError> {
        let statement = text(args, "statement")?.to_owned();
        let mut resources = Vec::new();
        for raw in strings(args, "paths") {
            resources.push(GoalResource::Path(Address::parse(&raw)?));
        }
        for raw in strings(args, "externals") {
            resources.push(GoalResource::External(raw));
        }
        if resources.is_empty() {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "register a goal",
                "no resources named",
            )
            .with_recovery(
                "name at least one path or external resource; a goal that claims nothing \
                 cannot keep anyone off it",
            ));
        }
        let entry = GoalEntry {
            id: self.mint()?,
            owner: self.owner.clone(),
            resources,
            statement,
            standing: args
                .get("standing")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        };
        (self.booking.0)(&entry)?;
        let mut result = Map::new();
        result.insert("id".to_owned(), Value::String(entry.id.as_str().to_owned()));
        result.insert("registered".to_owned(), Value::Bool(true));
        Payload::new(result)
    }
}

/// The refusal a model is handed when `entry` clashes: `E_GOAL_CONFLICT`,
/// with the one move the level that decides the clash leaves it. Spelled
/// here alone, so the authority that arbitrates says it the one way.
#[must_use]
pub fn conflict_refusal(entry: &GoalEntry, level: &Level) -> AxError {
    AxError::failure(
        AxCode::GoalConflict,
        "register a goal",
        entry.statement.clone(),
    )
    .with_recovery(next_move(level))
}

/// The third part of the refusal: one thing to do, named after the level
/// that decides the clash.
fn next_move(level: &Level) -> String {
    match level {
        Level::Serialize { after } => format!(
            "`{}` holds this ground and is the standing goal; wait for it, then register again",
            after.as_str()
        ),
        Level::Arbitrate { with } => format!(
            "signal the resident who registered `{}` and agree which of you takes it",
            with.as_str()
        ),
    }
}

pub struct GoalTool {
    meta: ToolMeta,
    desk: Arc<Mutex<GoalDesk>>,
}

impl GoalTool {
    /// `room` is what the tool declares it writes into; the register
    /// itself is the city's, and the gate that matters is the clash.
    ///
    /// # Errors
    /// Propagates a malformed tool name or parameter schema.
    pub fn new(room: Address, desk: Arc<Mutex<GoalDesk>>) -> Result<GoalTool, AxError> {
        let mut properties = Map::new();
        for (field, kind, description) in [
            (
                "statement",
                "string",
                "what you intend to do, in one sentence",
            ),
            (
                "paths",
                "array",
                "addresses you will be working in; overlapping ones clash",
            ),
            (
                "externals",
                "array",
                "named resources outside the city that only one goal may hold",
            ),
            (
                "standing",
                "boolean",
                "true for ongoing responsibility, false for one piece of work",
            ),
        ] {
            let mut spec = Map::new();
            spec.insert("type".to_owned(), Value::String(kind.to_owned()));
            spec.insert(
                "description".to_owned(),
                Value::String(description.to_owned()),
            );
            properties.insert(field.to_owned(), Value::Object(spec));
        }
        let mut params = Map::new();
        params.insert("type".to_owned(), Value::String("object".to_owned()));
        params.insert("properties".to_owned(), Value::Object(properties));
        params.insert(
            "required".to_owned(),
            Value::Array(vec![Value::String("statement".to_owned())]),
        );
        Ok(GoalTool {
            meta: ToolMeta {
                name: ToolName::parse("goal")?,
                disclosure:
                    "Claim the ground you are about to work on, so two residents do not edit the \
                     same thing; refused if someone holds it already."
                        .to_owned(),
                params: Payload::new(params)?,
                effect: Effect::Write { domain: room },
                cost_tier: CostTier::Free,
                timeout: None,
                render: RenderIntent::Generic,
                temporal: Temporal::Timeless,
            },
            desk,
        })
    }
}

impl Tool for GoalTool {
    fn meta(&self) -> &ToolMeta {
        &self.meta
    }

    fn invoke(&self, call: &ToolCall) -> Result<ToolOutcome, AxError> {
        if call.name != self.meta.name {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "register a goal",
                format!("call routed to the wrong tool: {}", call.name.as_str()),
            )
            .with_recovery(format!(
                "call `{}`, the name this tool answers to",
                self.meta.name.as_str()
            )));
        }
        let mut desk = self.desk.lock().map_err(|_| {
            AxError::failure(
                AxCode::StorageFatal,
                "reach the goal register",
                "the register is already in use",
            )
            .with_recovery("restart this city")
        })?;
        let result = desk.register(call.args.as_map())?;
        Ok(ToolOutcome {
            result,
            attachments: Vec::new(),
        })
    }
}

fn text<'a>(args: &'a Map<String, Value>, key: &str) -> Result<&'a str, AxError> {
    args.get(key).and_then(Value::as_str).ok_or_else(|| {
        AxError::failure(
            AxCode::InvalidArgs,
            "register a goal",
            format!("missing string argument `{key}`"),
        )
        .with_recovery(format!("pass `{key}` as a string"))
    })
}

fn strings(args: &Map<String, Value>, key: &str) -> Vec<String> {
    args.get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;

    /// A goal tool whose authority is a register held in the test: it
    /// arbitrates as the accounting thread does and keeps what it takes.
    fn tool(registered: Vec<GoalEntry>) -> GoalTool {
        let mut held = registered;
        let booking = GoalBooking::new(move |entry| match crate::arbitrate(&held, entry) {
            None => {
                held.push(entry.clone());
                Ok(())
            }
            Some(level) => Err(conflict_refusal(entry, &level)),
        });
        let desk = Arc::new(Mutex::new(GoalDesk::new(
            RunId::CITY,
            "potter@lab.1".to_owned(),
            booking,
        )));
        GoalTool::new(Address::parse("lab/room1").unwrap(), desk).unwrap()
    }

    fn call(args: Value) -> ToolCall {
        ToolCall {
            id: "tu_1".to_owned(),
            name: ToolName::parse("goal").unwrap(),
            args: Payload::new(args.as_object().unwrap().clone()).unwrap(),
        }
    }

    fn held(id: &str, path: &str, standing: bool) -> GoalEntry {
        GoalEntry {
            id: GoalId::new(id).unwrap(),
            owner: "mason@lab.2".to_owned(),
            resources: vec![GoalResource::Path(Address::parse(path).unwrap())],
            statement: "keep the kiln".to_owned(),
            standing,
        }
    }

    #[test]
    fn a_clear_claim_registers_and_the_next_one_in_the_same_run_sees_it() {
        let tool = tool(Vec::new());
        tool.invoke(&call(serde_json::json!({
            "statement": "rewrite the notes",
            "paths": ["lab/room1/notes.md"],
        })))
        .unwrap();
        let second = tool.invoke(&call(serde_json::json!({
            "statement": "rewrite them again",
            "paths": ["lab/room1/notes.md"],
        })));
        assert_eq!(
            second.map_err(|refusal| *refusal.code()).err(),
            Some(AxCode::GoalConflict),
            "a run that could claim the same ground twice would not be claiming anything"
        );
    }

    #[test]
    fn a_claim_on_held_ground_is_refused_with_the_level_that_decides_it() {
        let tool = tool(vec![held("g-held", "lab/room1", true)]);
        let refusal = tool
            .invoke(&call(serde_json::json!({
                "statement": "repaint the room",
                "paths": ["lab/room1/wall.md"],
                "standing": false,
            })))
            .unwrap_err();
        assert_eq!(refusal.code(), &AxCode::GoalConflict);
        let recovery = refusal.recovery();
        assert!(
            recovery.contains("g-held") && recovery.contains("wait"),
            "one standing goal against one piece of work serialises, and the refusal says so: \
             {recovery}"
        );
    }

    #[test]
    fn a_reading_that_no_machine_can_do_goes_to_a_resident_not_to_the_person() {
        let tool = tool(vec![held("g-held", "lab/room1", true)]);
        let refusal = tool
            .invoke(&call(serde_json::json!({
                "statement": "also keep the kiln",
                "paths": ["lab/room1"],
                "standing": true,
            })))
            .unwrap_err();
        let recovery = refusal.recovery();
        assert!(
            recovery.contains("signal the resident"),
            "two standing goals is a reading, and reading is a model's work: {recovery}"
        );
    }

    #[test]
    fn a_goal_that_claims_nothing_is_refused() {
        let tool = tool(Vec::new());
        let refusal = tool
            .invoke(&call(serde_json::json!({ "statement": "be helpful" })))
            .unwrap_err();
        assert_eq!(refusal.code(), &AxCode::InvalidArgs);
    }
}
