// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One building, what it has written down, and what waits in each room.

use crate::lang::{Msg, fill, say};
use channels::{Address, BuildingAnswer, ClientFrame, InboxAnswer, Query};
use dioxus::prelude::*;

use super::faces::Faces;
use super::leaf::{Leaf, opening_leaf, room_addr};

/// Which of this page drop zones a drag is currently over.
///
/// Exhaustive rather than a name and a sentinel: "the building" and "a
/// room called nothing" are not the same state, and a pair of strings
/// could spell the second.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Over {
    Nothing,
    Building,
    Room(String),
}

#[component]
pub fn BuildingView(
    addr: Address,
    answer: Option<BuildingAnswer>,
    /// The city's standing goals. Read from the city answer rather than
    /// the building one: a pursuit is declared at the depth-zero
    /// position, so the city is the thing that knows about all of them.
    pursuits: Vec<channels::PursuitLine>,
    /// What waits in the room this page last asked about.
    inbox: Option<InboxAnswer>,
    /// How many signal events the stream has carried. A change means the
    /// open room's queue may have moved, so the page asks again.
    signals: u64,
    /// Whether the socket is live; see `app::Root`.
    live: Signal<bool>,
    on_frame: EventHandler<ClientFrame>,
    /// Points the control surface at this building, so the one form
    /// that starts work is the one form that starts work.
    on_select: EventHandler<Option<String>>,
    /// Where a gesture goes. The page reads no drag itself: what one
    /// means is `web::drop`'s answer, and where it goes is the root's.
    on_drop: EventHandler<(crate::drop::Target, crate::drop::Dropped)>,
) -> Element {
    let lang = use_context::<Signal<crate::lang::Lang>>();
    let word = move |msg: Msg| say(lang(), msg);
    let asked = use_signal(|| None::<String>);
    let wanted = addr.as_str().to_owned();
    use_effect(use_reactive!(|(wanted, live)| {
        let mut asked = asked;
        if live()
            && asked().as_deref() != Some(wanted.as_str())
            && let Ok(addr) = Address::parse(&wanted)
        {
            asked.set(Some(wanted.clone()));
            on_frame.call(ClientFrame::Query(Query::BuildingView { addr }));
        }
    }));
    let mut leaf = use_signal(|| None::<Leaf>);

    // The room's mailbox is asked for when a room is opened, and again
    // when the stream says a signal moved. `signals` is a count of those
    // events, not their content: the queue itself is the city's answer,
    // and folding one here would be a second place that claims to know
    // what waits in a room.
    let showing_room = match leaf() {
        Some(Leaf::Room(ref name)) => Some(name.clone()),
        _ => None,
    };
    let of_building = addr.clone();
    use_effect(use_reactive!(|(
        showing_room,
        signals,
        live,
        of_building,
    )| {
        let _ = signals;
        if live()
            && let Some(ref room) = showing_room
            && let Some(room_at) = room_addr(&of_building, room)
        {
            on_frame.call(ClientFrame::Query(Query::InboxView { addr: room_at }));
        }
    }));
    let Some(answer) = answer else {
        return rsx! {
            section { class: "building",
                crate::panel::Empty {
                    status: fill(word(Msg::BuildingAsking), &[("addr", addr.as_str())]),
                    what: word(Msg::BuildingAskingWhat).to_owned(),
                }
            }
        };
    };
    let showing = leaf().unwrap_or_else(|| opening_leaf(&answer));
    // Which zone a drag is over. One signal rather than one per room: a
    // drag is in one place at a time, so a set of booleans could spell a
    // state that cannot happen. `dragleave` always fires - even on a
    // cancelled drag - so the way back to `Over::Nothing` does not depend
    // on a drop happening.
    let mut over = use_signal(|| Over::Nothing);
    let archive_tab = fill(
        word(Msg::BuildingArchiveTab),
        &[("count", &answer.archive.len().to_string())],
    );
    let docs = answer.docs.clone();
    let rooms = answer.rooms.len();
    rsx! {
        section { class: "building",
            crate::panel::Panel {
                title: fill(word(Msg::BuildingTitle), &[("addr", answer.addr.as_str())]),
                figure: (rooms > 0).then(|| rooms.to_string()),
                scope: word(Msg::BuildingScope).to_owned(),
                source: word(Msg::BuildingSource).to_owned(),
            header { class: "building-head",
                // A drop zone rather than a decoration: work aimed by
                // dragging lands on a place, and this header is the
                // building. What the drop means is decided in one
                // function; this only says where it landed.
                h2 {
                    class: if over() == Over::Building { "drop-zone over" } else { "drop-zone" },
                    title: "{word(Msg::DropHere)}",
                    ondragover: move |event| event.prevent_default(),
                    ondragenter: move |event| {
                        event.prevent_default();
                        over.set(Over::Building);
                    },
                    ondragleave: move |_| over.set(Over::Nothing),
                    ondrop: {
                        let here = answer.addr.clone();
                        move |event: Event<DragData>| {
                            event.prevent_default();
                            over.set(Over::Nothing);
                            on_drop.call((
                                crate::drop::Target::Place(here.clone()),
                                crate::drop::from_event(&event),
                            ));
                        }
                    },
                    "{answer.addr.as_str()}"
                }
                // Not a fourth dispatch form: this fills the bar at the
                // bottom of the window, which is where work is started
                // from every page and where a person looks for it next
                // time.
                button {
                    class: "start-here",
                    onclick: {
                        let here = answer.addr.as_str().to_owned();
                        move |_| on_select.call(Some(here.clone()))
                    },
                    "{word(Msg::BuildingStartHere)}"
                }
                crate::progress::ProgressBar {
                    bar: crate::progress::bar(
                        &answer.progress,
                        false,
                        crate::progress::Subject::Plan,
                        lang(),
                    ),
                }
                if answer.rooms.is_empty() {
                    span { class: "rooms", "{word(Msg::BuildingNoRooms)}" }
                }
            }
            for problem in answer.problems.clone() {
                p { key: "{problem}", class: "problems",
                    "{fill(word(Msg::BuildingUnreadableRow), &[(\"problem\", &problem)])}"
                }
            }
            div { class: "tabs",
                if !answer.plan.is_empty() {
                    button {
                        class: "tab",
                        "aria-current": if showing == Leaf::Plan { "true" } else { "false" },
                        onclick: move |_| leaf.set(Some(Leaf::Plan)),
                        "{word(Msg::BuildingPlanTab)}"
                    }
                }
                for doc in docs.clone() {
                    button {
                        key: "{doc.name}",
                        class: "tab",
                        "aria-current": if showing == Leaf::Doc(doc.name.clone()) { "true" } else { "false" },
                        onclick: {
                            let name = doc.name.clone();
                            move |_| leaf.set(Some(Leaf::Doc(name.clone())))
                        },
                        "{doc.name}"
                    }
                }
                button {
                    class: "tab",
                    "aria-current": if showing == Leaf::Archive { "true" } else { "false" },
                    onclick: move |_| leaf.set(Some(Leaf::Archive)),
                    "{archive_tab}"
                }
                button {
                    class: "tab",
                    "aria-current": if showing == Leaf::Reach { "true" } else { "false" },
                    onclick: move |_| leaf.set(Some(Leaf::Reach)),
                    "{word(Msg::BuildingReachTab)}"
                }
                // The rooms are listed here and nowhere else on this page:
                // a second list of them would be a second answer to "what
                // is in this building".
                for room in answer.rooms.clone() {
                    button {
                        key: "room-{room}",
                        class: if over() == Over::Room(room.clone()) {
                            "tab room drop-zone over"
                        } else {
                            "tab room drop-zone"
                        },
                        title: "{word(Msg::DropHere)}",
                        "aria-current": if showing == Leaf::Room(room.clone()) { "true" } else { "false" },
                        onclick: {
                            let name = room.clone();
                            move |_| leaf.set(Some(Leaf::Room(name.clone())))
                        },
                        ondragover: move |event| event.prevent_default(),
                        ondragenter: {
                            let name = room.clone();
                            move |event: Event<DragData>| {
                                event.prevent_default();
                                over.set(Over::Room(name.clone()));
                            }
                        },
                        ondragleave: move |_| over.set(Over::Nothing),
                        ondrop: {
                            let landed = room_addr(&answer.addr, &room);
                            move |event: Event<DragData>| {
                                event.prevent_default();
                                over.set(Over::Nothing);
                                // A room name this city cannot address
                                // is not a place. It is not a session
                                // either, which is what this used to
                                // say - the refusal now names what
                                // actually happened.
                                let target = match landed.clone() {
                                    Some(addr) => crate::drop::Target::Place(addr),
                                    None => crate::drop::Target::Nowhere,
                                };
                                on_drop.call((target, crate::drop::from_event(&event)));
                            }
                        },
                        "{room}/"
                    }
                }
            }            Faces { showing: showing.clone(), answer: answer.clone(), docs: docs.clone(), inbox: inbox.clone(), pursuits: pursuits.clone(), on_frame: on_frame }
            }
        }
    }
}
