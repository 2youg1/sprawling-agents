// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The turns: one row per turn, what it did inside it.

use crate::lang::{Msg, fill, say};
use dioxus::prelude::*;

use super::commands::{cut_line, short_oid, tokens_line};

/// The turns: one row per turn, what it did inside it.
#[component]
pub fn Rounds(turns: Vec<crate::turn::Turn>) -> Element {
    let lang = use_context::<Signal<crate::lang::Lang>>();
    let word = move |msg: Msg| say(lang(), msg);
    rsx! {
            // A turn is one row, and what it did is inside it. The event
            // stream is the Ledger's shape; this is the reader's, and
            // both are readings of the same records.
            ol { class: "turns",
                for round in turns {
                    li { key: "{round.opened.value()}", class: "turn",
                        header { class: "turn-head",
                            span { class: "n",
                                "{fill(word(Msg::TurnNumber), &[(\"n\", &round.number.to_string())])}"
                            }
                            span { class: "seq", "{round.opened.value()}" }
                            span { class: "count",
                                if round.calls.is_empty() {
                                    "{word(Msg::TurnNoTools)}"
                                } else {
                                    "{fill(word(Msg::TurnTools), &[(\"count\", &round.calls.len().to_string())])}"
                                }
                            }
                            // What this one turn cost. Both numbers are
                            // in `model_returned`; neither is derived
                            // here, and the token figure carries no
                            // denominator because the wire has none.
                            if let Some(spent) = round.spent {
                                span { class: "spent", "{crate::readout::render_usd(spent)}" }
                            }
                            if let Some(used) = round.used {
                                span { class: "used", "{tokens_line(lang(), used)}" }
                            }
                            if let Some(ref why) = round.stopped {
                                span { class: "stopped",
                                    "{fill(word(Msg::TurnStopped), &[(\"why\", why)])}"
                                }
                            }
                        }
                        // What the model actually said. The reason this
                        // page exists rather than a tool log, and the
                        // one thing the fold used to drop wholesale.
                        if let Some(ref said) = round.said {
                            p { class: "said", "{said}" }
                        }
                        for call in round.calls {
                            div { key: "{call.at.value()}", class: "call",
                                span { class: "tool", "{call.tool}" }
                                // What it acted on. Absent when the
                                // arguments name no one thing, which is
                                // what every row used to look like.
                                if let Some(ref on) = call.subject {
                                    span { class: "arg", "{on}" }
                                }
                                span { class: "{call.outcome.class()}",
                                    "{word(call.outcome.word())}"
                                }
                                // Where the bytes are. The row shows a
                                // shape; this addresses the rest of it.
                                span { class: "seq", "{call.at.value()}" }
                                // A disclosure, not a dump: one click,
                                // bounded, and it states what it cut.
                                if let Some(ref said) = call.output {
                                    details { class: "output",
                                        summary { "{word(Msg::TurnOutput)}" }
                                        pre { "{said.head}" }
                                        if said.cut > 0 {
                                            p { class: "cut", "{cut_line(lang(), said.cut, call.at)}" }
                                        }
                                    }
                                }
                            }
                        }
                        // What else happened in this turn. Everything
                        // here changed what the turn did or what it is
                        // waiting on; the rest stays in the stream.
                        for note in round.notes {
                            div { key: "note-{note.at().value()}", class: "note",
                                match note {
                                    crate::turn::Note::Refused { ref error, .. } => {
                                        let said = crate::alert::refused(lang(), error);
                                        rsx! {
                                            span { class: "code", "{said.code}" }
                                            span { class: "what", "{said.what}" }
                                            span { class: "recovery", "{said.recovery}" }
                                        }
                                    }
                                    crate::turn::Note::Fenced { oid, .. } => rsx! {
                                        span { class: "what",
                                            "{fill(word(Msg::NoteFenced), &[(\"oid\", &short_oid(oid))])}"
                                        }
                                    },
                                    crate::turn::Note::Waiting { .. } => rsx! {
                                        span { class: "what", "{word(Msg::NoteWaiting)}" }
                                    },
                                    crate::turn::Note::Arrived { ref from, ref said, .. } => rsx! {
                                        span { class: "what",
                                            "{fill(word(Msg::NoteArrived), &[(\"from\", from)])}"
                                        }
                                        if !said.is_empty() {
                                            span { class: "arg", "{said}" }
                                        }
                                    },
                                    crate::turn::Note::Discarded { count, .. } => rsx! {
                                        span { class: "what",
                                            "{fill(word(Msg::NoteDiscarded), &[(\"count\", &count.to_string())])}"
                                        }
                                    },
                                }
                                span { class: "seq", "{note.at().value()}" }
                            }
                        }
                    }
                }
            }
    }
}
