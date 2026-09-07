// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What each model is for, and what is attached.

use crate::lang::{Msg, say};
use channels::{ClientFrame, EndpointsAnswer, Query};
use dioxus::prelude::*;

use super::page::model_count;
use super::tables::{endpoint_rows, tag_rows};

/// What each model is for, and what is attached.
#[component]
pub fn TablesView(answer: EndpointsAnswer, on_frame: EventHandler<ClientFrame>) -> Element {
    let lang = use_context::<Signal<crate::lang::Lang>>();
    let word = move |msg: Msg| say(lang(), msg);
    let rows = endpoint_rows(&answer);
    let tags = tag_rows(&answer);
    rsx! {
            h2 { "{word(Msg::SettingsWhatEachModelIsFor)}" }
            table { class: "tags",
                for row in tags {
                    tr { key: "{row.tag}",
                        td { "{row.tag}" }
                        td {
                            match row.chosen {
                                Some(choice) => rsx! { "{choice.endpoint} / {choice.model}" },
                                None => rsx! { span { class: "unset", "{word(row.consequence)}" } },
                            }
                        }
                    }
                }
            }
            h2 { "{word(Msg::SettingsWhatIsAttached)}" }
            table { class: "endpoints",
                for row in rows {
                    tr { key: "{row.name}",
                        td { "{row.name}" }
                        td { "{row.base_url}" }
                        td { "{word(row.reach)}, {word(row.credential)}" }
                        td {
                            // A provider that serves forty-six models is
                            // a fact; forty-six identifiers run together
                            // is not a reading of it. The count leads,
                            // the list is one disclosure away.
                            details { class: "models",
                                summary { "{model_count(lang(), row.models.len())}" }
                                for model in row.models {
                                    span { key: "{model}", class: "model", "{model}" }
                                }
                            }
                        }
                    }
                }
            }
            button {
                class: "refresh quiet",
                onclick: move |_| on_frame.call(ClientFrame::Query(Query::EndpointView)),
                "{word(Msg::SettingsReadItAgain)}"
            }
    }
}
