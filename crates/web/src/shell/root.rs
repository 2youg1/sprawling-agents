// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The root: three regions, and nothing that decides anything.

use channels::EventRecord;
use dioxus::prelude::*;

use crate::app::{ProviderHealth, Snapshot};

use crate::command::DEFAULT_EFFORT;
#[cfg(not(target_arch = "wasm32"))]
#[cfg(target_arch = "wasm32")]
use crate::mount::{Wiring, connect};
use crate::readout::{page_named, standing_of};
use crate::route::View;
use crate::route::{destinations, showing};

/// The root: three regions, and nothing that decides anything.
///
/// Business state is the server's; this reads a snapshot handed to it.
///
/// **Five regions became three.** The right-hand column carried a
/// provider status that read "normal" almost always, and a steady
/// "everything is fine" is the absence of a problem rather than a fact:
/// it never changed anybody's next action, so it does not stay on
/// screen. The three counts it also held do change the next action, so
/// they moved into the top bar. The footer held a dispatch bar, which
/// now stands at the top of the table its rows land in.
#[component]
pub fn Root(
    snapshot: Snapshot,
    view: View,
    endpoints: Option<channels::EndpointsAnswer>,
    city: Option<channels::CityAnswer>,
    cost: Option<channels::CostAnswer>,
    building: Option<channels::BuildingAnswer>,
    discards: Option<channels::DiscardAnswer>,
    inbox: Option<channels::InboxAnswer>,
    hits: Option<channels::ArchiveAnswer>,
    filed: Option<channels::RegistryAnswer>,
    /// What the city last refused this person, if anything. Cleared by
    /// the person, never by the passage of time: an answer that fades
    /// before it is read is an answer nobody gave.
    refused: Option<crate::alert::Refused>,
    records: Vec<EventRecord>,
    selected: Option<String>,
    /// A task line a drop wrote, on its way to the composer.
    dropped: Option<String>,
    /// A line a drop wrote into an open session's box. Separate from
    /// `dropped` because the two boxes take different gestures: aiming
    /// new work, and saying something into work already running.
    steered: Option<String>,
    /// What this city has written down, counted. Read by the record
    /// page, which is the page those counts are about.
    vitals: Option<channels::MetricsAnswer>,
    /// What the open session changed on disk, once the server has said.
    changes: Option<channels::ChangesAnswer>,
    rounds: Option<channels::RoundsAnswer>,
    /// Whether frames are flowing yet.
    ///
    /// A page asks its question when it mounts, and the first mount
    /// happens before the socket has finished its handshake - a frame
    /// sent then is dropped, by design, because a queue would be a second
    /// place where "what did the person ask for" lives. So the pages
    /// watch this instead and ask again the moment there is somebody to
    /// ask. Without it the first page a person sees says "asking the city
    /// what it holds" forever.
    live: Signal<bool>,
    on_frame: EventHandler<channels::ClientFrame>,
    on_select: EventHandler<Option<String>>,
    /// Where a gesture goes. One handler for every drop zone, so what a
    /// drag means is answered once.
    on_drop: EventHandler<(crate::drop::Target, crate::drop::Dropped)>,
    on_view: EventHandler<View>,
    on_dismiss: EventHandler<()>,
) -> Element {
    // The language every word on this page is said in. One signal for
    // the whole tree rather than a prop through twenty components: what
    // a person reads in is a fact about the page, not about a panel.
    let lang = use_context::<Signal<crate::lang::Lang>>();
    let word = move |msg: crate::lang::Msg| crate::lang::say(lang(), msg);
    let spots = destinations(&snapshot);
    let counts = crate::sessions::counts_said(lang(), &snapshot);
    let unwell = !matches!(snapshot.provider(), ProviderHealth::Healthy);

    // An old link naming a run is resolved here, where the room is
    // known, and the address bar is rewritten to the name a person can
    // read. The router stays pure; the redirect happens once, in the one
    // place that holds the fact it needs.
    let resolving = view.clone();
    let resolved = crate::session::room_for_link(&snapshot, &resolving);
    use_effect(use_reactive!(|(resolved,)| {
        if let Some(landed) = resolved.clone() {
            on_view.call(landed);
        }
    }));

    rsx! {
        main { class: "layout",
            header { class: "top-bar",
                span { class: "address", "{page_named(lang(), &view)}" }
                // Only when it is not normal. A steady "provider: fine"
                // is a problem's absence, and an absence that occupies a
                // permanent line teaches a reader to stop reading it.
                if unwell {
                    span { class: "unwell",
                        if matches!(snapshot.provider(), ProviderHealth::Unknown) {
                            "{word(crate::lang::Msg::CityUnwell)}"
                        } else {
                            "{word(snapshot.provider().word())}"
                        }
                    }
                }
                if let Some(told) = refused.clone() {
                    div { class: "refusal", role: "alert",
                        span { class: "refusal-code", "{told.code}" }
                        span { class: "refusal-what", "{told.what}" }
                        span { class: "refusal-way", "{told.recovery}" }
                        button {
                            class: "refusal-close",
                            "aria-label": "{word(crate::lang::Msg::AlertDismiss)}",
                            onclick: move |_| on_dismiss.call(()),
                            "\u{00d7}"
                        }
                    }
                }
                span { class: "counts",
                    for said in counts {
                        span { key: "{said}", "{said}" }
                    }
                }
            }
            nav { class: "left-nav",
                // Anchors, not buttons. Writing the fragment is the only
                // way a view changes, so an `<a href>` is already a whole
                // navigation - and it arrives with the keyboard, the
                // middle click, "copy link address" and the link role a
                // screen reader announces, none of which a button with an
                // onclick would have had.
                div { class: "nav-group",
                    for spot in spots {
                        a {
                            key: "{spot.label:?}",
                            class: "nav-item",
                            href: "{crate::route::to_fragment(&spot.view)}",
                            "aria-current": if showing(&spot.view, &view) { "page" } else { "false" },
                            "{word(spot.label)}"
                            if let Some(waiting) = spot.waiting {
                                span { class: "badge", "{waiting}" }
                            }
                        }
                    }
                }
                // What this whole city is doing, at the foot of the
                // column that names its parts. Stopping it left the top
                // bar because it stood beside the send button, which is
                // the one place a person's hand is already moving fast.
                div { class: "city-state",
                    p { class: "standing", "{word(standing_of(&snapshot))}" }
                    button {
                        class: "quiet",
                        r#type: "button",
                        onclick: move |_| on_frame.call(crate::command::halt(!snapshot.is_halted())),
                        if snapshot.is_halted() {
                            "{word(crate::lang::Msg::ReleaseCity)}"
                        } else {
                            "{word(crate::lang::Msg::HaltCity)}"
                        }
                    }
                }
            }
            section { class: "centre",
                match view {
                    // A run named by an old link, while the fold that
                    // says which room it is in has not arrived. Said
                    // rather than left blank: the link is not broken, the
                    // answer is not here yet.
                    View::Run(_) => rsx! {
                        crate::panel::Panel {
                            title: word(crate::lang::Msg::AskingWhatItHolds).to_owned(),
                            scope: None,
                            figure: None,
                            source: word(crate::lang::Msg::SessionSource).to_owned(),
                        }
                    },
                    View::Sessions => rsx! {
                        crate::sessions::SessionsView {
                            snapshot: snapshot.clone(),
                            city: city.clone(),
                            endpoints: endpoints.clone(),
                            effort: DEFAULT_EFFORT.to_owned(),
                            dropped: dropped.clone(),
                            live,
                            on_frame,
                            on_view,
                            on_drop,
                        }
                    },
                    View::Session(ref addr) => rsx! {
                        crate::session::SessionView {
                            addr: addr.clone(),
                            snapshot: snapshot.clone(),
                            records: records.clone(),
                            changes: changes.clone(),
                            rounds: rounds.clone(),
                            cost: cost.clone(),
                            building: building.clone(),
                            steered: steered.clone(),
                            live,
                            on_frame,
                            on_drop,
                        }
                    },
                    View::Waiting => rsx! {
                        crate::waiting::WaitingView {
                            snapshot: snapshot.clone(),
                            live,
                            on_frame,
                        }
                    },
                    View::Record(lens) => rsx! {
                        crate::record::RecordView {
                            lens,
                            records: records.clone(),
                            hits: hits.clone(),
                            filed: filed.clone(),
                            discards: discards.clone(),
                            vitals: vitals.clone(),
                            live,
                            on_frame,
                        }
                    },
                    View::Cost => rsx! {
                        crate::dashboard::CostsView {
                            answer: cost.clone(),
                            usage: snapshot.usage(),
                            spent: snapshot.spent(),
                            live,
                            on_frame,
                        }
                    },
                    View::Setup => rsx! {
                        crate::settings::Settings {
                            answer: endpoints.clone(),
                            login_url: snapshot.login_url().map(str::to_owned),
                            served: snapshot.served().clone(),
                            live,
                            on_frame,
                        }
                    },
                    View::Building(ref addr) => rsx! {
                        crate::building_view::BuildingView {
                            addr: addr.clone(),
                            answer: building.clone(),
                            pursuits: crate::command::pursuits_of(city.as_ref()),
                            inbox: inbox.clone(),
                            signals: snapshot.signals_seen(),
                            live,
                            on_frame,
                            on_select,
                            on_drop,
                        }
                    },
                }
            }
        }
    }
}
