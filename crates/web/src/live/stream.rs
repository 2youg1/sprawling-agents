// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The stream and the empty states: raw order one click down.

use crate::lang::{Msg, say};
use channels::RunId;
use dioxus::prelude::*;

use super::commands::line_text;

/// The stream and the empty states: raw order one click down.
#[component]
pub fn Stream(
    feed: super::feed::Feed,
    lines: Vec<super::feed::Line>,
    held: usize,
    run: Option<RunId>,
) -> Element {
    let lang = use_context::<Signal<crate::lang::Lang>>();
    let word = move |msg: Msg| say(lang(), msg);
    rsx! {
            // The event stream stays, one click down: it is what the
            // Ledger holds, and a reader who wants the raw order should
            // not have to leave the page to see it. Absent when there is
            // nothing behind it, because a control that opens onto an
            // empty list teaches the reader it is not worth opening.
            details { class: "stream", hidden: held == 0,
                summary { "{word(Msg::LiveEveryEvent)}" }
                ol { class: "lines",
                    for line in lines {
                        li {
                            key: "{line.seq.value()}",
                            class: if line.from_person { "line person" } else { "line" },
                            span { class: "seq", "{line.seq.value()}" }
                            span { class: "kind", "{line.kind:?}" }
                            span { class: "text", "{line_text(lang(), &line)}" }
                        }
                    }
                }
            }
            // Which of the two empty states this is turns on whether the
            // page is watching a session, not on how many the picker had
            // to offer: the session page hands that picker an empty list
            // and would otherwise tell a reader looking at a named room
            // that no work has ever been sent anywhere.
            if feed.lines().is_empty() {
                crate::panel::Empty {
                    status: if run.is_none() {
                        word(Msg::LiveNoRunYet).to_owned()
                    } else {
                        word(Msg::LiveNothingSince).to_owned()
                    },
                    what: if run.is_none() {
                        word(Msg::LiveNoRunYetWhat).to_owned()
                    } else {
                        word(Msg::LiveNothingSinceWhat).to_owned()
                    },
                }
            }
    }
}
