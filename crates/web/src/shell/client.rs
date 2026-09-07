// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The client: the live snapshot and mounting it.

use dioxus::prelude::*;

use channels::EventRecord;

use super::nav::reachable;
use super::root::Root;
use crate::app::Snapshot;
use crate::asking::room_asked_for;
use crate::lang::Msg;
#[cfg(not(target_arch = "wasm32"))]
use crate::mount::Outbound;
use crate::mount::{Keyboard, follow_the_address_bar, listen_for_keys};
#[cfg(target_arch = "wasm32")]
use crate::mount::{Wiring, connect};
use crate::route::View;

/// The live client: it holds the snapshot the stream folds into, and
/// renders [`Root`] against it. Every judgement about the connection
/// belongs to `socket::Link`; every judgement about what an event means
/// belongs to `Snapshot::apply`. This component only holds the two
/// together and decides nothing itself.
#[component]
pub fn App() -> Element {
    // Provided before anything renders, because every component below
    // reads it. The first value is the browser's own setting: a person
    // whose machine is in Chinese should not have to find a switch to
    // be spoken to in Chinese.
    use_context_provider(|| Signal::new(crate::lang::preferred()));
    // Read back rather than kept from the line above: the signal is the
    // one authority for what language this page reads in, and the
    // listeners below say their words when they fire, not when they are
    // registered - so a person who switches language mid-session is
    // answered in the new one.
    let lang = use_context::<Signal<crate::lang::Lang>>();
    let snapshot = use_signal(Snapshot::new);
    #[cfg_attr(
        target_arch = "wasm32",
        expect(
            unused_mut,
            reason = "in a browser the address bar moves the signal, not this handle"
        )
    )]
    let mut view = use_signal(View::default);
    // The address bar is the authority for which page is showing, and
    // the listener below is the only thing that moves the signal. A
    // click writes the fragment and hears its own change back, so a
    // click and the browser's back button travel the same path and
    // cannot disagree about where the person is.
    let endpoints = use_signal(|| None::<channels::EndpointsAnswer>);
    let city = use_signal(|| None::<channels::CityAnswer>);
    let cost = use_signal(|| None::<channels::CostAnswer>);
    let building = use_signal(|| None::<channels::BuildingAnswer>);
    let discards = use_signal(|| None::<channels::DiscardAnswer>);
    let inbox = use_signal(|| None::<channels::InboxAnswer>);
    let hits = use_signal(|| None::<channels::ArchiveAnswer>);
    let filed = use_signal(|| None::<channels::RegistryAnswer>);
    let records = use_signal(Vec::<EventRecord>::new);
    let mut refused = use_signal(|| None::<crate::alert::Refused>);
    // The address bar is the authority for which page is showing, and the
    // listener below is the only thing that moves the signal, so a click
    // and the browser's back button travel one path and cannot disagree
    // about where the person is. A fragment this build cannot resolve
    // becomes a refusal rather than a silent landing on the first page.
    follow_the_address_bar(view, refused, lang);
    let mut selected = use_signal(|| None::<String>);
    let mut dropped = use_signal(|| None::<String>);
    // A line a drop wrote into the session's box, held here for the same
    // reason `dropped` is: the box belongs to a view that a drop can
    // reach from outside it.
    // What the open session changed on disk. An answer, so it is held
    // beside the others and a reload asks again rather than trusting it.
    let changes = use_signal(|| None::<channels::ChangesAnswer>);
    let live = use_signal(|| false);
    // What the keyboard opened. Held here rather than inside `Root`
    // because the listener that sets them is registered once for the
    // window, and a page redraw must not take a reader's palette away.
    let mut palette = use_signal(|| false);
    let mut keymap = use_signal(|| false);
    listen_for_keys(Keyboard {
        chord: use_signal(crate::keys::Chord::default),
        palette,
        keymap,
        view,
        refused,
    });
    // The room the last dispatch asked for, so its run can be opened
    // when it starts rather than left for the person to find among the
    // others.
    // What the record page's ledger lens states about the whole history.
    let vitals = use_signal(|| None::<channels::MetricsAnswer>);
    // A line a drop wrote into an open session's box, on its way there.
    let mut steered = use_signal(|| None::<String>);
    let mut expecting = use_signal(|| None::<String>);
    #[cfg(target_arch = "wasm32")]
    let outbound = connect(Wiring {
        snapshot,
        endpoints,
        city,
        cost,
        building,
        discards,
        inbox,
        hits,
        filed,
        vitals,
        changes,
        records,
        live,
        view,
        expecting,
        refused,
        lang,
    });
    #[cfg(not(target_arch = "wasm32"))]
    let outbound = Outbound;
    rsx! {
        Root {
            snapshot: snapshot(),
            view: view(),
            endpoints: endpoints(),
            city: city(),
            cost: cost(),
            building: building(),
            discards: discards(),
            inbox: inbox(),
            hits: hits(),
            filed: filed(),
            refused: refused(),
            records: records(),
            selected: selected(),
            dropped: dropped(),
            steered: steered(),
            vitals: vitals(),
            changes: changes(),
            live,
            on_frame: move |frame: channels::ClientFrame| {
                if let Some(room) = room_asked_for(&frame) {
                    expecting.set(Some(room));
                }
                outbound.call(frame);
            },
            on_select: move |id| selected.set(id),
            // One place answers what a drag meant, whichever zone it
            // landed on. A refusal takes the same route every other
            // refusal takes, so a gesture with no meaning reads like
            // everything else the city would not do.
            on_drop: move |(target, what): (crate::drop::Target, crate::drop::Dropped)| {
                match crate::drop::read(&target, &what) {
                    crate::drop::Meaning::Aim { addr, task } => {
                        selected.set(Some(addr.as_str().to_owned()));
                        dropped.set(Some(task));
                    }
                    // The bar already knows where the work goes, because
                    // somebody put it there. Only the task is written.
                    crate::drop::Meaning::Task { task } => {
                        dropped.set(Some(task));
                    }
                    // Into the session's own box, unsent. The button is
                    // still the person's to press.
                    crate::drop::Meaning::Say { said, .. } => {
                        steered.set(Some(said));
                    }
                    crate::drop::Meaning::Refused { because } => {
                        refused.set(Some(crate::alert::refused(
                            lang(),
                            &crate::drop::refusal(lang(), because),
                        )));
                    }
                }
            },
            on_view: move |next: View| {
                #[cfg(target_arch = "wasm32")]
                crate::route::go(&next);
                #[cfg(not(target_arch = "wasm32"))]
                view.set(next);
            },

            on_dismiss: move |()| refused.set(None),
        }
        if palette() {
            crate::palette::Palette {
                offers: reachable(&snapshot(), city().as_ref(), lang()),
                on_go: move |going: View| {
                    palette.set(false);
                    #[cfg(target_arch = "wasm32")]
                    crate::route::go(&going);
                    #[cfg(not(target_arch = "wasm32"))]
                    view.set(going);
                },
                on_close: move |()| palette.set(false),
            }
        }
        if keymap() {
            KeyMap { on_close: move |()| keymap.set(false) }
        }
    }
}

/// The key map, shown by the key that is hardest to guess.
///
/// A product whose shortcuts are undocumented has no shortcuts: nobody
/// tries a chord they have not been told about. This is the one page in
/// the client that exists to be read once.
#[component]
fn KeyMap(on_close: EventHandler<()>) -> Element {
    let lang = use_context::<Signal<crate::lang::Lang>>();
    let word = move |msg: Msg| crate::lang::say(lang(), msg);
    let rows = [
        ("Ctrl / \u{2318} + K", Msg::KeysPalette),
        ("Ctrl / \u{2318} + \u{21b5}", Msg::KeysCompose),
        ("Esc", Msg::KeysDismiss),
        ("g", Msg::KeysGo),
        ("?", Msg::KeysShow),
    ];
    rsx! {
        div { class: "palette-scrim", onclick: move |_| on_close.call(()),
            div {
                class: "keymap",
                onclick: move |event| event.stop_propagation(),
                h2 { "{word(Msg::KeysTitle)}" }
                p { class: "note", "{word(Msg::KeysScope)}" }
                dl {
                    for (chord, said) in rows {
                        div { key: "{chord}", class: "keymap-row",
                            dt { class: "chord", "{chord}" }
                            dd { "{word(said)}" }
                        }
                    }
                }
            }
        }
    }
}
