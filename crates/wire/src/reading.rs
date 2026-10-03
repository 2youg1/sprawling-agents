// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Reading one ledger payload into the wire's own values.
//!
//! **Why it is here and not on the server.** Both ends need it. The
//! server folds a session into rounds and answers `Query::Rounds` with
//! them; a client folds the pushed event stream forward into what it
//! believes, and reading "what the model said" out of a `model_returned`
//! payload is the same rule in both places (ARCHITECTURE.md section 5,
//! step 12: the same fold, on both sides of the wire). One authority,
//! two callers.
//!
//! Pure and fail-open: a field this build cannot read produces `None`
//! rather than an error, and a note whose payload will not read back as
//! its kind becomes a `Note::Unreadable` naming the failure, because a
//! view that hid a record it could not parse would be a view that lies
//! about what happened.

use kernel::event::record::{
    CheckpointCommitted, FileDiscarded, FiredAction, ProviderDegraded, WatchdogFired,
};
use kernel::{AxCode, AxError, EventKind, EventRecord, Seq};

use crate::answer::{Note, Output, Used};

/// The most lines of one tool's output a row carries.
pub const OUTPUT_LINES: usize = 12;

/// A payload field as a string, when it is one.
#[must_use]
pub fn text(value: Option<&serde_json::Value>) -> Option<String> {
    value.and_then(|held| held.as_str()).map(str::to_owned)
}

/// What the model said, out of the message `runtime::turn` recorded.
#[must_use]
pub fn said_in(message: &serde_json::Value) -> Option<String> {
    blocks_of(message, "text", "text")
}

/// What the model thought, out of the same message.
///
/// **Answered, although the blocks look like transport.** Thinking
/// blocks are carried end to end because the provider verifies the
/// signature it issued against them. For a model that spends most of a
/// call reasoning, withholding them leaves a person watching an empty
/// thread for minutes and then reading two sentences - so the reasoning
/// is answered as its own field, and the page folds it away beside the
/// prose rather than mixing the two.
///
/// `RedactedThinking` stays out: its payload is encrypted, so there is
/// nothing in it a person could read.
#[must_use]
pub fn thought_in(message: &serde_json::Value) -> Option<String> {
    blocks_of(message, "thinking", "thinking")
}

/// The blocks of one kind, joined, or nothing when there are none.
fn blocks_of(message: &serde_json::Value, kind: &str, field: &str) -> Option<String> {
    let blocks = message.as_object()?.get("content")?.as_array()?;
    let found: Vec<&str> = blocks
        .iter()
        .filter_map(|block| {
            let map = block.as_object()?;
            (map.get("kind")?.as_str()? == kind).then(|| map.get(field)?.as_str())?
        })
        .collect();
    (!found.is_empty()).then(|| found.join("\n"))
}

/// The counters `ModelUsage` carries, read through its own
/// deserialisation so a row written under an older meaning of
/// `input_tokens` is converted by the one reader that knows the versions.
/// Absent when the row carries no usage it can read, which is a
/// different fact from having spent nothing.
/// The two durations are not usage: `model_returned` keeps them beside
/// it, and the fold that holds the whole line fills them in.
#[must_use]
pub fn used_in(usage: &serde_json::Value) -> Option<Used> {
    let usage = <kernel::ModelUsage as serde::Deserialize>::deserialize(usage).ok()?;
    Some(Used {
        input: usage.input_tokens,
        output: usage.output_tokens,
        cached: usage.cache_read_tokens,
        cache_write: Some(usage.cache_write_tokens),
        first_us: None,
        took_us: None,
    })
}

/// What a tool said, cut to [`OUTPUT_LINES`] with the remainder counted.
///
/// A result too large to carry was replaced upstream by
/// `runtime::offload`, whose substitute already states its own size and
/// names the `Locator` holding the rest. This never reads that line: the
/// substitute's format has one authority and it is not this module.
#[must_use]
pub fn output_in(said: &serde_json::Value) -> Option<Output> {
    match said {
        serde_json::Value::String(text) => bounded(text),
        other @ (serde_json::Value::Null
        | serde_json::Value::Bool(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::Array(_)
        | serde_json::Value::Object(_)) => bounded(&other.to_string()),
    }
}

/// What a call was asked for, laid out over lines and cut at the same
/// limit as what it answered.
///
/// Laid out rather than compact: the bound is counted in lines, and a
/// whole argument object printed on one line would satisfy a line limit
/// while staying unreadable — the window has to cut something a person
/// would otherwise have read. A value that will not format falls back to
/// its compact form, because an unreadable argument is still worth more
/// than an absent one.
#[must_use]
pub fn arguments_in(args: &serde_json::Value) -> Option<Output> {
    if args.is_null() {
        return None;
    }
    let laid_out = serde_json::to_string_pretty(args).unwrap_or_else(|_| args.to_string());
    bounded(&laid_out)
}

/// The one place [`OUTPUT_LINES`] is applied.
///
/// Shared by what a call was asked for and what it answered, so the two
/// halves of a row are cut by the same rule and a reader comparing them
/// is comparing equal windows.
fn bounded(whole: &str) -> Option<Output> {
    if whole.trim().is_empty() {
        return None;
    }
    let head: Vec<&str> = whole.lines().take(OUTPUT_LINES).collect();
    let cut = whole.lines().count().saturating_sub(head.len());
    Some(Output {
        head: head.join("\n"),
        cut,
        pinned: None,
    })
}

/// Whether this kind changed what the turn did or what it waits on, and
/// what to say about it if so.
///
/// Exhaustive over the kinds that earn a note and closed against the
/// rest: an enum that grew an arm per event would be the event stream
/// with extra steps.
#[must_use]
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "an arm per event kind would be the event stream with extra steps"
)]
pub fn note_of(kind: EventKind, record: &EventRecord) -> Option<Note> {
    let at = record.seq();
    let map = record.data().as_map();
    match kind {
        // The carrier table in `kernel::error` decides which codes land
        // under which kind, and `runtime::run` writes the error flat
        // into the payload. A payload that will not read back as one is
        // kept as the failure to read it rather than rendered as a
        // refusal this build invented, or dropped as if nothing refused.
        EventKind::GateDenied | EventKind::BudgetLimit => {
            Some(match record.data().read::<AxError>() {
                Ok(error) => Note::Refused { error, at },
                Err(err) => unreadable(kind, &err, at),
            })
        }
        // The vault's fallback to session memory shares this kind and
        // changed nothing a turn did.
        EventKind::ProviderDegraded => match record.data().read() {
            Ok(ProviderDegraded::Refused(error)) => Some(Note::Refused { error, at }),
            Ok(ProviderDegraded::VaultFellBack(_)) => None,
            Err(err) => Some(unreadable(kind, &err, at)),
        },
        EventKind::WatchdogFired => match record.data().read::<WatchdogFired>() {
            Ok(fired) => backed_off(fired.action, at),
            Err(err) => Some(unreadable(kind, &err, at)),
        },
        // When the person answered is recorded under the city's own run,
        // which this one record cannot see; the rounds fold pairs it.
        EventKind::ApprovalRequested => Some(Note::Waiting {
            at,
            t: record.t(),
            answered: None,
        }),
        // The job pin that opens a dispatch is a `checkpoint_committed`
        // naming no commit, and the record type says so rather than
        // leaving this reader to infer it from a missing key.
        EventKind::CheckpointCommitted => match record.data().read() {
            Ok(CheckpointCommitted::Committed(commit)) => Some(Note::Checkpointed {
                oid: commit.oid,
                at,
            }),
            Ok(CheckpointCommitted::JobPinned { .. }) => None,
            Err(err) => Some(unreadable(kind, &err, at)),
        },
        EventKind::SteerReceived | EventKind::SignalConsumed => Some(Note::Arrived {
            from: text(map.get("source"))
                .or_else(|| text(map.get("from")))
                .unwrap_or_else(|| record.who().to_owned()),
            said: text(map.get("text")).unwrap_or_default(),
            at,
        }),
        EventKind::FileDiscarded => Some(match record.data().read::<FileDiscarded>() {
            Ok(discarded) => Note::Discarded {
                count: discarded.paths.len(),
                at,
            },
            Err(err) => unreadable(kind, &err, at),
        }),
        _ => None,
    }
}

/// A back-off is the provider's refusal, waited out: the line keeps the
/// failure's stable code and subject, and they are shown as that
/// refusal. The line records only the code and the subject, so the
/// action and the recovery are composed here from the kind of line it is,
/// not read back. A steer and a freeze earn no note of their own, because
/// the steer and the frozen run are lines of their own.
fn backed_off(action: FiredAction, at: Seq) -> Option<Note> {
    match action {
        FiredAction::BackOff {
            until_ms,
            code,
            subject,
        } => Some(match AxCode::parse(&code) {
            Some(code) => Note::Refused {
                error: AxError::failure(code, "call the provider", subject).with_recovery(format!(
                    "the watchdog calls again no earlier than {until_ms} ms"
                )),
                at,
            },
            None => unreadable(
                EventKind::WatchdogFired,
                &format!("unknown code {code}"),
                at,
            ),
        }),
        FiredAction::Steer { .. } | FiredAction::Freeze { .. } => None,
    }
}

/// The note a payload leaves when it will not read back as its kind.
fn unreadable(kind: EventKind, err: &dyn std::fmt::Display, at: Seq) -> Note {
    Note::Unreadable {
        cause: format!("{kind:?} payload did not read back: {err}"),
        at,
    }
}
