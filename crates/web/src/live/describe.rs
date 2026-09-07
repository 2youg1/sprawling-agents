// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One event, one short line.

use crate::lang::{Msg, fill, say};
use channels::{EventKind, EventRecord};
use dioxus::prelude::*;

/// One line of text for an event.
///
/// Deliberately short and deliberately not the payload: a live view that
/// prints raw payloads is a log, and the reason to watch a session is to
/// see its shape, not its bytes. The bytes are one click away in
/// `ledger_view`.
///
/// **Amended (ux-9).** That ruling holds and it was answering the wrong
/// question. What it forbids is a dump; what it was read as forbidding is
/// any disclosure at all, and the cost of the wider reading is that
/// everything this product does differently happens inside a single turn
/// and reaches the screen as another grey line - a refusal in three
/// parts, a checkpoint fence, a write outside its domain, a compaction
/// that reports what it dropped. `read` and `read src/lex.rs` differ by
/// nothing a byte count can measure: the second says what it did.
///
/// So this function keeps its job, which is one short line per event, and
/// `web::turn` folds the same records into the rounds a person reads. The
/// bytes are still only in the Ledger, and a call still carries the `seq`
/// that addresses them.
#[must_use]
pub fn describe(record: &EventRecord) -> (Option<Msg>, String) {
    let who = record
        .addr()
        .map_or_else(|| record.who().to_owned(), |addr| addr.as_str().to_owned());
    let msg = match record.kind() {
        EventKind::ToolCalled => Some(Msg::LineToolCalled),
        EventKind::ToolResult => Some(Msg::LineToolResult),
        EventKind::ModelCalled => Some(Msg::LineModelCalled),
        EventKind::ModelReturned => Some(Msg::LineModelReturned),
        EventKind::SteerReceived => Some(Msg::LineSteered),
        EventKind::GateDenied => Some(Msg::LineGateDenied),
        EventKind::ApprovalRequested => Some(Msg::LineApprovalRequested),
        EventKind::RunFrozen => Some(Msg::LineRunFrozen),
        // A kind this build has no sentence for still produces a line:
        // the kind's own name beside who did it.
        _ => None,
    };
    (msg, who)
}

/// What this session did to the disk.
///
/// The counts come from git between two real commits; nothing here is
/// folded from an event, because the fence is the authority on what
/// moved and a second reading of it would be a second answer.
///
/// `None` is "not asked yet", which is a different fact from "changed
/// nothing" - so the block is absent rather than empty, and an empty
/// answer says so in its own words.
#[component]
pub fn Changed(changes: Option<channels::ChangesAnswer>) -> Element {
    let lang = use_context::<Signal<crate::lang::Lang>>();
    let word = move |msg: Msg| say(lang(), msg);
    let Some(moved) = changes else {
        return rsx! {};
    };
    rsx! {
        details { class: "changed", open: true,
            summary {
                "{fill(word(Msg::ChangedFiles), &[(\"count\", &moved.files.len().to_string())])}"
            }
            if moved.files.is_empty() {
                p { class: "note", "{word(Msg::ChangedNothing)}" }
            }
            for file in moved.files.clone() {
                div { key: "{file.path}", class: "changed-file",
                    span { class: "how", "{word(how_word(&file.how))}" }
                    span { class: "path", "{file.path}" }
                    match file.lines {
                        channels::Lines::Counted { added, removed } => rsx! {
                            span { class: "added", "+{added}" }
                            span { class: "removed", "\u{2212}{removed}" }
                        },
                        // No count exists for these bytes, and a zero
                        // would be a measurement nobody made.
                        channels::Lines::Binary => rsx! {
                            span { class: "binary", "{word(Msg::ChangedBinary)}" }
                        },
                    }
                }
            }
        }
    }
}

/// One line, said. `None` from [`describe`] falls back to the event's own
/// kind, which is English in the Ledger and stays English here.
#[must_use]
pub fn describe_in(lang: crate::lang::Lang, record: &EventRecord) -> String {
    match describe(record) {
        (Some(msg), who) => crate::lang::fill(crate::lang::say(lang, msg), &[("who", &who)]),
        (None, who) => format!("{:?} · {who}", record.kind()),
    }
}

/// What happened to one file, as a word rather than a symbol.
///
/// Colour cannot carry it: this design has two chromatic tokens and both
/// already mean something else, so the status is a word and the counts
/// are signs - the same instrument `Outcome::Failed` uses.
fn how_word(how: &channels::How) -> Msg {
    match *how {
        channels::How::Added => Msg::ChangedAdded,
        channels::How::Modified => Msg::ChangedModified,
        channels::How::Deleted => Msg::ChangedDeleted,
        channels::How::Renamed { .. } => Msg::ChangedRenamed,
    }
}
