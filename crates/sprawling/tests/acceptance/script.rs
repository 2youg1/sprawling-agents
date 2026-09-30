// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The scripted model: the first run the worker builds a model for
//! follows the script, and every later one says one line and stops.
//!
//! Later runs are the ones the script's own calls start - a delegate, a
//! node handed down, a resident woken by a signal, the successor - and
//! a line of text ends each of them on its first turn, so the order of
//! the script belongs to the first run alone.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use kernel::{AxError, ContentBlock, Model, ModelRequest, ModelReturn, ToolCall, ToolName};

/// One call the script makes: the tool, and the arguments it is given.
pub(crate) struct Step {
    pub(crate) tool: &'static str,
    pub(crate) args: serde_json::Value,
}

/// The names of the tools the first request offered, in its order.
pub(crate) type Offered = Arc<Mutex<Vec<String>>>;

/// What each tool result said to the model, by the id of its call.
pub(crate) type Heard = Arc<Mutex<BTreeMap<String, String>>>;

/// Hands the script to the first model it is asked for and a quiet
/// model to every later one.
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

/// Makes one call per turn, so each wave holds one tool and the order
/// of the history is the order of the script; says it is done once the
/// script runs out.
struct Lead {
    steps: Vec<Step>,
    next: usize,
    offered: Offered,
    heard: Heard,
}

impl Model for Lead {
    fn call(&mut self, req: &ModelRequest) -> Result<ModelReturn, AxError> {
        // A request with no tools is a question asked after the run, such
        // as the reading taken before a successor starts; it is not a
        // turn of the script.
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
        self.next += 1;
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

/// The id the script gives its `n`th call, counted from one.
pub(crate) fn call_id(n: usize) -> String {
    format!("call-{n}")
}

/// Ends every run on its first turn.
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
