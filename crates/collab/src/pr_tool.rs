// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The pull request tool: the face `collab::pr` shows a model.
//!
//! The losing line of the whole design sits behind this one interface —
//! the resident who wrote the work does not decide whether it is good.
//! Three things enforce it, and none of them is a rule someone has to
//! remember: `Pr<Open>` has no `merged`, `Artifact` has no public
//! constructor, and this tool refuses a verification whose caller is the
//! implementer.
//!
//! Verifying and merging are one action rather than two. A `Verified`
//! request nobody merged would be a third state for a person to chase,
//! and the merge is not a second decision: it is what verification
//! means. A refusal is the other outcome of the same call.

use std::sync::{Arc, Mutex};

use kernel::{
    Address, AxCode, AxError, B3Hash, CostTier, Effect, Locator, Payload, RenderIntent, Temporal,
    Tool, ToolCall, ToolMeta, ToolName, ToolOutcome,
};
use serde_json::{Map, Value};

use crate::fanin::Claim;
use crate::pr::Pr;
use crate::workshop::NodeId;

mod request;

pub use request::OpenRequest;

/// What the run did to the city's requests. Exhaustive for the same
/// reason as the other desks: every variant is a line the worker has to
/// write, so a new one must be a compile error where the writing
/// happens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrEffect {
    /// Commit this run's tree onto its branch and record the request.
    Opened { branch: String },
    /// Merge the branch into the city trunk and record who checked it.
    Merged { request: OpenRequest, by: String },
    /// Record a refusal. The branch stays where it is; the work is not
    /// lost, it is not accepted.
    Rejected {
        request: OpenRequest,
        by: String,
        why: String,
    },
}

/// The run's side of the request register.
#[derive(Debug)]
pub struct PrDesk {
    who: String,
    room: Address,
    /// The tree this run works in, if it has one. A run without a tree
    /// has nothing to offer, and says so rather than offering the city's
    /// own files.
    branch: Option<String>,
    node: Option<NodeId>,
    open: Vec<OpenRequest>,
    effects: Vec<PrEffect>,
}

impl PrDesk {
    #[must_use]
    pub fn new(
        who: String,
        room: Address,
        branch: Option<String>,
        node: Option<NodeId>,
        open: Vec<OpenRequest>,
    ) -> PrDesk {
        PrDesk {
            who,
            room,
            branch,
            node,
            open,
            effects: Vec::new(),
        }
    }

    /// What the worker has to carry out, drained so it cannot run twice.
    pub fn take_effects(&mut self) -> Vec<PrEffect> {
        std::mem::take(&mut self.effects)
    }

    fn open(&mut self) -> Result<Payload, AxError> {
        let (Some(branch), Some(node)) = (self.branch.clone(), self.node.clone()) else {
            return Err(AxError::failure(
                AxCode::ToolUnavailable,
                "open a pull request",
                "this run has no tree of its own",
            )
            .with_recovery(
                "work in a building whose rules ask for review; a run that writes straight into \
                 the city has nothing to offer for merging",
            ));
        };
        if self.open.iter().any(|request| request.branch == branch) {
            return Err(
                AxError::failure(AxCode::InvalidArgs, "open a pull request", branch).with_recovery(
                    "this branch already has a request waiting; ask someone to check it",
                ),
            );
        }
        // Held as a request only after the worker commits the tree: the
        // commit is what the record names, and naming a commit that does
        // not exist yet would be a record about nothing.
        self.effects.push(PrEffect::Opened {
            branch: branch.clone(),
        });
        let mut result = Map::new();
        result.insert("node".to_owned(), Value::String(node.as_str().to_owned()));
        result.insert("branch".to_owned(), Value::String(branch));
        result.insert(
            "waiting_for".to_owned(),
            Value::String("another resident to check it".to_owned()),
        );
        Payload::new(result)
    }

    fn list(&self) -> Result<Payload, AxError> {
        let mut rows = Vec::with_capacity(self.open.len());
        for request in &self.open {
            let mut row = Map::new();
            row.insert(
                "node".to_owned(),
                Value::String(request.node.as_str().to_owned()),
            );
            row.insert("branch".to_owned(), Value::String(request.branch.clone()));
            row.insert(
                "implementer".to_owned(),
                Value::String(request.implementer.clone()),
            );
            // Stated rather than filtered out: a resident who cannot see
            // its own request would think it had vanished.
            row.insert(
                "yours".to_owned(),
                Value::Bool(request.implementer == self.who),
            );
            rows.push(Value::Object(row));
        }
        let mut result = Map::new();
        result.insert("requests".to_owned(), Value::Array(rows));
        Payload::new(result)
    }

    fn check(&mut self, args: &Map<String, Value>) -> Result<Payload, AxError> {
        let branch = text(args, "branch", "check a pull request")?;
        let Some(request) = self
            .open
            .iter()
            .find(|request| request.branch == branch)
            .cloned()
        else {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "check a pull request",
                branch.to_owned(),
            )
            .with_recovery("list the requests waiting, then name one of those branches"));
        };
        if request.implementer == self.who {
            return Err(AxError::failure(
                AxCode::GateDenied,
                "check a pull request",
                branch.to_owned(),
            )
            .with_recovery("you wrote this; ask another resident to run the done check on it"));
        }
        let passed = args.get("passed").and_then(Value::as_bool).ok_or_else(|| {
            AxError::failure(
                AxCode::InvalidArgs,
                "check a pull request",
                "missing boolean argument `passed`",
            )
            .with_recovery("say whether the node's own done check passed, as true or false")
        })?;
        if !passed {
            let why = args
                .get("why")
                .and_then(Value::as_str)
                .unwrap_or("the done check did not pass")
                .to_owned();
            let mut result = Map::new();
            result.insert("branch".to_owned(), Value::String(branch.to_owned()));
            result.insert("merged".to_owned(), Value::Bool(false));
            result.insert("why".to_owned(), Value::String(why.clone()));
            self.effects.push(PrEffect::Rejected {
                request,
                by: self.who.clone(),
                why,
            });
            return Payload::new(result);
        }
        // The typestate is walked here, not trusted: `verified` is the
        // only door from a claim to an artifact, and it refuses a
        // verifier who is the producer. Walking it means the refusal
        // cannot be skipped by a caller who forgot.
        let digest = B3Hash::digest(request.commit.as_bytes());
        let claim = Claim::new(
            request.node.clone(),
            Locator::parse(&format!("file:{}@{}", self.room.as_str(), request.commit))?,
            digest,
            request.implementer.clone(),
        );
        let artifact = claim.verified(true, &self.who)?;
        let pending = Pr::open(
            request.node.clone(),
            request.implementer.clone(),
            request.branch.clone(),
        )?;
        let verified = pending.verified(&artifact)?;
        let mut result = Map::new();
        result.insert("branch".to_owned(), Value::String(branch.to_owned()));
        result.insert("merged".to_owned(), Value::Bool(true));
        result.insert(
            "verified_by".to_owned(),
            Value::String(verified.verified_by().to_owned()),
        );
        self.open.retain(|held| held.branch != request.branch);
        self.effects.push(PrEffect::Merged {
            request,
            by: self.who.clone(),
        });
        Payload::new(result)
    }
}

pub struct PrTool {
    meta: ToolMeta,
    desk: Arc<Mutex<PrDesk>>,
}

impl PrTool {
    /// # Errors
    /// Propagates a malformed tool name or parameter schema.
    pub fn new(room: Address, desk: Arc<Mutex<PrDesk>>) -> Result<PrTool, AxError> {
        let mut properties = Map::new();
        for (field, kind, description) in [
            (
                "action",
                "string",
                "`open` to offer your work, `list` to see what is waiting, `check` to run \
                 someone else's done check",
            ),
            ("branch", "string", "check only: the branch you looked at"),
            (
                "passed",
                "boolean",
                "check only: whether that node's own done check passed",
            ),
            ("why", "string", "check only: why it did not pass"),
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
            Value::Array(vec![Value::String("action".to_owned())]),
        );
        Ok(PrTool {
            meta: ToolMeta {
                name: ToolName::parse("pr")?,
                disclosure:
                    "Offer your work for review, or check someone else's; nothing you wrote \
                     reaches the building until another resident has checked it."
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

impl Tool for PrTool {
    fn meta(&self) -> &ToolMeta {
        &self.meta
    }

    fn invoke(&mut self, call: &ToolCall) -> Result<ToolOutcome, AxError> {
        if call.name != self.meta.name {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "work a pull request",
                format!("call routed to the wrong tool: {}", call.name.as_str()),
            ));
        }
        let args = call.args.as_map();
        let mut desk = self.desk.lock().map_err(|_| {
            AxError::failure(
                AxCode::StorageFatal,
                "reach the request register",
                "the register is already in use",
            )
            .with_recovery("restart this city")
        })?;
        let result = match text(args, "action", "read a pull request action")? {
            "open" => desk.open()?,
            "list" => desk.list()?,
            "check" => desk.check(args)?,
            other => {
                return Err(AxError::failure(
                    AxCode::InvalidArgs,
                    "read a pull request action",
                    other.to_owned(),
                )
                .with_recovery("use `open`, `list` or `check`"));
            }
        };
        Ok(ToolOutcome { result })
    }
}

fn text<'a>(args: &'a Map<String, Value>, key: &str, action: &str) -> Result<&'a str, AxError> {
    args.get(key).and_then(Value::as_str).ok_or_else(|| {
        AxError::failure(
            AxCode::InvalidArgs,
            action.to_owned(),
            format!("missing string argument `{key}`"),
        )
        .with_recovery(format!("pass `{key}` as a string"))
    })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
