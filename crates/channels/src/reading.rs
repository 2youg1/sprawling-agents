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
//! Pure and fail-open: a payload this build cannot read produces `None`
//! rather than an error, because a view that hid a record it could not
//! parse would be a view that lies about what happened.

use kernel::{EventKind, EventRecord, GitOid};

use crate::answer::{Note, Output, Used};

/// The most lines of one tool's output a row carries.
pub const OUTPUT_LINES: usize = 12;

/// Argument names that say what a call acted on, in the order they are
/// preferred. Taken from the tool definitions rather than guessed:
/// `path` is what twelve of them take, and the rest name their one
/// subject.
const SUBJECT_KEYS: [&str; 4] = ["path", "addr", "program", "arm"];

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
/// **This used to be withheld, and the reason it was no longer holds.**
/// Thinking blocks are carried end to end because the provider verifies
/// the signature it issued against them, and that made them look like
/// transport rather than content. For a model that spends most of a
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

/// The counters `ModelUsage` carries. Absent when the provider sent no
/// usage, which is a different fact from having spent nothing.
#[must_use]
pub fn used_in(usage: &serde_json::Value) -> Option<Used> {
    let map = usage.as_object()?;
    let counter = |name: &str| {
        map.get(name)
            .and_then(serde_json::Value::as_u64)
            .map(kernel::Tokens::new)
    };
    let input = counter("input_tokens")?;
    let output = counter("output_tokens")?;
    Some(Used {
        input,
        output,
        cached: counter("cache_read_tokens").unwrap_or_default(),
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
    let whole = match said {
        serde_json::Value::String(text) => text.clone(),
        other => other.to_string(),
    };
    if whole.trim().is_empty() {
        return None;
    }
    let head: Vec<&str> = whole.lines().take(OUTPUT_LINES).collect();
    let cut = whole.lines().count().saturating_sub(head.len());
    Some(Output {
        head: head.join("\n"),
        cut,
    })
}

/// The one argument a person recognises a call by.
#[must_use]
pub fn subject_of(args: Option<&serde_json::Value>) -> Option<String> {
    let map = args?.as_object()?;
    for key in SUBJECT_KEYS {
        if let Some(named) = text(map.get(key)) {
            return Some(named);
        }
    }
    // A tool this build has no preferred key for still says something,
    // rather than falling back to the bare tool name.
    map.values().find_map(|value| text(Some(value)))
}

/// Whether this kind changed what the turn did or what it waits on, and
/// what to say about it if so.
///
/// Exhaustive over the kinds that earn a note and closed against the
/// rest: an enum that grew an arm per event would be the event stream
/// with extra steps.
#[must_use]
pub fn note_of(kind: EventKind, record: &EventRecord) -> Option<Note> {
    let at = record.seq();
    let map = record.data().as_map();
    match kind {
        // The carrier table in `kernel::error` decides which codes land
        // under which kind, and `runtime::run` writes the error flat
        // into the payload. A payload that will not read back as one is
        // left to the event stream rather than rendered as a refusal
        // this build invented.
        EventKind::GateDenied
        | EventKind::BudgetLimit
        | EventKind::WatchdogFired
        | EventKind::ProviderDegraded => {
            let value = serde_json::Value::Object(map.clone());
            serde_json::from_value(value)
                .ok()
                .map(|error| Note::Refused { error, at })
        }
        EventKind::ApprovalRequested => Some(Note::Waiting { at }),
        EventKind::CheckpointCommitted => text(map.get("oid"))
            .as_deref()
            .and_then(GitOid::parse)
            .map(|oid| Note::Fenced { oid, at }),
        EventKind::SteerReceived | EventKind::SignalConsumed => Some(Note::Arrived {
            from: text(map.get("source"))
                .or_else(|| text(map.get("from")))
                .unwrap_or_else(|| record.who().to_owned()),
            said: text(map.get("text")).unwrap_or_default(),
            at,
        }),
        EventKind::FileDiscarded => Some(Note::Discarded {
            count: map
                .get("paths")
                .and_then(serde_json::Value::as_array)
                .map_or(1, Vec::len),
            at,
        }),
        _ => None,
    }
}
