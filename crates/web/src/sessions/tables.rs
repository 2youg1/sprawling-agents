// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The tables: what is moving, what ended, and which buildings are busy.

use dioxus::prelude::*;

use super::listing::{SeatRow, spent_of};
use crate::app::Snapshot;
use crate::lang::{Lang, Msg, fill, say};
use crate::route::View;

/// The tables.
#[component]
pub fn Tables(
    in_flight: Vec<SeatRow>,
    ended: Vec<SeatRow>,
    city: Option<channels::CityAnswer>,
    snapshot: Snapshot,
    busy: std::collections::BTreeSet<channels::Address>,
    live: Signal<bool>,
    on_frame: EventHandler<channels::ClientFrame>,
    on_view: EventHandler<View>,
) -> Element {
    let lang = use_context::<Signal<Lang>>();
    let word = move |msg: Msg| say(lang(), msg);
    rsx! {

        section { class: "panel",
            div { class: "panel-head",
                h2 { class: "panel-title", "{word(Msg::NavSessions)}" }
                span { class: "panel-figure", "{in_flight.len()}" }
            }
            p { class: "panel-scope", "{word(Msg::SessionsScope)}" }
            div { class: "panel-body",
                if in_flight.is_empty() {
                    div { class: "empty",
                        span { class: "empty-status", "{word(Msg::SessionsNothingYet)}" }
                        span { class: "empty-what", "{word(Msg::SessionsNothingWhat)}" }
                    }
                } else {
                    for row in in_flight {
                        SessionRow { key: "{row.addr.as_str()}", row }
                    }
                }
            }
            p { class: "panel-source",
                {
                    fill(
                        word(Msg::SessionsSource),
                        &[
                            (
                                "seq",
                                &snapshot
                                    .applied_through()
                                    .map(|seq| seq.value().to_string())
                                    .unwrap_or_else(|| word(Msg::AskingWhatItHolds).to_owned()),
                            ),
                        ],
                    )
                }
            }
        }

        if !ended.is_empty() {
            section { class: "panel",
                div { class: "panel-head",
                    h2 { class: "panel-title", "{word(Msg::SessionsEnded)}" }
                }
                p { class: "panel-scope", "{word(Msg::SessionsEndedScope)}" }
                div { class: "panel-body",
                    for row in ended {
                        SessionRow { key: "{row.addr.as_str()}", row }
                    }
                }
                p { class: "panel-source", "{word(Msg::SessionsEndedSource)}" }
            }
        }

        // Which buildings are busy, drawn rather than listed. One block
        // on this page rather than a destination of its own: it is a way
        // of picturing the table above it, not a separate question.
        section { class: "panel",
            div { class: "panel-head",
                h2 { class: "panel-title", "{word(Msg::NavCity)}" }
            }
            p { class: "panel-scope", "{word(Msg::SessionsCityScope)}" }
            div { class: "panel-body",
                crate::city_view::CityView {
                    city,
                    busy: busy,
                    selected: None,
                    live,
                    on_frame,
                    on_select: move |_| {},
                    on_open: move |name: String| {
                        if let Some(addr) = crate::route::opened_building(Some(name.as_str())) {
                            on_view.call(View::Building(addr));
                        }
                    },
                }
            }
        }
    }
}

/// One session, as a row that is itself the link to it.
///
/// An anchor and not a button: writing the fragment is the only way a
/// view changes, and an `<a href>` already does that. It arrives with
/// the keyboard, the middle click, "copy link address" and the link role
/// a screen reader announces — all of which a button with an `onclick`
/// would have had to be given back one at a time.
#[component]
pub(super) fn SessionRow(row: SeatRow) -> Element {
    let lang = use_context::<Signal<Lang>>();
    let mark = row.phase;
    let said = row.said.clone().unwrap_or_default();
    rsx! {
        a {
            class: "session-row",
            href: "{crate::route::to_fragment(&View::Session(row.addr.clone()))}",
            span {
                class: "phase {mark.token()}",
                role: "img",
                "aria-label": "{say(lang(), mark.word())}",
            }
            span { class: "room", "{row.addr.as_str()}" }
            span { class: "said", "{said}" }
            span { class: "turn",
                {fill(say(lang(), Msg::SessionsTurnCount), &[("n", &row.turns.to_string())])}
            }
            span { class: "spent", "{spent_of(lang(), row.spent)}" }
        }
    }
}
