// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use kernel::{AxError, ContentBlock, Model, ModelRequest, ModelReturn, ToolCall, ToolName};

pub(crate) struct Step {
    pub(crate) tool: &'static str,
    pub(crate) args: serde_json::Value,
}

pub(crate) type Offered = Arc<Mutex<Vec<String>>>;

pub(crate) type Heard = Arc<Mutex<BTreeMap<String, String>>>;

pub(crate) struct Scripted {
    lead: Mutex<Option<Lead>>,
}

pub(crate) fn scripted(steps: Vec<Step>) -> (Scripted, Offered, Heard) {
    let offered = Offered::default();
    let heard = Heard::default();
    let lead = Lead {
        steps,
        next: 0,
        offered: Arc::clone(&offered),
        heard: Arc::clone(&heard),
    };
    (
        Scripted {
            lead: Mutex::new(Some(lead)),
        },
        offered,
        heard,
    )
}

impl accounting::ModelFactory for Scripted {
    fn build(
        &self,
        _chosen: &gateway::Chosen<'_>,
        _redemption: gateway::Redemption,
    ) -> Result<Box<dyn Model + Send>, AxError> {
        Ok(match self.lead.lock().unwrap().take() {
            Some(lead) => Box::new(lead),
            None => Box::new(Quiet),
        })
    }
}

struct Lead {
    steps: Vec<Step>,
    next: usize,
    offered: Offered,
    heard: Heard,
}

impl Model for Lead {
    fn call(&mut self, req: &ModelRequest) -> Result<ModelReturn, AxError> {

        if req.chat.tools.is_empty() {
            return says("done");
        }
        let mut offered = self.offered.lock().unwrap();
        if offered.is_empty() {
            offered.extend(
                req.chat
                    .tools
                    .iter()
                    .map(|tool| tool.name.as_str().to_owned()),
            );
            offered.extend(dormant_tools(req));
        }
        drop(offered);
        self.heard.lock().unwrap().extend(
            req.chat
                .messages
                .iter()
                .flat_map(|message| &message.content)
                .filter_map(|block| match block {
                    ContentBlock::ToolResult {
                        tool_use_id,
                        content,
                        ..
                    } => Some((tool_use_id.clone(), content.clone())),
                    ContentBlock::Text { .. }
                    | ContentBlock::Thinking { .. }
                    | ContentBlock::RedactedThinking { .. }
                    | ContentBlock::ToolUse { .. }
                    | ContentBlock::Image(_) => None,
                }),
        );
        let Some(step) = self.steps.get(self.next) else {
            return says("every tool has answered");
        };
        self.next = self.next.saturating_add(1);
        let id = call_id(self.next);
        let name = ToolName::parse(step.tool)?;
        let args = kernel::Payload::new(step.args.as_object().cloned().unwrap())?;
        Ok(ModelReturn::bare(
            kernel::model::message_payload(&[ContentBlock::ToolUse {
                id: id.clone(),
                name: name.clone(),
                input: args.clone(),
            }])?,
            vec![ToolCall { id, name, args }],
        ))
    }
}

pub(crate) fn call_id(n: usize) -> String {
    format!("call-{n}")
}

struct Quiet;

impl Model for Quiet {
    fn call(&mut self, _req: &ModelRequest) -> Result<ModelReturn, AxError> {
        says("nothing to add")
    }
}

fn says(text: &str) -> Result<ModelReturn, AxError> {
    Ok(ModelReturn::bare(
        kernel::model::message_payload(&[ContentBlock::Text {
            text: text.to_owned(),
        }])?,
        Vec::new(),
    ))
}

pub(crate) fn dormant_tools(req: &ModelRequest) -> Vec<String> {
    req.chat
        .system
        .iter()
        .filter_map(|block| block.text.split_once("Dormant,"))
        .flat_map(|(_, index)| index.lines().skip(1))
        .filter_map(|line| line.strip_prefix("- "))
        .map(|entry| entry.split_once(':').map_or(entry, |(name, _)| name))
        .filter(|name| !name.starts_with("skill "))
        .map(str::to_owned)
        .collect()
}
