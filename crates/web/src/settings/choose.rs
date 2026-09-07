// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The model-choice form: provider, model, job.

use crate::lang::{Msg, say};
use channels::ClientFrame;
use channels::EndpointsAnswer;
use dioxus::prelude::*;

use crate::settings::forms::{SelectForm, SelectReadiness, models_of, select_ready};
use channels::ModelTag;

/// The model-choice form: provider, model, job.
#[component]
pub fn ChooseFormView(
    choice: Signal<SelectForm>,
    answer: EndpointsAnswer,
    on_frame: EventHandler<ClientFrame>,
) -> Element {
    let lang = use_context::<Signal<crate::lang::Lang>>();
    let word = move |msg: Msg| say(lang(), msg);
    rsx! {
            h2 { "{word(Msg::SettingsChooseModelHeading)}" }
            form {
                class: "choose",
                onsubmit: {
                    let served = answer.clone();
                    move |event: FormEvent| {
                        event.prevent_default();
                        if let Some(command) = crate::command::select_command(&choice.read(), &served) {
                            on_frame.call(ClientFrame::Command(Box::new(command)));
                        }
                    }
                },
                div { class: "field",
                    label { r#for: "choose-endpoint", "{word(Msg::SettingsProvider)}" }
                    select {
                        id: "choose-endpoint",
                        name: "endpoint",
                        onchange: move |event| {
                            // The model goes with the provider it came
                            // from: keeping the old name here is keeping
                            // a form that is already refused.
                            let mut picked = choice.write();
                            picked.endpoint = event.value();
                            picked.model = String::new();
                        },
                        option { value: "", "{word(Msg::SettingsWhichProvider)}" }
                        for endpoint in answer.endpoints.clone() {
                            option { key: "{endpoint.name}", value: "{endpoint.name}", "{endpoint.name}" }
                        }
                    }
                }
                div { class: "field",
                    label { r#for: "choose-model", "{word(Msg::SettingsPickModel)}" }
                    select {
                        id: "choose-model",
                        name: "model",
                        onchange: move |event| choice.write().model = event.value(),
                        option { value: "", "{word(Msg::SettingsWhichModel)}" }
                        for model in models_of(&answer, &choice.read().endpoint) {
                            option { key: "{model}", value: "{model}", "{model}" }
                        }
                    }
                }
                div { class: "field",
                    label { r#for: "choose-tag", "{word(Msg::SettingsForWhichJob)}" }
                    select {
                        id: "choose-tag",
                        name: "tag",
                        onchange: move |event| {
                            choice.write().tag = ModelTag::ALL
                                .into_iter()
                                .find(|tag| tag.to_string() == event.value());
                        },
                        option { value: "", "{word(Msg::SettingsWhatFor)}" }
                        for tag in ModelTag::ALL {
                            option { key: "{tag}", value: "{tag}", "{tag}" }
                        }
                    }
                }
                // What is missing is said beside the form, not written on
                // the button. A disabled control whose label is an error
                // message is two things at once and reads as neither.
                div { class: "field",
                    button {
                        r#type: "submit",
                        disabled: select_ready(&choice.read(), &answer) != SelectReadiness::Ready,
                        "{word(Msg::SettingsPointJobAtModel)}"
                    }
                }
                // Under the row rather than inside the last field. The
                // sentence reads the whole choice, so it belongs to the
                // form; and a field that is one line taller than its
                // neighbours drags the button out of a row whose controls
                // align on their bottom edge, which is what put this
                // sentence alongside the selects instead of beneath them.
                if select_ready(&choice.read(), &answer) != SelectReadiness::Ready {
                    span { class: "hint blocking",
                        "{word(select_ready(&choice.read(), &answer).sentence())}"
                    }
                }
            }
    }
}
