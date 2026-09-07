// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The composer: the ladder for first runs and the box that sends work.

use dioxus::prelude::*;

use super::plan::{Field, Plan, Rung};
use crate::app::Snapshot;
use crate::lang::{Lang, Msg, around, fill, say};

/// The composer.
#[component]
pub fn Composer(
    task: Signal<String>,
    plan: Signal<Plan>,
    opened: Signal<Option<Field>>,
    standing: Rung,
    rooms: Vec<String>,
    sendable: bool,
    send: Callback<(), ()>,
    snapshot: Snapshot,
    on_drop: EventHandler<(crate::drop::Target, crate::drop::Dropped)>,
) -> Element {
    let lang = use_context::<Signal<Lang>>();
    let word = move |msg: Msg| say(lang(), msg);
    rsx! {
        // The ladder. Nothing here stores progress, so attaching a
        // provider outside this window makes the first rung disappear by
        // itself. Each rung offers one action, because a person handed
        // two problems at once solves neither.
        if standing == Rung::NoModel {
            crate::panel::Panel {
                title: word(Msg::FirstNoModelTitle).to_owned(),
                scope: Some(word(Msg::FirstNoModelScope).to_owned()),
                figure: None,
                source: word(Msg::FirstNoModelSource).to_owned(),
                crate::panel::Empty {
                    status: fill(word(Msg::FirstNoModelStatus), &[("tag", "main")]),
                    what: word(Msg::FirstNoModelWhat).to_owned(),
                    a { class: "nav-item", href: "#/setup", "{word(Msg::FirstNoModelWay)}" }
                    a { class: "nav-item", href: "#/setup",
                        "{word(Msg::FirstNoModelSubscription)}"
                    }
                }
            }
        }
        if standing == Rung::NoBuilding {
            crate::panel::Panel {
                title: word(Msg::FirstNoBuildingTitle).to_owned(),
                scope: Some(word(Msg::FirstNoBuildingScope).to_owned()),
                figure: None,
                source: word(Msg::FirstNoBuildingSource).to_owned(),
                crate::panel::Empty {
                    status: word(Msg::FirstNoBuildingStatus).to_owned(),
                    what: fill(word(Msg::FirstNoBuildingWhat), &[("example", "lab")]),
                    a { class: "nav-item", href: "#/", "{word(Msg::FirstNoBuildingWay)}" }
                }
            }
        }
        form {
            class: "panel composer",
            onsubmit: move |event| {
                event.prevent_default();
                send.call(());
            },
            div { class: "panel-head",
                h2 { class: "panel-title",
                    if standing == Rung::NeverSent {
                        "{word(Msg::FirstDispatchTitle)}"
                    } else {
                        "{word(Msg::ComposerTitle)}"
                    }
                }
            }
            p { class: "panel-scope",
                if standing == Rung::NeverSent {
                    "{word(Msg::FirstDispatchScope)}"
                } else {
                    "{word(Msg::ComposerScope)}"
                }
            }
            div { class: "panel-body",
                textarea {
                    class: "composer-task",
                    placeholder: "{word(Msg::ComposerExample)}",
                    rows: "2",
                    value: "{task}",
                    ondragover: move |event| event.prevent_default(),
                    ondrop: move |event| {
                        event.prevent_default();
                        on_drop
                            .call((
                                crate::drop::Target::Composer,
                                crate::drop::from_event(&event),
                            ));
                    },
                    oninput: move |event| task.set(event.value()),
                    // Enter sends and Shift+Enter makes a line, which is
                    // the shape every message box a person has used takes.
                    // Said beside the button as well: a keystroke nobody
                    // is told about is a keystroke nobody presses.
                    onkeydown: move |event| {
                        if event.key() == Key::Enter && !event.modifiers().shift() {
                            event.prevent_default();
                            send.call(());
                        }
                    },
                }
                p { class: "composer-plan",
                    for field in Field::ALL {
                        Decision {
                            key: "{field:?}",
                            field,
                            plan: plan.read().clone(),
                            opened,
                            rooms: rooms.clone(),
                            on_choose: move |value: String| {
                                plan.write().choose(field, value);
                                opened.set(None);
                            },
                        }
                    }
                    button {
                        class: "send",
                        r#type: "submit",
                        disabled: !sendable,
                        "{word(Msg::DispatchSend)}"
                    }
                }
                p { class: "composer-key",
                    if standing == Rung::NeverSent {
                        "{word(Msg::FirstDispatchKeys)}"
                    } else {
                        "{word(Msg::ComposerKeys)}"
                    }
                }
            }
            p { class: "panel-source",
                if standing == Rung::NeverSent {
                    "{word(Msg::FirstDispatchSource)}"
                } else {
                    "{word(Msg::ComposerSource)}"
                }
            }
        }
    }
}

/// One word of the inferred sentence, and the control it opens into.
///
/// Opening replaces the word in place rather than revealing a panel: the
/// sentence keeps its shape, so a person who opens a word by accident
/// has not lost the page they were reading.
#[component]
pub(super) fn Decision(
    field: Field,
    plan: Plan,
    opened: Signal<Option<Field>>,
    rooms: Vec<String>,
    on_choose: EventHandler<String>,
) -> Element {
    let lang = use_context::<Signal<Lang>>();
    let word = move |msg: Msg| say(lang(), msg);
    let (before, after) = around(word(field.sentence()), field.slot());
    let value = plan.value(field).to_owned();
    let ink = plan.ink(field);
    // A room this city has never seen is the one field a person must be
    // able to type into; the rest are closed sets, and a closed set is
    // offered rather than typed.
    let choices: Vec<(String, String)> = match field {
        Field::Room => Vec::new(),
        Field::Mode => crate::command::MODES
            .iter()
            .map(|tag| ((*tag).to_owned(), (*tag).to_owned()))
            .collect(),
        Field::Effort => crate::command::EFFORTS
            .iter()
            .skip(1)
            .map(|(tag, msg)| ((*tag).to_owned(), say(lang(), *msg).to_owned()))
            .collect(),
    };

    rsx! {
        if opened() == Some(field) {
            label { class: "composer-open",
                span { class: "label", "{word(field.label())}" }
                if field == Field::Room {
                    input {
                        class: "composer-field",
                        list: "composer-rooms",
                        value: "{value}",
                        autofocus: true,
                        onchange: move |event| on_choose.call(event.value()),
                    }
                    datalist { id: "composer-rooms",
                        for room in rooms.clone() {
                            option { key: "{room}", value: "{room}" }
                        }
                    }
                } else {
                    select {
                        class: "composer-field",
                        value: "{value}",
                        onchange: move |event| on_choose.call(event.value()),
                        for (tag , shown) in choices.clone() {
                            option { key: "{tag}", value: "{tag}", "{shown}" }
                        }
                    }
                }
            }
        } else {
            button {
                class: "{ink}",
                r#type: "button",
                onclick: move |_| opened.set(Some(field)),
                "{before}"
                b { "{value}" }
                "{after}"
            }
            if field != Field::Effort {
                span { class: "dot", "·" }
            }
        }
    }
}
