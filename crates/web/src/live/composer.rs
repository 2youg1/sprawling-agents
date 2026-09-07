// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The steer box and the interventions: speaking and stopping.

use crate::lang::{Msg, say};
use channels::{ClientFrame, RunId, Seq};
use dioxus::prelude::*;

use super::commands::{cancel_command, fork_command, fork_line, steer_command};

/// The steer box and the interventions: speaking and stopping.
#[component]
pub fn Composer(
    run: Option<RunId>,
    last_seq: Option<Seq>,
    steer: Signal<String>,
    over: Signal<bool>,
    on_frame: EventHandler<ClientFrame>,
    on_drop: EventHandler<(crate::drop::Target, crate::drop::Dropped)>,
) -> Element {
    let lang = use_context::<Signal<crate::lang::Lang>>();
    let word = move |msg: Msg| say(lang(), msg);
    rsx! {
            // Speaking into a run needs a run. With none chosen this was
            // an input box beside a button that could never fire, which
            // reads as a broken control rather than as a missing choice.
            match run {
                None => rsx! {
                    p { class: "note",
                        "{word(Msg::LivePickASession)}"
                    }
                },
                Some(id) => rsx! {
                    form {
                        class: if over() { "steer drop-zone over" } else { "steer drop-zone" },
                        title: "{word(Msg::DropHere)}",
                        // Section 8-38 refused a drop on a run because no
                        // meaning could be carried out. This box is that
                        // meaning, and the principle it protected holds:
                        // the line is written, the button is not pressed.
                        ondragover: move |event| event.prevent_default(),
                        ondragenter: move |event| {
                            event.prevent_default();
                            over.set(true);
                        },
                        ondragleave: move |_| over.set(false),
                        ondrop: move |event: Event<DragData>| {
                            event.prevent_default();
                            over.set(false);
                            on_drop.call((
                                crate::drop::Target::Run(id),
                                crate::drop::from_event(&event),
                            ));
                        },
                        onsubmit: move |event| {
                            event.prevent_default();
                            let said = steer.read().trim().to_owned();
                            if said.is_empty() {
                                return;
                            }
                            on_frame.call(steer_command(id, &said));
                            steer.set(String::new());
                        },
                        input {
                            name: "steer",
                            placeholder: "{word(Msg::LiveSteerPlaceholder)}",
                            value: "{steer}",
                            oninput: move |event| steer.set(event.value()),
                        }
                        button {
                            r#type: "submit",
                            disabled: steer.read().trim().is_empty(),
                            "{word(Msg::LiveSteerSend)}"
                        }
                    }
                },
            }
            // The rest of what a person may do to a run. Each says what it
            // makes rather than how it feels, and none of them acts on a
            // guess about which run was meant.
            if let Some(id) = run {
                div { class: "interventions",
                    // Stopping this session. Quiet and away from the box
                    // a person types into: it cannot be taken back, and
                    // the one control that cannot be taken back must not
                    // sit where a hand is already moving fast.
                    button {
                        class: "quiet",
                        onclick: move |_| on_frame.call(cancel_command(id)),
                        "{word(Msg::CancelLastRun)}"
                    }
                    button {
                        class: "quiet",
                        disabled: last_seq.is_none(),
                        onclick: move |_| {
                            if let Some(at) = last_seq {
                                on_frame.call(fork_command(id, at));
                            }
                        },
                        match last_seq {
                            Some(at) => rsx! { "{fork_line(lang(), at)}" },
                            None => rsx! { "{word(Msg::LiveNothingToBranch)}" },
                        }
                    }
                    p { class: "note",
                        "{word(Msg::LiveInterventionNote)}"
                    }
                }
            }
    }
}
