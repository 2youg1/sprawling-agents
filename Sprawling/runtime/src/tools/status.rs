// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{
    Address, AxCode, AxError, ByteLen, CostTier, DelegateKind, Effect, Payload, RenderIntent,
    Temporal, TimeMs, Tokens, Tool, ToolCall, ToolDef, ToolMeta, ToolName, ToolOutcome,
};
use serde_json::{Map, Value};

use crate::backlog::{Backlog, Standing};
use crate::clock::{ClockReading, iso};
use crate::reminder::ContextReading;
use kernel::RunPolicy;
use kernel::event::record::PromptTools;
use std::path::{Path, PathBuf};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildStatus {
    pub room: Address,
    pub kind: DelegateKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusSnapshot {
    pub who: String,
    pub addr: Address,

    pub policy: RunPolicy,
    pub ctx_limit: Tokens,
    pub trust: String,
    pub write_domain: String,
    pub locks: Vec<String>,

    pub worktree: PathBuf,
    pub signals_pending: u32,
    pub provider_mode: ProviderMode,

    pub neighbours: u32,
}

pub struct StatusTool {
    snapshot: StatusSnapshot,
    children: Box<dyn Fn() -> Vec<ChildStatus> + Send + Sync>,

    backlog: Option<Backlog>,

    context: ContextReading,

    clock: ClockReading,

    catalog: Box<dyn Fn() -> Option<PromptTools> + Send + Sync>,
    meta: ToolMeta,
}

impl StatusTool {

    pub const NAME: &'static str = "status";

    pub fn new(snapshot: StatusSnapshot) -> Result<StatusTool, AxError> {
        StatusTool::watching(snapshot, Box::new(Vec::new))
    }

    pub fn definition() -> Result<ToolDef, AxError> {
        let mut params = Map::new();
        params.insert("type".to_owned(), Value::String("object".to_owned()));
        params.insert("properties".to_owned(), Value::Object(Map::new()));
        Ok(ToolDef {
            name: ToolName::parse(Self::NAME)?,
            description: "Report your current situation in fourteen lines: who you are, where you \
                          are, your mode and run policy, context used against the window, your trust, your write \
                          domain and its locks, the worktree and its size, the signals waiting, your \
                          children, the clock, whether the provider is serving, your neighbours, your backlog, \
                          and your catalog counts and bytes. None of it is stated anywhere else."
                .to_owned(),
            input_schema: Payload::new(params)?,
        })
    }

    pub fn watching(
        snapshot: StatusSnapshot,
        children: Box<dyn Fn() -> Vec<ChildStatus> + Send + Sync>,
    ) -> Result<StatusTool, AxError> {
        let defined = Self::definition()?;
        Ok(StatusTool {
            snapshot,
            children,
            backlog: None,
            context: ContextReading::default(),
            clock: ClockReading::default(),
            catalog: Box::new(|| None),
            meta: ToolMeta {
                name: defined.name,
                disclosure: defined.description,
                params: defined.input_schema,
                effect: Effect::Read,
                cost_tier: CostTier::Free,
                timeout: None,
                render: RenderIntent::Generic,
                temporal: Temporal::Timestamped,
            },
        })
    }

    #[must_use]
    pub fn metering(mut self, context: ContextReading) -> StatusTool {
        self.context = context;
        self
    }

    #[must_use]
    pub fn clocked(mut self, clock: ClockReading) -> StatusTool {
        self.clock = clock;
        self
    }

    #[must_use]
    pub fn cataloging(
        mut self,
        read: Box<dyn Fn() -> Option<PromptTools> + Send + Sync>,
    ) -> StatusTool {
        self.catalog = read;
        self
    }

    #[must_use]
    pub fn reporting(mut self, backlog: Backlog) -> StatusTool {
        self.backlog = Some(backlog);
        self
    }

    fn standing(&self) -> Vec<Standing> {
        self.backlog
            .as_ref()
            .and_then(|table| table.standing(&self.snapshot.addr).ok())
            .unwrap_or_default()
    }
}

impl StatusSnapshot {

    fn render(&self, live: &Live<'_>) -> String {
        let Live {
            used,
            now,
            children,
            standing,
            disk,
            catalog,
        } = live;
        let locks = if self.locks.is_empty() {
            "none".to_owned()
        } else {
            self.locks.join(", ")
        };
        let children = render_children(children);
        let backlog = render_standing(standing);
        let now = now.map_or_else(|| "not stamped".to_owned(), iso);
        let disk = match disk {
            Ok(bytes) => format!("{} bytes", bytes.get()),
            Err(err) => format!("size unreadable: {err}"),
        };
        let mut lines = vec![
            format!("who: {}", self.who),
            format!("addr: {}", self.addr),
            format!(
                "mode: {} (write {}, admit {}, landing {})",
                self.policy.mode.as_str(),
                self.policy.write.as_str(),
                self.policy.admit.as_str(),
                self.policy.landing.as_str()
            ),
            format!("ctx: {}/{}", used.get(), self.ctx_limit.get()),
            format!("trust: {}", self.trust),
            format!("write_domain: {} (locks: {locks})", self.write_domain),
            format!("worktree: {} ({disk})", self.worktree.display()),
            format!("signals_pending: {}", self.signals_pending),
            format!("children: {children}"),
            format!("now: {now}"),
            format!("provider_mode: {}", self.provider_mode.as_str()),
            format!("neighbours: {}", self.neighbours),
            format!("backlog: {backlog}"),
        ];
        lines.push(catalog.map_or_else(
            || "catalog: not frozen".to_owned(),
            |table| {
                format!(
                    "catalog: {} always, {} dormant, {} skills, {} core bytes, {} index bytes, {} reading bytes",
                    table.always, table.dormant, table.skills, table.len, table.index, table.reading
                )
            },
        ));
        lines.join("\n")
    }
}

struct Live<'a> {
    used: Tokens,
    now: Option<TimeMs>,
    children: &'a [ChildStatus],
    standing: &'a [Standing],
    disk: std::io::Result<ByteLen>,
    catalog: Option<&'a PromptTools>,
}

fn tree_bytes(root: &Path) -> std::io::Result<ByteLen> {
    let mut total: u64 = 0;
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            if kind.is_dir() {
                if entry.file_name() != kernel::RESERVED_PREFIX {
                    pending.push(entry.path());
                }
            } else if kind.is_file() {
                total = total.saturating_add(entry.metadata()?.len());
            }
        }
    }
    Ok(ByteLen::new(total))
}

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

    fn invoke(&self, call: &ToolCall) -> Result<ToolOutcome, AxError> {
        if call.name != self.meta.name {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "read status",
                format!("call routed to the wrong tool: {}", call.name.as_str()),
            )
            .with_recovery(format!(
                "call `{}`, the name this tool answers to",
                self.meta.name.as_str()
            )));
        }
        let catalog = (self.catalog)();
        let mut result = Map::new();
        result.insert(
            "text".to_owned(),
            Value::String(self.snapshot.render(&Live {
                used: self.context.tokens(),
                now: self.clock.latest(),
                children: &(self.children)(),
                standing: &self.standing(),
                disk: tree_bytes(&self.snapshot.worktree),
                catalog: catalog.as_ref(),
            })),
        );
        Ok(ToolOutcome {
            result: Payload::new(result)?,
            attachments: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests;
