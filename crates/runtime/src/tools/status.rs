// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The status tool: the model's view of its own situation, in the
//! thirteen frozen fields, in that order.
//!
//! The order is not cosmetic. A model reading its status reads the top
//! first, so identity and mode come before the context reading, and that
//! comes before the long tail of locks and children. Freezing the order
//! means a Resident's habits transfer across versions instead of being
//! relearned each time the field list grows - which is why the last
//! field went on the end rather than beside the one it belongs with.
//!
//! **No ceiling is reported, because there is none** (card-11.7). What
//! stands where a spend ceiling used to is the context reading, which is
//! tokens a run has actually used against the window it was given.
//!
//! Nothing here samples. The executor sets the snapshot once per turn
//! and the tool reports it; a tool that read the clock itself would
//! make two calls in one turn disagree about "now".

use kernel::{
    Address, AxCode, AxError, ByteLen, CostTier, DelegateKind, Effect, Payload, RenderIntent,
    Temporal, Tokens, Tool, ToolCall, ToolMeta, ToolName, ToolOutcome,
};
use serde_json::{Map, Value};

use crate::backlog::{Backlog, Standing};
use crate::clock::ClockStamp;
use crate::mode::Mode;

/// How the gateway is currently able to serve. Degraded and LocalOnly
/// are situations the model should plan around, so they are reported
/// rather than hidden behind a retry.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderMode {
    Normal,
    Degraded,
    LocalOnly,
}

impl ProviderMode {
    pub fn as_str(self) -> &'static str {
        match self {
            ProviderMode::Normal => "normal",
            ProviderMode::Degraded => "degraded",
            ProviderMode::LocalOnly => "local_only",
        }
    }
}

/// One piece of work this run handed down.
///
/// Two fields, because two facts exist. A child starts after the run
/// that asked for it has frozen, so while that run is still reading its
/// own status the child has no id, no phase and no context reading - and
/// a field that can only ever hold zero says "zero" where the truth is
/// "not yet".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildStatus {
    pub room: Address,
    pub kind: DelegateKind,
}

/// The twelve frozen fields. The thirteenth, `backlog`, is read live
/// from the table rather than frozen here, for the reason `children` is
/// a closure: what is running changes while the run goes on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusSnapshot {
    pub who: String,
    pub addr: Address,
    pub mode: Mode,
    pub ctx_used: Tokens,
    pub ctx_limit: Tokens,
    pub trust: String,
    pub write_domain: String,
    pub locks: Vec<String>,
    pub worktree_path: String,
    pub worktree_disk: ByteLen,
    pub signals_pending: u32,
    pub now: Option<ClockStamp>,
    pub provider_mode: ProviderMode,
    /// How many residents this run can reach, itself excluded. A count
    /// rather than a list: the list grows with the building's population
    /// and this text does not, so the names live behind the `neighbours`
    /// tool and what stands here is whether asking is worth a call.
    ///
    /// People, not places. An empty room takes messages nobody reads, so
    /// counting one would make `neighbours: 1` mean the opposite of what
    /// it says.
    pub neighbours: u32,
}

pub struct StatusTool {
    snapshot: StatusSnapshot,
    children: Box<dyn Fn() -> Vec<ChildStatus> + Send>,
    /// The table this run's background commands stand in. `None` in a
    /// tool built without one, which reports `backlog: none` truthfully:
    /// nothing was started through a table that does not exist.
    backlog: Option<Backlog>,
    meta: ToolMeta,
}

impl StatusTool {
    /// A run that hands nothing down.
    ///
    /// # Errors
    /// Propagates a malformed parameter schema.
    pub fn new(snapshot: StatusSnapshot) -> Result<StatusTool, AxError> {
        StatusTool::watching(snapshot, Box::new(Vec::new))
    }

    /// A run whose delegate desk is asked every time the model calls
    /// `status`.
    ///
    /// A closure rather than a field, because delegation happens after
    /// this tool is built: a snapshot taken before the run started is
    /// empty for the whole run. A closure rather than a seam, because
    /// the desk lives in `collab` and this crate may not depend on it -
    /// the assembly layer owns both ends and hands one to the other.
    ///
    /// # Errors
    /// Propagates a malformed parameter schema.
    pub fn watching(
        snapshot: StatusSnapshot,
        children: Box<dyn Fn() -> Vec<ChildStatus> + Send>,
    ) -> Result<StatusTool, AxError> {
        let mut params = Map::new();
        params.insert("type".to_owned(), Value::String("object".to_owned()));
        params.insert("properties".to_owned(), Value::Object(Map::new()));
        Ok(StatusTool {
            snapshot,
            children,
            backlog: None,
            meta: ToolMeta {
                name: ToolName::parse("status")?,
                disclosure:
                    "Report your current situation: mode, context, domain, children, backlog."
                        .to_owned(),
                params: Payload::new(params)?,
                effect: Effect::Read,
                cost_tier: CostTier::Free,
                timeout: None,
                render: RenderIntent::Generic,
                temporal: Temporal::Timestamped,
            },
        })
    }

    /// The executor's once-per-turn update. Sampling inside the tool
    /// would let two calls in one turn disagree.
    pub fn set_snapshot(&mut self, snapshot: StatusSnapshot) {
        self.snapshot = snapshot;
    }

    /// The table whose members at this run's address the thirteenth
    /// line reports (runtime-SPEC 8-28-2).
    #[must_use]
    pub fn reporting(mut self, backlog: Backlog) -> StatusTool {
        self.backlog = Some(backlog);
        self
    }

    /// What stands in the backlog at this run's address. A table that
    /// cannot be reached reads as empty: the tool reports the run's
    /// situation, and a poisoned lock is a fact about the process that
    /// a model can do nothing with.
    fn standing(&self) -> Vec<Standing> {
        self.backlog
            .as_ref()
            .and_then(|table| table.standing(&self.snapshot.addr).ok())
            .unwrap_or_default()
    }
}

impl StatusSnapshot {
    /// The frozen order, one field per line.
    ///
    /// The result is text rather than a JSON object because a JSON
    /// object has no order a reader can rely on — `serde_json`'s map
    /// sorts its keys, so "the frozen order" would silently become
    /// alphabetical. Order is a property of what the model reads, so it
    /// is expressed where the model reads it.
    pub fn render(&self, children: &[ChildStatus], standing: &[Standing]) -> String {
        let locks = if self.locks.is_empty() {
            "none".to_owned()
        } else {
            self.locks.join(", ")
        };
        let children = render_children(children);
        let backlog = render_standing(standing);
        let now = match &self.now {
            Some(stamp) => stamp.render(),
            None => "not stamped".to_owned(),
        };
        [
            format!("who: {}", self.who),
            format!("addr: {}", self.addr),
            format!("mode: {}", self.mode.as_str()),
            format!("ctx: {}/{}", self.ctx_used.get(), self.ctx_limit.get()),
            format!("trust: {}", self.trust),
            format!("write_domain: {} (locks: {locks})", self.write_domain),
            format!(
                "worktree: {} ({} bytes)",
                self.worktree_path,
                self.worktree_disk.get()
            ),
            format!("signals_pending: {}", self.signals_pending),
            format!("children: {children}"),
            format!("now: {now}"),
            format!("provider_mode: {}", self.provider_mode.as_str()),
            format!("neighbours: {}", self.neighbours),
            format!("backlog: {backlog}"),
        ]
        .join("\n")
    }
}

/// What is still running at this run's address, or the word for none.
/// One line, for the reason `render_children` gives. No duration: this
/// table samples no clock, and a status that sampled one to say "for
/// forty seconds" would be a second sampling point.
fn render_standing(standing: &[Standing]) -> String {
    if standing.is_empty() {
        return "none".to_owned();
    }
    standing
        .iter()
        .map(|member| format!("{} {} ({})", member.id, member.what, member.kind.as_str()))
        .collect::<Vec<String>>()
        .join("; ")
}

/// Where each piece of handed-down work went, or the word for none.
/// One line, because `status` is read as lines and a list that wrapped
/// would break the field order the whole tool exists to keep.
fn render_children(children: &[ChildStatus]) -> String {
    if children.is_empty() {
        return "none".to_owned();
    }
    children
        .iter()
        .map(|child| format!("{} ({})", child.room, child.kind.as_str()))
        .collect::<Vec<String>>()
        .join("; ")
}

impl Tool for StatusTool {
    fn meta(&self) -> &ToolMeta {
        &self.meta
    }

    fn invoke(&mut self, call: &ToolCall) -> Result<ToolOutcome, AxError> {
        if call.name != self.meta.name {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "read status",
                format!("call routed to the wrong tool: {}", call.name.as_str()),
            ));
        }
        let mut result = Map::new();
        result.insert(
            "text".to_owned(),
            Value::String(self.snapshot.render(&(self.children)(), &self.standing())),
        );
        Ok(ToolOutcome {
            result: Payload::new(result)?,
            attachments: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests;
