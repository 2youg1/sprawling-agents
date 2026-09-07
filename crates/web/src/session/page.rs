// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One session: the four questions and five readings.

use channels::Address;
use dioxus::prelude::*;

use crate::app::Snapshot;
use crate::lang::{Lang, Msg, fill, say};

use super::facts::head_facts;
use super::links::building_of;
use super::tabs::Tab;

/// One session.
#[component]
#[allow(
    clippy::too_many_arguments,
    reason = "one page, one prop per fact it draws"
)]
pub fn SessionView(
    addr: Address,
    snapshot: Snapshot,
    records: Vec<channels::EventRecord>,
    /// What this session changed on disk, once the server has said.
    changes: Option<channels::ChangesAnswer>,
    /// This city's spend, for the one row that belongs to this run.
    cost: Option<channels::CostAnswer>,
    /// The building this room is in, for the documents tab.
    building: Option<channels::BuildingAnswer>,
    /// A line a drop wrote into this session's box, unsent. It stops in
    /// the box: this session is already spending, which is exactly where
    /// a gesture nobody could take back would cost the most.
    steered: Option<String>,
    live: Signal<bool>,
    on_frame: EventHandler<channels::ClientFrame>,
    on_drop: EventHandler<(crate::drop::Target, crate::drop::Dropped)>,
) -> Element {
    let lang = use_context::<Signal<Lang>>();
    let word = move |msg: Msg| say(lang(), msg);
    let mut tab = use_signal(Tab::default);
    // Whether the feed scrolls itself. A fact about reading one session,
    // so it is this page's and does not outlive it.
    let mut following = use_signal(|| true);
    let found = snapshot.session_at(&addr);

    let Some((run, row)) = found else {
        return rsx! {
            crate::panel::Panel {
                title: word(Msg::SessionUnknown).to_owned(),
                scope: None,
                figure: None,
                source: word(Msg::SessionSource).to_owned(),
                crate::panel::Empty {
                    status: word(Msg::SessionUnknown).to_owned(),
                    what: word(Msg::SessionUnknownWhat).to_owned(),
                    a { class: "nav-item", href: "#/", "{word(Msg::SessionAllSessions)}" }
                }
            }
        };
    };
    let facts = head_facts(lang(), row);
    // Folded once, above the tabs, because the tab strip itself has to
    // say when a skill's bytes moved: a warning a person has to open a
    // tab to see arrives after the turn it was about.
    let given = crate::prompt::given(&records, run);
    let phase = row.phase;
    let turns = row.turns;
    let mine: Vec<channels::EventRecord> = records
        .iter()
        .filter(|held| held.run() == run)
        .cloned()
        .collect();

    rsx! {
        header { class: "session-head",
            a { class: "session-back", href: "#/", "{word(Msg::SessionAllSessions)}" }
            h1 { class: "address",
                span {
                    class: "phase {phase.token()}",
                    role: "img",
                    "aria-label": "{say(lang(), phase.word())}",
                }
                span { class: "room", "{addr.as_str()}" }
                span { class: "turn",
                    {fill(word(Msg::SessionTurnOrdinal), &[("n", &turns.to_string())])}
                }
            }
            p { class: "session-facts",
                for fact in facts {
                    span {
                        key: "{fact.said}",
                        class: if fact.known { "fact" } else { "fact unknown" },
                        "{fact.said}"
                    }
                }
            }
            // The sentence a person typed, above everything this run
            // did with it. First on the page because it is the question
            // the rest of the page answers, and because it is the one
            // thing here somebody wrote themselves.
            if let Some(ref asked) = row.task {
                p { class: "session-task", "{asked}" }
            }
            p { class: "panel-scope", "{word(Msg::SessionScope)}" }
            p { class: "panel-scope", "{word(Msg::SessionContextScope)}" }
        }

        nav { class: "session-tabs",
            for one in Tab::ALL {
                button {
                    key: "{one:?}",
                    class: if one == Tab::Prompt && given.disturbed() { "tab alert" } else { "tab" },
                    "aria-current": if tab() == one { "page" } else { "false" },
                    onclick: move |_| tab.set(one),
                    "{word(one.word())}"
                }
            }
        }

        match tab() {
            Tab::Turns => rsx! {
                // What the model is saying right now, if it is saying
                // anything. Above the turns because it is the newest
                // thing, and gone the moment `model_returned` lands: the
                // record below then carries the settled text, which is
                // the text a replay produces.
                if let Some(said) = snapshot.saying(&run) {
                    p { class: "said saying", "{said}" }
                }
                crate::live::LiveView {
                    feed: crate::live::Feed::replay(mine.iter(), Some(run), true),
                    turns: crate::turn::turns(mine.iter()),
                    run: Some(run),
                    runs: Vec::new(),
                    following: following(),
                    steered: steered.clone(),
                    changes: None,
                    live,
                    on_frame,
                    on_follow: move |on| following.set(on),
                    on_drop,
                    on_watch: move |_| {},
                }
            },
            Tab::Changes => rsx! {
                crate::live::Changed { changes: changes.clone() }
            },
            Tab::Cost => rsx! {
                crate::dashboard::CostsView {
                    answer: cost.clone(),
                    usage: snapshot.usage(),
                    spent: row.spent.unwrap_or_default(),
                    live,
                    on_frame,
                }
            },
            Tab::Prompt => rsx! {
                crate::prompt::PromptView { given: given.clone() }
            },
            Tab::Docs => rsx! {
                crate::building_view::BuildingView {
                    addr: building_of(&addr).unwrap_or_else(|| addr.clone()),
                    answer: building.clone(),
                    // The session page asks for one building, never for
                    // the city, so it has no pursuit list to hand on. An
                    // empty one draws the "set a goal" form, which is the
                    // honest thing to offer where nothing is known.
                    pursuits: Vec::new(),
                    inbox: None,
                    signals: snapshot.signals_seen(),
                    live,
                    on_frame,
                    on_select: move |_| {},
                    on_drop,
                }
            },
        }
    }
}
