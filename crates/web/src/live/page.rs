// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Watching one session as it happens.

use crate::lang::{Msg, fill, say};
use channels::{ClientFrame, RunId};
use dioxus::prelude::*;

use super::commands::{named, run_id_line};
use super::composer::Composer;
use super::describe::Changed;
use super::feed::Feed;
use super::rounds::Rounds;
use super::stream::Stream;

/// One session as it happens, plus the two things a person does while
/// watching: say something into it, or stop it.
#[component]
pub fn LiveView(
    feed: Feed,
    /// The rounds the server folded for this session, when it has been
    /// asked and has answered. `None` is "not asked yet", which is a
    /// different thing from "this session has no rounds".
    ///
    /// The fold is the server's since card-6.5: a second client must be
    /// able to draw a session without reimplementing it, so it lives
    /// where the wire can reach it and not in this crate.
    rounds: Option<channels::RoundsAnswer>,
    run: Option<RunId>,
    /// Every run the client knows of, newest first, with the word the
    /// page shows for its phase.
    runs: Vec<(RunId, String)>,
    following: bool,
    /// A line a drop wrote into this session's box. It stops in the box:
    /// a run is already spending, which is exactly where a gesture
    /// nobody could take back would cost the most.
    steered: Option<String>,
    /// What this session has changed on disk, when the server has been
    /// asked and answered. `None` is "not asked yet", which is a
    /// different thing from "changed nothing".
    changes: Option<channels::ChangesAnswer>,
    /// Whether the socket is live; see `app::Root`. A page that asked
    /// before the handshake finished asked nobody.
    live: Signal<bool>,
    on_frame: EventHandler<ClientFrame>,
    on_follow: EventHandler<bool>,
    on_drop: EventHandler<(crate::drop::Target, crate::drop::Dropped)>,
    on_watch: EventHandler<Option<RunId>>,
) -> Element {
    let lang = use_context::<Signal<crate::lang::Lang>>();
    let word = move |msg: Msg| say(lang(), msg);
    // What happened in this session before this tab opened.
    //
    // The stream carries what happens next, and the city-wide backfill is
    // one bounded slice divided between every session in flight - so a
    // session older than that slice was simply not in it, and this page
    // rendered blank for it. Asked once per session, the same way every
    // other page asks its own question when it opens.
    let asked = use_signal(|| None::<RunId>);
    use_effect(use_reactive!(|(run, live)| {
        let mut asked = asked;
        if live()
            && let Some(id) = run
            && asked() != Some(id)
        {
            asked.set(Some(id));
            on_frame.call(ClientFrame::Query(channels::Query::RunHistory {
                run: id,
                before: None,
                limit: channels::HISTORY_MAX,
            }));
            // The same records, folded. Asked in the same breath as the
            // slice they come from, so a page never shows a stream with
            // no rounds beside it.
            on_frame.call(ClientFrame::Query(channels::Query::Rounds { run: id }));
        }
    }));
    // What this session has changed on disk.
    //
    // Measured from the session's first fence to the working tree, so a
    // wave still running counts: the tree is what a person is looking
    // at, not the tree git last recorded. Asked once per base, because
    // the base does not move while a session is open.
    let turns: Vec<channels::Turn> = rounds
        .as_ref()
        .map(|held| held.turns.clone())
        .unwrap_or_default();
    let opened = rounds.as_ref().and_then(|held| held.opened_at);
    let fenced = use_signal(|| None::<channels::GitOid>);
    use_effect(use_reactive!(|(opened, live)| {
        let mut fenced = fenced;
        if live()
            && let Some(base) = opened
            && fenced() != Some(base)
        {
            fenced.set(Some(base));
            on_frame.call(ClientFrame::Query(channels::Query::Changes {
                base,
                head: None,
            }));
        }
    }));
    let steer = use_signal(String::new);
    // Reactive for the same reason the dispatch bar's is: a line written
    // by a drop after the first render would otherwise never arrive.
    let written = steered.clone();
    use_effect(use_reactive!(|written| {
        let mut steer = steer;
        if let Some(ref line) = written
            && !line.is_empty()
        {
            steer.set(line.clone());
        }
    }));
    // Whether a drag is over this session's box. A hover rule cannot say
    // so: device input events are suppressed for the whole of a drag.
    let over = use_signal(|| false);
    let lines = feed.lines().to_vec();
    let dropped = feed.dropped();
    let held = lines.len();
    let last_seq = lines.last().map(|line| line.seq);
    let dropped_line = fill(word(Msg::LiveDropped), &[("dropped", &dropped.to_string())]);
    rsx! {
        section { class: "live",
            crate::panel::Panel {
                // Never a claim about the city: this window opens when the
                // page connects, so "no run has been dispatched here" is a
                // sentence it has no standing to say. The overview reads
                // the city's own count for that.
                // The title reads the same count the figure does, so a
                // panel cannot contradict its own body. It used to read
                // how many runs the picker below had to offer, and the
                // session page passes that picker an empty list on
                // purpose - one session needs no chooser - so that page
                // announced "nothing has happened since this page
                // connected" above eight turns and a figure of 57. The
                // live page had the same defect from the other side: a
                // window holding only city events names no run, and it
                // said nothing had happened while showing them.
                title: match (held, run) {
                    (0, _) => word(Msg::LiveNothingSinceConnected).to_owned(),
                    (_, Some(_)) => word(Msg::LiveOneSession).to_owned(),
                    (_, None) => word(Msg::LiveEveryRun).to_owned(),
                },
                figure: (held > 0).then(|| held.to_string()),
                scope: word(Msg::LiveScope).to_owned(),
                source: word(Msg::LiveSource).to_owned(),
            // Which session is being watched is a choice, not a guess.
            // With two runs in flight, "the latest one" is a coin toss,
            // and the page was showing one of them without saying so.
            div { class: "runs",
                button {
                    "aria-current": if run.is_none() { "true" } else { "false" },
                    onclick: move |_| on_watch.call(None),
                    "{word(Msg::LiveEverything)}"
                }
                for (id, said) in runs.clone() {
                    button {
                        key: "{id}",
                        "aria-current": if run == Some(id) { "true" } else { "false" },
                        onclick: move |_| on_watch.call(Some(id)),
                        "{said}"
                    }
                }
            }
            header { class: "live-head",
                match run {
                    // The session first, because that is what the person
                    // called it; the run identifier stays on the page
                    // because it is what the Ledger and `sprawling fork`
                    // are addressed by.
                    Some(id) => rsx! {
                        span { class: "run", "{named(&runs, id)}" }
                        span { class: "run-id", "{run_id_line(lang(), id)}" }
                    },
                    None => rsx! { span { class: "run", "{word(Msg::LiveEveryRun)}" } },
                }
                label { class: "follow",
                    input {
                        r#type: "checkbox",
                        checked: following,
                        onchange: move |event| on_follow.call(event.checked()),
                    }
                    "{word(Msg::LiveFollowEnd)}"
                }
            }
            if dropped > 0 {
                p { class: "dropped", "{dropped_line}" }
            }
            // What this session did to the disk, which is the fastest way
            // to see what an agent has been doing. The counts come from
            // git between two real commits; nothing here is folded from
            // an event, because the fence is the authority on what moved.
            Changed { changes: changes.clone() }
            Rounds { turns: turns }
            Stream { feed: feed, lines: lines, held: held, run: run }
            Composer { run: run, last_seq: last_seq, steer: steer, over: over, on_frame: on_frame, on_drop: on_drop }
            }
        }
    }
}
