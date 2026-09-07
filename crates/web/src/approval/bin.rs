// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The bin: what was discarded and how it comes back.

use crate::lang::{Msg, say};
use channels::{ClientFrame, Locator, TimeMs};
use dioxus::prelude::*;

/// How a discarded thing comes back. Rendered as a sentence rather than a
/// type name, because the person reading it is deciding whether to bother.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReturnPath {
    /// Restorable from a checkpoint that already exists.
    FromCheckpoint(String),
    /// The bytes are in content-addressed storage.
    FromStore(String),
    /// Not stored, but reproducible: the reason says how.
    Rebuild(String),
    /// A restoration scheme this client is too old to describe.
    ///
    /// Fail-closed for a view means refusing to *invent an action*: the row
    /// still appears, because hiding a discarded thing would be worse, but
    /// it says "look at the Ledger" rather than offering a button whose
    /// behaviour this build cannot predict.
    Undescribed,
}

impl ReturnPath {
    /// Reads a `Restoration` as an instruction. Every arm has an answer,
    /// because a discard with no way back never became an event.
    #[must_use]
    pub fn of(restoration: &channels::Restoration) -> Self {
        match *restoration {
            channels::Restoration::Tracked(ref locator) => {
                Self::FromCheckpoint(render_locator(locator))
            }
            channels::Restoration::Interred(ref locator) => {
                Self::FromStore(render_locator(locator))
            }
            channels::Restoration::Rebuildable { ref reason } => Self::Rebuild(reason.clone()),
            _ => Self::Undescribed,
        }
    }

    /// The instruction, in the language the reader chose.
    ///
    /// Takes the language rather than answering in English, for the reason
    /// `ProviderHealth::word` does: a sentence assembled outside
    /// `web::lang` is a second authority for the wording, and the way it
    /// shows up is a Chinese page with one English row in it.
    #[must_use]
    pub fn sentence(&self, lang: crate::lang::Lang) -> String {
        match *self {
            Self::FromCheckpoint(ref at) => {
                crate::lang::fill(say(lang, Msg::BinRestoreCheckpoint), &[("at", at)])
            }
            Self::FromStore(ref at) => {
                crate::lang::fill(say(lang, Msg::BinRestoreStored), &[("at", at)])
            }
            Self::Rebuild(ref how) => {
                crate::lang::fill(say(lang, Msg::BinRebuild), &[("how", how)])
            }
            Self::Undescribed => say(lang, Msg::BinNoDescription).to_owned(),
        }
    }
}

fn render_locator(locator: &Locator) -> String {
    format!("{locator:?}")
}

/// One row of the Recycle Bin.
///
/// No byte count: `file_discarded` does not record one, and a column that
/// is zero on every row is not missing data but a lie repeated per row.
/// `restored` is here instead, because that is a thing the record does
/// know - `discard_restored` turns it true.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinRow {
    pub what: String,
    pub discarded_at: TimeMs,
    pub return_path: ReturnPath,
    pub restored: bool,
}

/// Reads the city's answer as rows. The one place the wire's shape
/// becomes the view's shape, so "how a discard is described" is decided
/// once and read everywhere.
#[must_use]
pub fn bin_rows(answer: &channels::DiscardAnswer) -> Vec<BinRow> {
    recycle_bin(
        answer
            .rows
            .iter()
            .map(|line| BinRow {
                what: line.path.clone(),
                discarded_at: line.at,
                return_path: line
                    .restoration
                    .as_ref()
                    .map_or(ReturnPath::Undescribed, ReturnPath::of),
                restored: line.restored,
            })
            .collect(),
    )
}

/// The Recycle Bin: what was discarded, newest first, each row stating
/// how it comes back.
///
/// There is no button on any row of this page. Two commands could have
/// carried one - `Rollback` for a checkpoint, and nothing at all for a
/// content address - and no city executes `Rollback`, so both rows would
/// have ended in a refusal. What a person gets instead is the sentence
/// [`ReturnPath::sentence`] writes: an instruction they can act on,
/// which is more than a button that only ever fails.
#[component]
pub fn RecycleBinView(
    answer: Option<channels::DiscardAnswer>,
    /// Whether the socket is live; see `app::Root`.
    live: Signal<bool>,
    on_frame: EventHandler<ClientFrame>,
) -> Element {
    let lang = use_context::<Signal<crate::lang::Lang>>();
    let word = move |msg: Msg| say(lang(), msg);
    let asked = use_signal(|| false);
    use_effect(move || {
        let mut asked = asked;
        if live() && !asked() {
            asked.set(true);
            on_frame.call(ClientFrame::Query(channels::Query::DiscardView));
        }
    });
    let Some(answer) = answer else {
        return rsx! {
            section { class: "recycle-bin",
                crate::panel::Empty {
                    status: word(Msg::BinAsking).to_owned(),
                    what: word(Msg::BinAskingWhat).to_owned(),
                }
            }
        };
    };
    let rows = bin_rows(&answer);
    let count = rows.len();
    let outstanding = rows.iter().filter(|row| !row.restored).count();
    rsx! {
        section { class: "recycle-bin",
            crate::panel::Panel {
                title: if rows.is_empty() {
                        word(Msg::BinNothingDiscarded).to_owned()
                    } else {
                        word(Msg::BinTitle).to_owned()
                    },
                figure: (outstanding > 0).then(|| outstanding.to_string()),
                scope: word(Msg::BinScope).to_owned(),
                source: word(Msg::BinSource).to_owned(),
            if rows.is_empty() {
                crate::panel::Empty {
                    status: word(Msg::BinNoneYet).to_owned(),
                    what: word(Msg::BinNoneYetWhat).to_owned(),
                }
            }
            for row in rows {
                article {
                    key: "{row.what}",
                    class: if row.restored { "binned back" } else { "binned" },
                    span { class: "what", "{row.what}" }
                    span { class: "way-back", "{row.return_path.sentence(lang())}" }
                    if row.restored {
                        span { class: "note", "{word(Msg::BinAlreadyRestored)}" }
                    }
                }
            }
            if count > 0 {
                p { class: "note",
                    "{word(Msg::BinRollbackNote)}"
                }
            }
            }
        }
    }
}

/// Orders the Recycle Bin newest first: the mistake a person is looking for
/// is almost always the last one they made.
#[must_use]
pub fn recycle_bin(mut rows: Vec<BinRow>) -> Vec<BinRow> {
    rows.sort_by(|left, right| {
        right
            .discarded_at
            .cmp(&left.discarded_at)
            .then_with(|| left.what.cmp(&right.what))
    });
    rows
}
