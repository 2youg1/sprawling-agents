// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::event::record::{
    ModelReturned, Rendering, RunStarted, SteerReceived, ToolAnswer, ToolResult,
};
use kernel::model::content_from_message;
use kernel::{
    Address, AxCode, AxError, ChatMessage, ContentBlock, EventKind, EventRecord, RunId, Seq,
};

use crate::compaction::Exchange;
use crate::conversation::{Conversation, Opening};
use crate::replay::VerifiedLedger;

mod lineage;

pub use lineage::fork_draft;
use lineage::forked_from;

pub fn prefix(mother: &VerifiedLedger, at_seq: Seq) -> Result<Vec<Vec<u8>>, AxError> {
    let refuse = || {
        AxError::failure(
            AxCode::InvalidArgs,
            "fork",
            format!("at_seq {}", at_seq.value()),
        )
        .with_recovery(match mother.tail_seq() {
            Some(tail) => format!("the mother sequence ends at seq {}", tail.value()),
            None => "the mother sequence is empty".to_string(),
        })
    };
    let tail = mother.tail_seq().ok_or_else(refuse)?;
    if at_seq > tail {
        return Err(refuse());
    }
    let index = usize::try_from(at_seq.value()).map_err(|_| refuse())?;
    let end = index.checked_add(1).ok_or_else(refuse)?;
    mother
        .raw_lines()
        .get(..end)
        .map(<[Vec<u8>]>::to_vec)
        .ok_or_else(refuse)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Inherited {
    pub messages: Vec<ChatMessage>,

    pub at: Seq,

    pub delivered: std::collections::BTreeSet<Address>,

    pub rule_warnings: Vec<String>,
}

fn no_start(owner: RunId) -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "fork",
        format!("{owner} has no run_started in this ledger"),
    )
    .with_recovery("name a run this history holds, or start a session instead")
}

fn fold_run<'a>(
    mut conversation: Conversation,
    records: impl Iterator<Item = &'a EventRecord>,
) -> Result<Inherited, AxError> {
    let mut at = Seq::FIRST;
    let mut delivered = std::collections::BTreeSet::new();
    let mut rule_warnings = Vec::new();
    let mut open: Option<Wave> = None;
    for record in records {
        match record.kind() {
            EventKind::RunStarted => {
                let started = record.data().read::<RunStarted>()?;
                conversation.push_task_lines(
                    (&started.task, &started.goal),
                    rebuilt(&started),
                    &handed_down_by(&started),
                    &started.pictures,
                );
                at = record.seq();
            }
            EventKind::ModelReturned => {
                let committed = commit(&mut conversation, open.take())?;
                delivered.extend(committed.rooms);
                rule_warnings.extend(committed.warnings);
                let returned = record.data().read::<ModelReturned>()?;
                let assistant = content_from_message(&returned.message)?;
                let mut exchange = Exchange::new();
                exchange.push_assistant(assistant);
                open = Some(Wave {
                    exchange,
                    expect: usize::try_from(returned.calls).unwrap_or(usize::MAX),
                    rules: std::collections::BTreeMap::new(),
                    warnings: Vec::new(),
                });
                if returned.calls == 0 {
                    let committed = commit(&mut conversation, open.take())?;
                    delivered.extend(committed.rooms);
                    rule_warnings.extend(committed.warnings);
                    at = record.seq();
                }
            }
            EventKind::ToolResult => {
                let result = record.data().read::<ToolResult>()?;
                if let Some(wave) = open.as_mut() {
                    wave.exchange.push_result(result_block(&result)?);
                    wave.remember_rules(&result);
                    if wave.exchange.results().len() >= wave.expect {
                        let committed = commit(&mut conversation, open.take())?;
                        delivered.extend(committed.rooms);
                        rule_warnings.extend(committed.warnings);
                        at = record.seq();
                    }
                }
            }

            EventKind::PromptShapeCompared => conversation.mark_sent(),
            EventKind::SteerReceived => {
                let steer = record.data().read::<SteerReceived>()?;
                conversation.push_steer(
                    &crate::conversation::Speaker::from_recorded(&steer.source)?,
                    &steer.text,
                    &steer.pictures,
                );
            }

            EventKind::CityInitialized
            | EventKind::BuildingCreated
            | EventKind::BuildingConfigured
            | EventKind::BuildingRemoved
            | EventKind::SessionOpened
            | EventKind::RunForked
            | EventKind::PromptAssembled
            | EventKind::ModelCalled
            | EventKind::ToolCalled
            | EventKind::ResultOffloaded
            | EventKind::GateChecked
            | EventKind::GateDenied
            | EventKind::CheckpointCommitted
            | EventKind::HandoffWritten
            | EventKind::CancelReceived
            | EventKind::WatchdogFired
            | EventKind::BudgetLimit
            | EventKind::RunFrozen
            | EventKind::LogTruncated
            | EventKind::SignalEnqueued
            | EventKind::SignalLanded
            | EventKind::SignalConsumed
            | EventKind::DraftHeld
            | EventKind::DraftResolved
            | EventKind::GoalRegistered
            | EventKind::GoalConflict
            | EventKind::ArbitrationVerdict
            | EventKind::RepairStarted
            | EventKind::RepairReused
            | EventKind::WorktreeOpened
            | EventKind::PrOpened
            | EventKind::PrMerged
            | EventKind::PrRejected
            | EventKind::RoadmapClaimed
            | EventKind::RoadmapFinished
            | EventKind::RoadmapReleased
            | EventKind::RoadmapSplit
            | EventKind::RoadmapBlocked
            | EventKind::PursuitChanged
            | EventKind::ApprovalRequested
            | EventKind::ApprovalResolved
            | EventKind::PolicyCreated
            | EventKind::PolicyRevoked
            | EventKind::TaintPromoted
            | EventKind::CrossBuildingTransfer
            | EventKind::CityHalted
            | EventKind::BackpressureShed
            | EventKind::DigestInvalidated
            | EventKind::EndpointAttached
            | EventKind::EndpointProbed
            | EventKind::EndpointLost
            | EventKind::ModelSelected
            | EventKind::ProviderDegraded
            | EventKind::LoginStarted
            | EventKind::EvalRun
            | EventKind::AssetArchived
            | EventKind::CredentialLent
            | EventKind::SecretCaptured
            | EventKind::SecretEgressBlocked
            | EventKind::FileDiscarded
            | EventKind::DiscardRestored
            | EventKind::AutonomyChanged
            | EventKind::WentBack
            | EventKind::FileRestored
            | EventKind::GovernedDocumentWritten
            | EventKind::SpineDocumentWritten
            | EventKind::RulesChanged
            | EventKind::ToolkitLinkOpened
            | EventKind::EmbeddingCalled
            | EventKind::RerankCalled
            | EventKind::AdviserAsked
            | EventKind::AdviserAnswered
            | EventKind::AdviserFellBack
            | EventKind::CacheRenewed
            | EventKind::PrefixHeld
            | EventKind::HarnessReported
            | EventKind::HarnessAnswered
            | EventKind::RemoteOpened
            | EventKind::RemoteClosed
            | EventKind::DevicePaired
            | EventKind::DeviceRevoked
            | EventKind::RemoteSessionStarted
            | EventKind::DocumentWritten
            | EventKind::ProposalOffered
            | EventKind::ProposalDecided
            | EventKind::ProposalWithdrawn
            | EventKind::RunPolicyChanged
            | EventKind::SessionNamed
            | EventKind::SkillAudited
            | EventKind::SkillShelved
            | EventKind::SignalWaitStarted
            | EventKind::SignalWaitEnded => {}
        }
    }

    open.take();
    Ok(Inherited {
        messages: conversation.messages().to_vec(),
        at,
        delivered,
        rule_warnings,
    })
}

fn handed_down_by(started: &RunStarted) -> String {
    started.dispatched_by.as_ref().map_or_else(
        || UNRECORDED_DISPATCHER.to_owned(),
        |who| who.handed_down_by(started.predecessor, started.parent),
    )
}

const UNRECORDED_DISPATCHER: &str = "an unrecorded dispatcher";

fn rebuilt(started: &RunStarted) -> Opening {
    match started.opening {
        Some(Opening::FromJob) => Opening::Inherited,
        Some(spoken @ (Opening::Inherited | Opening::WithPerson)) => spoken,
        None if started.job.is_some() => Opening::Inherited,
        None => Opening::WithPerson,
    }
}

struct Wave {
    exchange: Exchange,
    expect: usize,
    rules: std::collections::BTreeMap<String, std::collections::BTreeSet<Address>>,
    warnings: Vec<String>,
}

impl Wave {
    fn remember_rules(&mut self, result: &ToolResult) {
        if result.name.as_str() != crate::tools::ReadTool::NAME {
            return;
        }
        let ToolAnswer::Answered { result: payload } = &result.answer else {
            return;
        };
        let Some(rules) = payload
            .as_map()
            .get("rules")
            .and_then(serde_json::Value::as_array)
        else {
            return;
        };
        let mut rooms = std::collections::BTreeSet::new();
        for (index, rule) in rules.iter().enumerate() {
            let room = rule.get("room").and_then(serde_json::Value::as_str);
            let text = rule.get("text").and_then(serde_json::Value::as_str);
            match (room, text) {
                (Some(room), Some(_)) => match Address::parse(room) {
                    Ok(address) => {
                        rooms.insert(address);
                    }
                    Err(err) => self.warnings.push(format!(
                        "read result {} rule {} has room {room:?}: {}",
                        result.tool_use_id,
                        index.saturating_add(1),
                        err.subject()
                    )),
                },
                _ => self.warnings.push(format!(
                    "read result {} rule {} has no room and text strings",
                    result.tool_use_id,
                    index.saturating_add(1)
                )),
            }
        }
        self.rules.insert(result.tool_use_id.clone(), rooms);
    }
}

struct Committed {
    rooms: std::collections::BTreeSet<Address>,
    warnings: Vec<String>,
}

fn commit(conversation: &mut Conversation, wave: Option<Wave>) -> Result<Committed, AxError> {
    let Some(mut wave) = wave else {
        return Ok(Committed {
            rooms: std::collections::BTreeSet::new(),
            warnings: Vec::new(),
        });
    };
    let shortened = wave.exchange.compact()?;
    conversation.push_assistant(wave.exchange.assistant().to_vec());
    conversation.push_tool_results(wave.exchange.results().to_vec());
    let rooms = wave
        .rules
        .into_iter()
        .filter(|(id, _)| !shortened.contains(id))
        .flat_map(|(_, rooms)| rooms)
        .collect();
    Ok(Committed {
        rooms,
        warnings: wave.warnings,
    })
}

fn result_block(result: &ToolResult) -> Result<ContentBlock, AxError> {
    Ok(ContentBlock::ToolResult {
        tool_use_id: result.tool_use_id.clone(),
        content: crate::rendering::shown(
            Rendering::of(result.rendered),
            &result.name,
            &result.answer,
        )?,
        is_error: match result.answer {
            ToolAnswer::Answered { .. } => false,
            ToolAnswer::Failed { .. } => true,
        },
        attachments: result.attachments.clone(),
    })
}

mod indexed;
pub use indexed::inherited_indexed;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod compaction_tests;

#[cfg(test)]
mod request_tests;
