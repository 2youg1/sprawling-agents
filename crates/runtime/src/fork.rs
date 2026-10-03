// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Fork: a new Run whose in-window history is a byte-identical prefix of
//! the mother sequence. Forking is not
//! resurrection: the mother's frozen state never changes, and the new
//! RunId arrives from the caller — this module is pure.
//! `crates/runtime/spec/Fork.lean` proves what a fork must hold (§8-2):
//! the prefix is the mother's first lines, and the cut never splits a wave.
//!
//! [`prefix`] consumes [`VerifiedLedger`], so replay and fork share one
//! rebuilder; a branch's conversation is rebuilt through the ledger
//! index by [`inherited_indexed`], because the ledger's only writer asks
//! for it and verified the ledger when it opened it.

use kernel::event::record::{ModelReturned, RunStarted, SteerReceived, ToolAnswer, ToolResult};
use kernel::model::content_from_message;
use kernel::{AxCode, AxError, ChatMessage, ContentBlock, EventKind, EventRecord, RunId, Seq};

use crate::compaction::Exchange;
use crate::conversation::{Conversation, Opening};
use crate::replay::VerifiedLedger;

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

/// The refusal for a run whose opening line this history does not hold.
fn no_start(owner: RunId) -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "fork",
        format!("{owner} has no run_started in this ledger"),
    )
    .with_recovery("name a run this history holds, or start a session instead")
}

/// Folds one run's own records, from its `run_started` through the cut,
/// into the conversation it sent. The one fold both doors share, so the
/// verified door and the indexed one cut at the same line.
fn fold_run<'a>(
    mut conversation: Conversation,
    records: impl Iterator<Item = &'a EventRecord>,
) -> Result<Inherited, AxError> {
    let mut at = Seq::FIRST;
    let mut open: Option<Wave> = None;
    for record in records {
        match record.kind() {
            EventKind::RunStarted => {
                let started = record.data().read::<RunStarted>()?;
                conversation.push_task_lines(&started.task, &started.goal, rebuilt(&started), NO_SENDER);
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
            // The live run marks what it sent right after each assembly,
            // and the shape line is the one every turn writes there
            // (prompt_assembled is written once per run), so a steer
            // after it lands where the live one landed.
            EventKind::PromptShapeCompared => conversation.mark_sent(),
            EventKind::SteerReceived => {
                let steer = record.data().read::<SteerReceived>()?;
                conversation.push_steer(
                    &crate::conversation::Speaker::from_recorded(&steer.source)?,
                    &steer.text,
                );
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
            // A fork rebuilds a model run's conversation, and what a
            // harness reported or answered is not one.
            | EventKind::HarnessReported
            | EventKind::HarnessAnswered
            // The remote door is the city's, never a run's conversation.
            | EventKind::RemoteOpened
            | EventKind::RemoteClosed
            | EventKind::DevicePaired
            | EventKind::DeviceRevoked
            | EventKind::RemoteSessionStarted
            // A save and a proposal are the person's page and a file on
            // disk, never a turn of the conversation.
            | EventKind::DocumentWritten
            | EventKind::ProposalOffered
            | EventKind::ProposalDecided
            | EventKind::ProposalWithdrawn
            // The sentence a policy change appends is not in the
            // conversation yet: no run reads the line today (kernel D21).
            | EventKind::RunPolicyChanged
            // A session's name and a skill's audit are the person's page.
            | EventKind::SessionNamed
            | EventKind::SkillAudited
            // How a reply wait ended reaches the conversation as the
            // `steer_received` line the turn writes when the run goes on
            // (collab D9), so it is rebuilt above; these two lines are
            // the wait's own accounting (kernel D32).
            | EventKind::SignalWaitStarted
            | EventKind::SignalWaitEnded => {}
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

/// The sender a rebuilt opening names: none, because [`rebuilt`] never
/// answers `FromJob`, the one opening that names who handed the work
/// down (`crates/city/spec/SpineFiles.lean` D21).
const NO_SENDER: &str = "";

/// How a branch writes the first message of a run it rebuilds: the way
/// the mother wrote it, so a provider's cached prefix holds from the
/// first message on (`crates/runtime/spec/Fork.lean` §8-58).
///
/// **`FromJob` is the one opening rewritten.** Its line says the task is
/// in the JOB.md above, and the JOB.md it means is the mother's, in the
/// run segment of her prefix; the branch's prefix carries its own brief,
/// so the line copied would point the branch at a file it was never
/// given. The rewrite costs the cache from the first message, and copying
/// her job file into the branch's run segment instead would cost it from
/// the system prompt (runtime D14).
///
/// A record from before `opening` was written is read the way it was
/// read before.
fn rebuilt(started: &RunStarted) -> Opening {
    match started.opening {
        Some(Opening::FromJob) => Opening::Inherited,
        Some(spoken @ (Opening::Inherited | Opening::WithPerson)) => spoken,
        None if started.job.is_some() => Opening::Inherited,
        None => Opening::WithPerson,
    }
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

mod indexed;
pub use indexed::inherited_indexed;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod compaction_tests;

#[cfg(test)]
mod request_tests;
