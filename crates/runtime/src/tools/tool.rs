// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The face of `tool` (`crates/runtime/Spec.lean` §8-60, §8-61): what an
//! admitted capability is, read from the run's catalog, and the one way
//! to run a tool that is not in the request's tool list. Its answers are
//! ordinary tool results appended at the end of the conversation, so the
//! request's tool list and its prompt cache do not move (runtime D21).
//!
//! A `run` is resolved at the tool face's entrance into the call it
//! stands for, which then passes its own tool's doors; this tool sees a
//! `run` only when that resolution refused, and answers the catalog's
//! refusal, so the refusal has one author.

use std::sync::{Arc, Mutex};

use kernel::{
    AxCode, AxError, CostTier, Effect, Payload, RenderIntent, Temporal, Tool, ToolCall, ToolMeta,
    ToolName, ToolOutcome,
};
use serde_json::Value;

use crate::catalog::Catalog;

const ACTION: &str = "tool";

/// The tool itself; it holds the catalog rather than a copy, as `read`
/// does, so what it answers is what the run was admitted.
pub struct ToolTool {
    catalog: Arc<Mutex<Catalog>>,
    meta: ToolMeta,
}

/// What one `tool` call asks for.
enum Op<'a> {
    List { namespace: Option<&'a str> },
    Find { query: &'a str },
    Detail { name: &'a str },
    Run,
}

impl ToolTool {
    /// The name this tool answers to, read by its registration, by
    /// `mode::core_tools` and by the catalog's resolution.
    pub const NAME: &'static str = "tool";

    /// The `op` that runs another tool: the only one the tool face
    /// resolves into the call it stands for.
    pub const RUN: &'static str = "run";

    /// # Errors
    /// Propagates a malformed parameter schema, which is a build-time
    /// defect rather than a runtime one.
    pub fn new(catalog: Arc<Mutex<Catalog>>) -> Result<ToolTool, AxError> {
        Ok(ToolTool {
            catalog,
            meta: ToolMeta {
                name: ToolName::parse(Self::NAME)?,
                disclosure: "Reach what this building admits beyond your tool list. `list` names \
                             the namespaces, or the members of one; `find` gives the closest \
                             tools to a few words; `detail` gives a tool's whole description and \
                             input schema; `run` runs it with its arguments, which are refused \
                             with the schema when they do not fit."
                    .to_owned(),
                params: Payload::of(&serde_json::json!({
                    "type": "object",
                    "properties": {
                        "op": { "type": "string", "enum": ["list", "find", "detail", "run"] },
                        "namespace": { "type": "string", "description": "list: the namespace \
                            to open; leave it out to list the namespaces" },
                        "query": { "type": "string", "description": "find: a few words of \
                            what you need" },
                        "name": { "type": "string", "description": "detail, run: a tool's \
                            whole name, as list or find spells it" },
                        "args": { "type": "object", "description": "run: the tool's own \
                            arguments, as its input schema asks" },
                    },
                    "required": ["op"],
                }))?,
                effect: Effect::Read,
                cost_tier: CostTier::Free,
                timeout: None,
                render: RenderIntent::Generic,
                temporal: Temporal::Timeless,
            },
        })
    }
}

impl Tool for ToolTool {
    fn meta(&self) -> &ToolMeta {
        &self.meta
    }

    fn invoke(&self, call: &ToolCall) -> Result<ToolOutcome, AxError> {
        if call.name != self.meta.name {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                ACTION,
                format!("call routed to the wrong tool: {}", call.name.as_str()),
            )
            .with_recovery(format!(
                "call `{}`, the name this tool answers to",
                Self::NAME
            )));
        }
        let catalog = self.catalog.lock().map_err(|_| poisoned())?;
        let text = match op(call)? {
            Op::List { namespace } => catalog.list(namespace),
            Op::Find { query } => catalog.find(query),
            Op::Detail { name } => catalog.detail(name)?,
            Op::Run => {
                let resolved = catalog.resolve_run(call)?;
                // Resolution succeeds here only when the tool face that
                // drove this call skipped it; running the tool from here
                // would pass none of its doors.
                return Err(AxError::failure(
                    AxCode::ToolUnavailable,
                    "run a tool through `tool`",
                    format!("`{}` was not resolved before it ran", resolved.name),
                )
                .with_recovery(
                    "report this against the tool face driving this run: it must resolve a \
                     `tool` `run` before admitting it",
                ));
            }
        };
        Ok(ToolOutcome {
            result: Payload::of(&serde_json::json!({ "text": text }))?,
            attachments: Vec::new(),
        })
    }
}

/// The op one call asks for, with the argument that op reads.
fn op(call: &ToolCall) -> Result<Op<'_>, AxError> {
    let given = call.args.as_map();
    let text = |key: &str| given.get(key).and_then(Value::as_str);
    let missing = |key: &str, op: &str| {
        AxError::failure(
            AxCode::InvalidArgs,
            ACTION,
            format!("`{op}` without a string argument `{key}`"),
        )
        .with_recovery(format!("pass `{key}` with `op: \"{op}\"`"))
    };
    match text("op") {
        Some("list") => Ok(Op::List {
            namespace: text("namespace"),
        }),
        Some("find") => text("query")
            .map(|query| Op::Find { query })
            .ok_or_else(|| missing("query", "find")),
        Some("detail") => text("name")
            .map(|name| Op::Detail { name })
            .ok_or_else(|| missing("name", "detail")),
        Some(ToolTool::RUN) => Ok(Op::Run),
        Some(other) => Err(AxError::failure(
            AxCode::InvalidArgs,
            ACTION,
            format!("no op called `{other}`"),
        )
        .with_recovery("pass `op` as one of `list`, `find`, `detail` or `run`")),
        None => Err(
            AxError::failure(AxCode::InvalidArgs, ACTION, "missing string argument `op`")
                .with_recovery("pass `op` as one of `list`, `find`, `detail` or `run`"),
        ),
    }
}

/// The catalog's lock was left by a thread that died; the same answer
/// `read` gives (`crates/runtime/spec/Tools/Read.lean`).
fn poisoned() -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        ACTION,
        "the catalog was left locked by a thread that died",
    )
    .with_recovery(
        "end this run and resume it: the catalog cannot be trusted again inside a process \
         where a thread died holding it",
    )
}
