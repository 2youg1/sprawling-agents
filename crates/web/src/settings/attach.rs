// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The attach-provider form: credential first, then endpoint.

use crate::lang::{Msg, say};
use channels::ClientFrame;
use channels::DialectKind;
use dioxus::prelude::*;

use super::tables::enrolment_note;
use crate::settings::forms::{AttachForm, AttachReadiness, ready};

/// The attach-provider form: credential first, then endpoint.
#[component]
pub fn AttachFormView(
    form: Signal<AttachForm>,
    secret_key: Signal<String>,
    enrolment: Signal<Option<String>>,
    served: std::collections::BTreeMap<String, Vec<String>>,
    on_frame: EventHandler<ClientFrame>,
) -> Element {
    let lang = use_context::<Signal<crate::lang::Lang>>();
    let word = move |msg: Msg| say(lang(), msg);
    rsx! {
            h2 { "{word(Msg::SettingsAttachProvider)}" }
            // The credential is not in this form. It goes to the
            // enrolment route first and comes back as a reference,
            // which is the only shape of it a Command can carry.
            form {
                class: "attach",
                // Dioxus 0.7 submits by default; this page never wants a
                // page navigation, so the default is refused explicitly.
                // <https://dioxuslabs.com/learn/0.7/migration/to_07/>
                onsubmit: move |event| {
                    event.prevent_default();
                    let filled = form.read().clone();
                    if let Some(command) = crate::command::attach_command(&filled) {
                        on_frame.call(ClientFrame::Command(Box::new(command)));
                    }
                },
                div { class: "field",
                    label { r#for: "attach-name", "{word(Msg::SettingsCallIt)}" }
                    input {
                        id: "attach-name",
                        name: "name",
                        placeholder: "{word(Msg::SettingsNamePlaceholder)}",
                        value: "{form.read().name}",
                        oninput: move |event| form.write().name = event.value(),
                    }
                }
                div { class: "field",
                    label { r#for: "attach-url", "{word(Msg::SettingsBaseUrl)}" }
                    input {
                        id: "attach-url",
                        name: "base_url",
                        // An address, on the domain RFC 2606 reserves for
                        // examples. The same string in every language.
                        // wording-ok: an address
                        placeholder: "https://api.provider.example/v1",
                        value: "{form.read().base_url}",
                        oninput: move |event| form.write().base_url = event.value(),
                    }
                    span { class: "hint", "{word(Msg::SettingsUrlHint)}" }
                }
                div { class: "field",
                    label { r#for: "attach-dialect", "{word(Msg::SettingsWhichWire)}" }
                    select {
                        id: "attach-dialect",
                        name: "dialect",
                        onchange: move |event| {
                            form.write().dialect = match event.value().as_str() {
                                "anthropic" => Some(DialectKind::Anthropic),
                                "openai" => Some(DialectKind::OpenAi),
                                _ => None,
                            };
                        },
                        option { value: "", "{word(Msg::SettingsWhichWire)}" }
                        // Two wire formats, named by the firms that publish
                        // them. `web::lang` refuses a phrase whose two
                        // languages are equal, and each of these is.
                        // wording-ok: a format's own name
                        option { value: "anthropic", "anthropic messages" }
                        // wording-ok: a format's own name
                        option { value: "openai", "openai chat completions" }
                    }
                }
                // The key. It is typed here and leaves immediately for
                // the enrolment route; what comes back is the reference,
                // and the key itself is never held by this page, never
                // put in a frame, and never shown again.
                div { class: "field wide",
                    label { r#for: "attach-key", "{word(Msg::SettingsKey)}" }
                    input {
                        id: "attach-key",
                        r#type: "password",
                        name: "key",
                        placeholder: "{word(Msg::SettingsKeyPlaceholder)}",
                        value: "{secret_key}",
                        oninput: move |event| secret_key.set(event.value()),
                    }
                    span { class: "hint",
                        "{word(Msg::SettingsKeyHint)}"
                    }
                }
                button {
                    r#type: "button",
                    disabled: secret_key.read().trim().is_empty()
                        || form.read().name.trim().is_empty(),
                    onclick: move |_| {
                        let realm = form.read().name.trim().to_owned();
                        let typed = secret_key.read().clone();
                        secret_key.set(String::new());
                        let said_in = lang();
                        crate::socket::enrol(&realm, "key", &typed, move |answer| {
                            let (reference, said) = enrolment_note(said_in, &answer);
                            if let Some(reference) = reference {
                                form.write().secret = Some(reference);
                            }
                            enrolment.set(Some(said));
                        });
                    },
                    "{word(Msg::SettingsPutKeyInVault)}"
                }
                if let Some(said) = enrolment.read().clone() {
                    p { class: "enrolment", "{said}" }
                }
                // Asking is free and attaching is not, so the order on
                // screen is the order of the decision: see what a key
                // buys, tick what this city may use, then register it.
                div { class: "field wide",
                    button {
                        r#type: "button",
                        class: "probe",
                        disabled: ready(&form.read()) != AttachReadiness::Ready,
                        onclick: move |_| {
                            let filled = form.read().clone();
                            if let Some(command) = crate::command::probe_command(&filled) {
                                on_frame.call(ClientFrame::Command(Box::new(command)));
                            }
                        },
                        "{word(Msg::SettingsAskWhatItServes)}"
                    }
                }
                if let Some(models) = served.get(form.read().base_url.trim()) {
                    fieldset { class: "admit",
                        legend { "{word(Msg::SettingsServes)}" }
                        for model in models.clone() {
                            label { class: "admit-row", key: "{model}",
                                input {
                                    r#type: "checkbox",
                                    name: "admit",
                                    value: "{model}",
                                    checked: form.read().admit.iter().any(|held| held == &model),
                                    onchange: {
                                        let model = model.clone();
                                        move |event: Event<FormData>| {
                                            let mut held = form.write();
                                            held.admit.retain(|kept| kept != &model);
                                            if event.checked() {
                                                held.admit.push(model.clone());
                                            }
                                        }
                                    },
                                }
                                span { "{model}" }
                            }
                        }
                        span { class: "hint", "{word(Msg::SettingsAdmitAll)}" }
                    }
                }
                div { class: "field wide submit",
                    button {
                        r#type: "submit",
                        disabled: ready(&form.read()) != AttachReadiness::Ready,
                        "{word(Msg::SettingsAttachThisProvider)}"
                    }
                    if ready(&form.read()) != AttachReadiness::Ready {
                        span { class: "hint blocking", "{word(ready(&form.read()).sentence())}" }
                    }
                }
            }
    }
}
