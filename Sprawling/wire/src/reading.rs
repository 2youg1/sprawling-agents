// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::event::record::{
    CheckpointCommitted, FileDiscarded, FiredAction, ProviderDegraded, SignalConsumed,
    SignalWaitStarted, SteerReceived, WatchdogFired,
};
use kernel::{AxCode, AxError, EventKind, EventRecord, Seq, TimeMs};

use crate::answer::{Note, Output, Speaker, Used};

pub const OUTPUT_LINES: usize = 12;

#[must_use]
pub fn text(value: Option<&serde_json::Value>) -> Option<String> {
    value.and_then(|held| held.as_str()).map(str::to_owned)
}

#[must_use]
pub fn said_in(message: &serde_json::Value) -> Option<String> {
    blocks_of(message, "text", "text")
}

#[must_use]
pub fn thought_in(message: &serde_json::Value) -> Option<String> {
    blocks_of(message, "thinking", "thinking")
}

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

#[must_use]
pub fn used_in(usage: &serde_json::Value) -> Option<Used> {
    let usage = <kernel::ModelUsage as serde::Deserialize>::deserialize(usage).ok()?;
    Some(Used {
        input: usage.input_tokens,
        output: usage.output_tokens,
        cached: usage.cache_read_tokens.reported(),
        cache_write: usage.cache_write_tokens.reported(),
        first_us: None,
        took_us: None,
    })
}

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

#[must_use]
pub fn arguments_in(args: &serde_json::Value) -> Option<Output> {
    if args.is_null() {
        return None;
    }
    let laid_out = serde_json::to_string_pretty(args).unwrap_or_else(|_| args.to_string());
    bounded(&laid_out)
}

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

#[must_use]
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "an arm per event kind would be the event stream with extra steps"
)]
pub fn note_of(kind: EventKind, record: &EventRecord) -> Option<Note> {
    let at = record.seq();
    match kind {
        EventKind::GateDenied | EventKind::BudgetLimit => {
            Some(match record.data().read::<AxError>() {
                Ok(error) => Note::Refused { error, at },
                Err(err) => unreadable(kind, &err, at),
            })
        }

        EventKind::ProviderDegraded => match record.data().read() {
            Ok(ProviderDegraded::Refused(error)) => Some(Note::Refused { error, at }),
            Ok(ProviderDegraded::VaultFellBack(_)) => None,
            Err(err) => Some(unreadable(kind, &err, at)),
        },
        EventKind::WatchdogFired => match record.data().read::<WatchdogFired>() {
            Ok(fired) => backed_off(fired.action, at),
            Err(err) => Some(unreadable(kind, &err, at)),
        },

        EventKind::ApprovalRequested => Some(Note::Waiting {
            at,
            t: record.t(),
            answered: None,
        }),

        EventKind::CheckpointCommitted => match record.data().read() {
            Ok(CheckpointCommitted::Committed(commit)) => Some(Note::Checkpointed {
                oid: commit.oid,
                at,
            }),
            Ok(CheckpointCommitted::JobPinned { .. }) => None,
            Err(err) => Some(unreadable(kind, &err, at)),
        },
        EventKind::SteerReceived => Some(match record.data().read::<SteerReceived>() {
            Ok(steer) => Note::Arrived {
                by: speaker_of(&steer.source),

                from: steer.source.split(' ').next().map(str::to_owned),
                said: Some(steer.text),
                t: record.t(),
                handback: None,
                kind: None,
                session: None,
                at,
            },
            Err(err) => unreadable(kind, &err, at),
        }),

        EventKind::SignalConsumed => Some(match record.data().read::<SignalConsumed>() {
            Ok(_taken) => Note::Arrived {
                from: None,
                said: None,
                by: Speaker::Resident,
                t: record.t(),
                handback: None,
                kind: None,
                session: None,
                at,
            },
            Err(err) => unreadable(kind, &err, at),
        }),

        EventKind::SignalWaitStarted => Some(match record.data().read::<SignalWaitStarted>() {
            Ok(started) => Note::AwaitingReply {
                on: started.on,
                until: TimeMs::new(started.deadline_ms),
                t: record.t(),
                ended: None,
                at,
            },
            Err(err) => unreadable(kind, &err, at),
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

fn speaker_of(source: &str) -> Speaker {
    match source {
        SteerReceived::PERSON_SOURCE => Speaker::User,
        SteerReceived::CITY_SOURCE => Speaker::City,
        _resident => Speaker::Resident,
    }
}

fn backed_off(action: FiredAction, at: Seq) -> Option<Note> {
    let (code, subject, recovery) = match action {
        FiredAction::BackOff {
            until_ms,
            code,
            subject,
        } => (
            code,
            subject,
            format!("the watchdog calls again no earlier than {until_ms} ms"),
        ),
        FiredAction::Switch { to, code, subject } => (
            code,
            subject,
            format!("the watchdog calls again at once on the account {to}"),
        ),
        FiredAction::Steer { .. } | FiredAction::Freeze { .. } => return None,
    };
    Some(match AxCode::parse(&code) {
        Some(code) => Note::Refused {
            error: AxError::failure(code, "call the provider", subject).with_recovery(recovery),
            at,
        },
        None => unreadable(
            EventKind::WatchdogFired,
            &format!("unknown code {code}"),
            at,
        ),
    })
}

fn unreadable(kind: EventKind, err: &dyn std::fmt::Display, at: Seq) -> Note {
    Note::Unreadable {
        cause: format!("{kind:?} payload did not read back: {err}"),
        at,
    }
}
