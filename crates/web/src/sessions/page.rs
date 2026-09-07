// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The first screen.

use dioxus::prelude::*;

use super::composer::Composer;
use super::listing::listing;
use super::plan::{Field, Plan, rung};
use super::tables::Tables;
use crate::app::Snapshot;
use crate::route::View;

/// The first screen.
#[component]
#[allow(
    clippy::too_many_arguments,
    reason = "one page, one prop per fact it draws"
)]
pub fn SessionsView(
    snapshot: Snapshot,
    /// What the city answered about itself, for the buildings a room can
    /// be completed against.
    city: Option<channels::CityAnswer>,
    /// What this machine has attached, for the first rung of the ladder.
    endpoints: Option<channels::EndpointsAnswer>,
    /// The depth this city thinks with when nobody says otherwise.
    effort: String,
    /// A task line a drop wrote. It fills the box a person would have
    /// typed into, so one gesture both aims and describes.
    dropped: Option<String>,
    live: Signal<bool>,
    on_frame: EventHandler<channels::ClientFrame>,
    on_view: EventHandler<View>,
    on_drop: EventHandler<(crate::drop::Target, crate::drop::Dropped)>,
) -> Element {
    let mut task = use_signal(String::new);
    let plan = use_signal(|| Plan::guessed(&snapshot, &effort));
    let mut opened: Signal<Option<Field>> = use_signal(|| None);

    // A drop writes the task and nothing else. It arrives as a prop
    // rather than as a signal because the gesture is answered once, in
    // `web::drop`, and a second reading of it here would be a second
    // answer to what a drag means.
    use_effect(move || {
        if let Some(written) = dropped.clone() {
            task.set(written);
        }
    });

    let (in_flight, ended) = listing(&snapshot);
    let standing = rung(endpoints.as_ref(), city.as_ref(), &snapshot);
    let rooms: Vec<String> = city
        .as_ref()
        .map(|answer| {
            answer
                .buildings
                .iter()
                .map(|raised| raised.addr.as_str().to_owned())
                .collect()
        })
        .unwrap_or_default();
    // The frame this box would send, built once from the two signals.
    // `dispatch_command` is the only place in the client that builds a
    // Dispatch, so asking it whether one can be built is the same
    // question as asking whether the button may be pressed - and the
    // button cannot go out of step with what pressing it would do.
    let intended = {
        let held = plan.read().clone();
        crate::command::dispatch_command(
            crate::command::Sending {
                room: &held.room,
                mode: &held.mode,
                effort: crate::command::effort_named(&held.effort),
            },
            &task.read(),
            "",
        )
    };
    let sendable = intended.is_some();
    let send = use_callback(move |_: ()| {
        let held = plan.read().clone();
        let written = task.read().clone();
        if let Some(frame) = crate::command::dispatch_command(
            crate::command::Sending {
                room: &held.room,
                mode: &held.mode,
                effort: crate::command::effort_named(&held.effort),
            },
            &written,
            "",
        ) {
            on_frame.call(frame);
            task.set(String::new());
            opened.set(None);
        }
    });

    rsx! {
        Composer { task: task, plan: plan, opened: opened, standing: standing, rooms: rooms.clone(), sendable: sendable, send: send, snapshot: snapshot.clone(), on_drop: on_drop }
        Tables { in_flight: in_flight, ended: ended, city: city.clone(), snapshot: snapshot.clone(), busy: crate::shell::busy_buildings(&snapshot), live: live, on_frame: on_frame, on_view: on_view }
    }
}
