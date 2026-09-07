// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The faces: what each open leaf shows.

use crate::lang::{Msg, fill, say};
use channels::{BuildingAnswer, ClientFrame, InboxAnswer};
use dioxus::prelude::*;

use super::leaf::Leaf;
use super::room::waiting_in;
use super::room::{RoomQueue, day_label};
use super::text::{class_of, pieces};

/// The faces.
#[component]
pub fn Faces(
    showing: Leaf,
    answer: BuildingAnswer,
    docs: Vec<channels::BuildingDoc>,
    inbox: Option<InboxAnswer>,
    pursuits: Vec<channels::PursuitLine>,
    on_frame: EventHandler<ClientFrame>,
) -> Element {
    let lang = use_context::<Signal<crate::lang::Lang>>();
    let word = move |msg: Msg| say(lang(), msg);
    rsx! {
            match showing {
                Leaf::Plan => rsx! {
                    crate::pursuit::PursuitView {
                        addr: answer.addr.clone(),
                        pursuits: pursuits.clone(),
                        on_frame,
                    }
                    crate::pursuit::AutonomyView {
                        scope: channels::HaltScope::Building(answer.addr.clone()),
                        on_frame,
                    }
                    crate::board::BoardView { answer: answer.clone() }
                },
                Leaf::Room(ref room) => rsx! {
                    div { class: "mailbox",
                        match waiting_in(inbox.as_ref(), &answer.addr, room) {
                            RoomQueue::Unasked => rsx! {
                                crate::panel::Empty {
                                    status: fill(word(Msg::BuildingAskingRoom), &[("room", room.as_str())]),
                                    what: word(Msg::BuildingAskingRoomWhat).to_owned(),
                                }
                            },
                            RoomQueue::Empty => rsx! {
                                crate::panel::Empty {
                                    status: fill(word(Msg::BuildingRoomEmpty), &[("room", room.as_str())]),
                                    what: word(Msg::BuildingRoomEmptyWhat).to_owned(),
                                }
                            },
                            RoomQueue::Waiting(lines) => rsx! {
                                p { class: "note",
                                    "{fill(word(Msg::BuildingWaitingCount), &[(\"count\", &lines.len().to_string())])}"
                                }
                                for line in lines {
                                    div { key: "{line.id}", class: "waiting",
                                        span { class: "kind", "{line.kind}" }
                                        span { class: "from", "{fill(word(Msg::BuildingSignalFrom), &[(\"who\", &line.from)])}" }
                                        span { class: "id", "{line.id}" }
                                    }
                                }
                            },
                        }
                    }
                },
                Leaf::Reach => rsx! {
                    crate::reach::ReachForm {
                        addr: answer.addr.clone(),
                        sandbox: answer.sandbox.clone(),
                        servers: answer.mcp.clone(),
                        on_frame,
                    }
                },
                Leaf::Archive => rsx! {
                    div { class: "archive",
                        if answer.archive.is_empty() {
                            crate::panel::Empty {
                                status: word(Msg::BuildingNothingFiled).to_owned(),
                                what: word(Msg::BuildingNothingFiledWhat).to_owned(),
                            }
                        }
                        for line in answer.archive.clone() {
                            div { key: "{line.day}-{line.subject}", class: "filed",
                                span { class: "day", "{day_label(line.day)}" }
                                span { class: "kind", "{line.kind}" }
                                span { class: "subject", "{line.subject}" }
                            }
                        }
                    }
                },
                Leaf::Doc(ref name) => {
                    let held = docs.iter().find(|doc| &doc.name == name);
                    match held {
                        Some(doc) => rsx! {
                            article { class: "doc",
                                p { class: "doc-note",
                                    {fill(word(Msg::BuildingDocSize),
                                          &[("name", &doc.name), ("bytes", &doc.bytes.to_string())])}
                                    if doc.truncated {
                                        "{word(Msg::BuildingTruncated)}"
                                    }
                                }
                                // Read as spans rather than shown as one
                                // wall of bytes. The lexing is
                                // `kernel::markdown`; this only walks
                                // what it returned, which is why the
                                // view has no rule of its own about
                                // which mark outranks which.
                                pre { class: "doc-text",
                                    for (at, piece, said) in pieces(&doc.text) {
                                        span {
                                            key: "{at}",
                                            class: match piece {
                                                Some(token) => class_of(token),
                                                None => "tok",
                                            },
                                            "{said}"
                                        }
                                    }
                                }
                            }
                        },
                        None => rsx! {
                            crate::panel::Empty {
                                status: format!("{name} is not in this building"),
                                what: word(Msg::BuildingNoDocument)
                                    .to_owned(),
                            }
                        },
                    }
                }
            }
    }
}
