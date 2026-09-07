// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The subscription login: two steps with a person in the middle.

use crate::lang::{Msg, say};
use channels::ClientFrame;
use channels::LoginStep;
use dioxus::prelude::*;

/// The subscription login: two steps with a person in the middle.
#[component]
pub fn LoginView(
    subscription: Signal<String>,
    code: Signal<String>,
    login_url: Option<String>,
    on_frame: EventHandler<ClientFrame>,
) -> Element {
    let lang = use_context::<Signal<crate::lang::Lang>>();
    let word = move |msg: Msg| say(lang(), msg);
    rsx! {
            h2 { "{word(Msg::SettingsSignIn)}" }
            // Two steps with a person in the middle: the provider shows
            // them a code after they approve, and they bring it back.
            // Nothing here listens on a port, because the provider's own
            // page is where the code is shown.
            div { class: "subscription",
                div { class: "field",
                    label { r#for: "subscription-provider", "{word(Msg::SettingsProvider)}" }
                    select {
                        id: "subscription-provider",
                        name: "subscription_provider",
                        onchange: move |event| subscription.set(event.value()),
                        // wording-ok: a provider's own name
                        option { value: "anthropic", "anthropic" }
                        // wording-ok: a provider's own name
                        option { value: "openai", "openai" }
                    }
                }
                button {
                    r#type: "button",
                    onclick: move |_| {
                        if let Some(command) = crate::command::login_command(&subscription.read(), LoginStep::Begin)
                        {
                            on_frame.call(ClientFrame::Command(Box::new(command)));
                        }
                    },
                    "{word(Msg::SettingsStartLogin)}"
                }
                match login_url.clone() {
                    None => rsx! {
                        p { class: "unset", "{word(Msg::SettingsNoLoginWaiting)}" }
                    },
                    Some(url) => rsx! {
                        p { class: "login-step",
                            "{word(Msg::SettingsOpenApproveePaste)}"
                        }
                        a { class: "login-url", href: "{url}", target: "_blank", "{url}" }
                        div { class: "field",
                            label { r#for: "login-code", "{word(Msg::SettingsCodeLabel)}" }
                            input {
                                id: "login-code",
                                name: "code",
                                placeholder: "{word(Msg::SettingsPasteHere)}",
                                value: "{code}",
                                oninput: move |event| code.set(event.value()),
                            }
                        }
                        button {
                            r#type: "button",
                            disabled: code.read().trim().is_empty(),
                            onclick: move |_| {
                                let typed = code.read().trim().to_owned();
                                code.set(String::new());
                                if let Some(command) = crate::command::login_command(
                                    &subscription.read(),
                                    LoginStep::Code { code: typed },
                                ) {
                                    on_frame.call(ClientFrame::Command(Box::new(command)));
                                }
                            },
                            "{word(Msg::SettingsFinishLogin)}"
                        }
                    },
                }
            }
    }
}
