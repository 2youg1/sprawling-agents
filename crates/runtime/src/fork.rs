// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Fork: a new Run whose in-window history is a byte-identical prefix of
//! the mother sequence. Forking is not
//! resurrection: the mother's frozen state never changes, and the new
//! RunId arrives from the caller — this module is pure.
//!
//! Consumes [`VerifiedLedger`] only: replay and fork share one rebuilder,
//! so fork correctness and replay correctness are the same assertion.

use kernel::event::record::{ModelReturned, RunStarted, SteerReceived, ToolAnswer, ToolResult};
use kernel::model::content_from_message;
use kernel::{AxCode, AxError, ChatMessage, ContentBlock, EventKind, EventRecord, RunId, Seq};

use crate::compaction::Exchange;
use crate::conversation::{Conversation, Opening};
use crate::replay::{VerifiedLedger, VerifiedLine};

mod lineage;

pub use lineage::fork_draft;
use lineage::forked_from;

/// The fork prefix: raw lines `0..=at_seq`, byte-exact. Past the tail is
/// `E_INVALID_ARGS`, never a silent clamp.
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

/// What a new session inherits from the one it branched off: the
/// conversation the mother had exchanged by a safe point.
#[derive(Debug, Clone, PartialEq)]
pub struct Inherited {
    /// The messages the forked run's window opens with, in the order the
    /// mother exchanged them.
    pub messages: Vec<ChatMessage>,
    /// The line those messages were rebuilt through. Equal to the
    /// `at_seq` asked for when that line was a safe point, and the
    /// nearest earlier one when it was not: a wave of tool calls is
    /// several lines and only its end is a place a conversation can be
    /// cut, because a provider refuses an assistant message whose tool
    /// uses nothing answers.
    pub at: Seq,
}

/// Rebuilds the mother's conversation from the ledger, through `at_seq`
/// or through the nearest safe point before it.
///
/// **The ledger is the only source, and it is enough.** A run's window is
/// folded forward by the loop that owns it, so it does not survive the
/// process; every line it was folded from does. This walks the mother's
/// own records - the opening from `run_started`, the assistant messages
/// from `model_returned`, the tool results from `tool_result`, and what
/// the person typed mid-flight from `steer_received` - and folds them
/// through the same [`Conversation`] the live loop folds through, so the
/// sequence a fork starts from is the sequence the mother sent rather
/// than a second reading of the same records.
///
/// **Redaction is already applied.** The ledger holds what the city
/// wrote after scanning for credentials, so a branch inherits the text
/// the mother actually sent, which is the redacted one. That is the
/// honest reading: the bytes before redaction exist nowhere any more.
///
/// **The turn boundary's compaction is re-applied here, from the same
/// judgment.** The mother's window carries the compacted exchange of
/// each turn (`runtime::compaction::Exchange`), and this rebuild compacts
/// at the same boundary of the same turn, so a branch starts from the
/// bytes the mother sent rather than from the fuller bytes the ledger
/// kept.
///
/// **A reminder is not rebuilt, and cannot be.** The context gauge's
/// nudge is folded into the window without a record of its own, because
/// it is the city talking to the model about this run's budget. A branch
/// starts with a gauge of its own, so it starts without that nudge.
///
/// # Errors
/// Refuses an `at_seq` that names no line, and a line that belongs to a
/// run with no `run_started` in this ledger - the same two facts
/// [`prefix`] refuses, in the same words a person can act on.
pub fn inherited(mother: &VerifiedLedger, at_seq: Seq) -> Result<Inherited, AxError> {
    let index = usize::try_from(at_seq.value()).map_err(|_| outside(mother, at_seq))?;
    let mut lines = mother.lines().iter();
    let owner = lines
        .nth(index)
        .and_then(run_of)
        .ok_or_else(|| outside(mother, at_seq))?;
    let start = (0..=index).find(|at| {
        matches!(
            mother.lines().get(*at).and_then(known),
            Some(record) if record.run() == owner && record.kind() == EventKind::RunStarted
        )
    });
    let Some(start) = start else {
        return Err(AxError::failure(
            AxCode::InvalidArgs,
            "fork",
            format!("{owner} has no run_started in this ledger"),
        )
        .with_recovery("name a run this history holds, or start a session instead"));
    };

    let mut conversation = Conversation::new();
    // A mother that was herself a branch opened with her own mother's
    // conversation, which lives on the grandmother's lines, not hers.
    if let Some(origin) = forked_from(mother, owner, index)? {
        conversation.push_inherited(&inherited(mother, origin)?.messages);
    }
    let mut at = Seq::FIRST;
    let mut open: Option<Wave> = None;
    for line in mother
        .lines()
        .iter()
        .take(index.saturating_add(1))
        .skip(start)
    {
        let Some(record) = known(line) else {
            continue;
        };
        if record.run() != owner {
            continue;
        }
        match record.kind() {
            EventKind::RunStarted => {
                let started = record.data().read::<RunStarted>()?;
                conversation.push_task_lines(
                    &started.task,
                    &started.goal,
                    if started.job.is_some() {
                        Opening::Inherited
                    } else {
                        Opening::WithPerson
                    },
                );
                at = record.seq();
            }
            EventKind::ModelReturned => {
                // A turn that is already open means the ledger has a
                // reply before the calls of the one before it were
                // answered, which a run cannot produce. Reading it as
                // the start of a new turn is what keeps a damaged
                // history from being silently rewound.
                commit(&mut conversation, open.take())?;
                let returned = record.data().read::<ModelReturned>()?;
                let assistant = content_from_message(&returned.message)?;
                let mut exchange = Exchange::new();
                exchange.push_assistant(assistant);
                open = Some(Wave {
                    exchange,
                    expect: usize::try_from(returned.calls).unwrap_or(usize::MAX),
                });
                if returned.calls == 0 {
                    commit(&mut conversation, open.take())?;
                    at = record.seq();
                }
            }
            EventKind::ToolResult => {
                let result = record.data().read::<ToolResult>()?;
                if let Some(wave) = open.as_mut() {
                    wave.exchange.push_result(result_block(&result)?);
                    if wave.exchange.results().len() >= wave.expect {
                        commit(&mut conversation, open.take())?;
                        at = record.seq();
                    }
                }
            }
            EventKind::SteerReceived => {
                let steer = record.data().read::<SteerReceived>()?;
                conversation.push_steer(&steer.source, &steer.text);
            }
            // Everything else is not the conversation: the segment
            // hashes a call was assembled from, the accounting, the
            // checks, the city's own lines. Listed rather than defaulted
            // so a kind added to the vocabulary is a decision here - is
            // this something the mother said or answered? - instead of a
            // silence.
            EventKind::CityInitialized
            | EventKind::BuildingCreated
            | EventKind::BuildingConfigured
            | EventKind::SessionOpened
            | EventKind::RunForked
            | EventKind::PromptAssembled
            // The cache shape measures a request; it is not one of the
            // things the mother said.
            | EventKind::PromptShapeCompared
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
            | EventKind::GovernedDocumentWritten
            | EventKind::SpineDocumentWritten
            | EventKind::RulesChanged
            | EventKind::ToolkitLinkOpened
            | EventKind::EmbeddingCalled
            | EventKind::RerankCalled
            | EventKind::AdviserAsked
            | EventKind::AdviserAnswered
            | EventKind::AdviserFellBack => {}
        }
    }
    // A wave the cut landed inside is dropped whole: the assistant
    // message that asked for the calls and the answers it did get are
    // one exchange, and half of one is a shape no provider accepts.
    open.take();
    Ok(Inherited {
        messages: conversation.messages().to_vec(),
        at,
    })
}

/// One turn being folded: the assistant message and the results it is
/// waiting for, held together because neither is a conversation without
/// the other. The value is `Exchange`, the same one the turn layer
/// collects live, so the two folds cannot drift.
struct Wave {
    exchange: Exchange,
    expect: usize,
}

/// Folds one whole turn into the window, at the same boundary the live
/// turn compacts at: the wave is complete here or it is dropped whole,
/// so the compaction meets what the turn boundary met and answers the
/// same bytes.
fn commit(conversation: &mut Conversation, wave: Option<Wave>) -> Result<(), AxError> {
    let Some(mut wave) = wave else {
        return Ok(());
    };
    wave.exchange.compact()?;
    conversation.push_assistant(wave.exchange.assistant().to_vec());
    conversation.push_tool_results(wave.exchange.results().to_vec());
    Ok(())
}

/// One tool result as the model reads it back, rebuilt from the record
/// through the same printing the live wave used.
///
/// The pictures a tool produced are not rebuilt: the record carries the
/// payload a tool answered with, and the block's `attachments` are the
/// references that payload held, which the next request re-resolves from
/// the content store. A fork without them loses what the next call would
/// have fetched anyway.
fn result_block(result: &ToolResult) -> Result<ContentBlock, AxError> {
    let (payload, is_error) = match &result.answer {
        ToolAnswer::Answered { result } => (result, false),
        ToolAnswer::Failed { error } => (error, true),
    };
    Ok(ContentBlock::ToolResult {
        tool_use_id: result.tool_use_id.clone(),
        content: serde_json::to_string(payload).map_err(|err| {
            AxError::failure(
                AxCode::InvalidArgs,
                "rebuild a tool result for a fork",
                err.to_string(),
            )
            .with_recovery(
                "report this against runtime::fork: a payload refused JSON after the \
                 record that holds it was read",
            )
        })?,
        is_error,
        attachments: Vec::new(),
    })
}

/// One record's own kind, read from a verified line.
fn known(line: &VerifiedLine) -> Option<&EventRecord> {
    match line {
        VerifiedLine::Known { record, .. } => Some(record),
        VerifiedLine::IgnoredUnknown { .. } => None,
    }
}

/// The run one line belongs to, when it belongs to one this build reads.
fn run_of(line: &VerifiedLine) -> Option<RunId> {
    known(line).map(EventRecord::run)
}

/// The refusal for a line that is not there, in the words a person can
/// act on: what the sequence ends at.
fn outside(mother: &VerifiedLedger, at_seq: Seq) -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "fork",
        format!("at_seq {}", at_seq.value()),
    )
    .with_recovery(match mother.tail_seq() {
        Some(tail) => format!("the mother sequence ends at seq {}", tail.value()),
        None => "the mother sequence is empty".to_string(),
    })
}

#[cfg(test)]
mod tests;
