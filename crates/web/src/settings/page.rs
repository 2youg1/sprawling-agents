// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The settings page: where a person turns a URL and a key into a model the city can call.
//!
//! Shell only: state lives here, the four sections are components.

use crate::lang::{Msg, fill, say};
use channels::{ClientFrame, EndpointsAnswer, Query};
use dioxus::prelude::*;

use super::forms::{AttachForm, SelectForm};
use super::tables::can_dispatch;

use super::attach::AttachFormView;
use super::choose::ChooseFormView;
use super::listing::TablesView;
use super::login::LoginView;

pub(crate) fn model_count(lang: crate::lang::Lang, count: usize) -> String {
    fill(
        say(lang, Msg::SettingsModelCount),
        &[("count", &count.to_string())],
    )
}

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

/// The settings page.
///
/// It renders what the server answered and hands every action back to
/// its caller. Deciding nothing is the point: whether a registration is
/// complete is stated once, above, and read here.
#[component]
pub fn Settings(
    answer: Option<EndpointsAnswer>,
    /// Where a person must go to finish a login this session began, from
    /// `app::Snapshot`. Absent means no login is waiting on anybody.
    login_url: Option<String>,
    /// What each base URL answered when it was asked what it serves,
    /// folded from `endpoint_probed`. A person cannot tick a list they
    /// have not seen.
    served: std::collections::BTreeMap<String, Vec<String>>,
    /// Whether the socket is live; see `app::Root`.
    live: Signal<bool>,
    on_frame: EventHandler<ClientFrame>,
) -> Element {
    let lang = use_context::<Signal<crate::lang::Lang>>();
    let word = move |msg: Msg| say(lang(), msg);
    let form = use_signal(AttachForm::default);
    let choice = use_signal(SelectForm::default);
    let key = use_signal(String::new);
    let subscription = use_signal(|| "anthropic".to_owned());
    let code = use_signal(String::new);
    let enrolment = use_signal(|| None::<String>);
    let asked = use_signal(|| false);
    // This page used to wait to be asked. It has a refresh button, and
    // nothing else in the client ever sent the query - so a person who
    // opened settings saw "asking the server what is attached" until they
    // found the button. A page that needs an answer asks for it.
    use_effect(move || {
        let mut asked = asked;
        if live() && !asked() {
            asked.set(true);
            on_frame.call(ClientFrame::Query(Query::EndpointView));
        }
    });
    let Some(answer) = answer else {
        return rsx! {
            section { class: "settings",
                crate::panel::Empty {
                    status: word(Msg::SettingsAsking).to_owned(),
                    what: word(Msg::SettingsAskingWhat).to_owned(),
                }
            }
        };
    };
    let dispatchable = can_dispatch(&answer);
    let attached = answer.endpoints.len();
    rsx! {
        section { class: "settings",
            crate::panel::Panel {
                title: if dispatchable {
                        word(Msg::SettingsDispatchable).to_owned()
                    } else {
                        word(Msg::SettingsNotDispatchable).to_owned()
                    },
                figure: (attached > 0).then(|| attached.to_string()),
                scope: word(Msg::SettingsScope).to_owned(),
                source: word(Msg::SettingsSource).to_owned(),
            if attached == 0 {
                crate::panel::Empty {
                    status: word(Msg::SettingsNoProvider).to_owned(),
                    what: word(Msg::SettingsNoProviderWhat).to_owned(),
                }
            }
            AttachFormView { form: form, secret_key: key, enrolment: enrolment, served: served, on_frame: on_frame }
            LoginView { subscription: subscription, code: code, login_url: login_url, on_frame: on_frame }
            ChooseFormView { choice: choice, answer: answer.clone(), on_frame: on_frame }
            TablesView { answer: answer.clone(), on_frame: on_frame }
            }
            // The language every word this client writes is said in.
            // Above the type panel because it is the setting a person
            // came here for, and beside it because both are about how
            // this interface reads rather than what the city did.
            crate::panel::Panel {
                title: crate::lang::say(lang(), crate::lang::Msg::SettingsLanguage).to_owned(),
                scope: crate::lang::say(lang(), crate::lang::Msg::SettingsLanguageScope).to_owned(),
                source: crate::lang::say(lang(), crate::lang::Msg::SettingsLanguageSource)
                    .to_owned(),
                div { class: "languages",
                    for choice in crate::lang::Lang::ALL {
                        button {
                            key: "{choice}",
                            "aria-current": if lang() == choice { "true" } else { "false" },
                            onclick: move |_| {
                                let mut held = lang;
                                held.set(choice);
                                crate::lang::remember(choice);
                            },
                            "{choice.endonym()}"
                        }
                    }
                }
            }
            // Interface. One setting, and it is not ours to hold.
            crate::panel::Panel {
                title: word(Msg::SettingsInterfaceTitle).to_owned(),
                scope: word(Msg::SettingsInterfaceScope).to_owned(),
                source: word(Msg::SettingsInterfaceSource).to_owned(),
                p { class: "note", "{word(Msg::SettingsInterfaceFaces)}" }
                p { class: "note", "{word(Msg::SettingsInterfaceContent)}" }
            }
        }
    }
}
