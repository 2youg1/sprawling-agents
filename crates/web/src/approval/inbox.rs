// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The inbox: things waiting for a person, grouped by cluster key.

use std::collections::BTreeMap;

use crate::lang::{Msg, fill, say};
use channels::{ApprovalClass, ApprovalItem, ClientFrame, PolicyVerdict, TimeMs};
use dioxus::prelude::*;

/// A group of identical questions, presented as one.
#[derive(Debug, Clone, PartialEq)]
pub struct Cluster {
    /// The key every member shares, rendered for a person.
    pub summary: String,
    /// Oldest first: the question that has waited longest leads.
    pub members: Vec<ApprovalItem>,
    /// Whether this group must be answered one at a time.
    pub answer_individually: bool,
}

impl Cluster {
    #[must_use]
    pub fn count(&self) -> usize {
        self.members.len()
    }

    /// When the oldest member arrived. Sorting on this puts the thing that
    /// has been blocking longest at the top, which is the only ordering a
    /// person waiting on their own work would accept.
    #[must_use]
    pub fn waiting_since(&self) -> Option<TimeMs> {
        self.members.iter().map(|item| item.created).min()
    }
}

/// Groups pending items into the list the Inbox shows.
///
/// Order is `(waiting_since, summary)`. Time leads because the oldest block
/// is the most expensive one; the summary breaks ties so the same set of
/// items always renders in the same order - two people looking at one city
/// must see one list.
#[must_use]
pub fn inbox(items: Vec<ApprovalItem>) -> Vec<Cluster> {
    let mut grouped: BTreeMap<(bool, String, String), Vec<ApprovalItem>> = BTreeMap::new();
    for item in items {
        // A tainted item is keyed by its own id, which makes its group a
        // group of one. This is C15 held by construction rather than by a
        // check somebody could forget: there is no key it can share.
        let key = if item.tainted {
            (true, item.id.as_str().to_owned(), String::new())
        } else {
            (
                false,
                format!("{:?}", item.cluster_key.class),
                item.cluster_key.detail.clone(),
            )
        };
        grouped.entry(key).or_default().push(item);
    }

    let mut clusters: Vec<Cluster> = grouped
        .into_iter()
        .map(|((tainted, class, detail), mut members)| {
            members.sort_by_key(|item| (item.created, item.id.as_str().to_owned()));
            let summary = if tainted {
                members
                    .first()
                    .map_or_else(|| class.clone(), |item| item.action_desc.clone())
            } else if detail.is_empty() {
                class
            } else {
                format!("{class}: {detail}")
            };
            Cluster {
                summary,
                members,
                answer_individually: tainted,
            }
        })
        .collect();
    clusters.sort_by(|left, right| {
        (left.waiting_since(), &left.summary).cmp(&(right.waiting_since(), &right.summary))
    });
    clusters
}

/// The inbox: what is waiting, grouped the way [`inbox`] groups it.
///
/// The page answers one item at a time and one group at a time, and the
/// difference is [`Cluster::answer_individually`] rather than a judgement
/// made here - a tainted item has no group to be answered with.
#[component]
pub fn ApprovalsView(
    items: Vec<ApprovalItem>,
    /// Whether the socket is live; see `app::Root`.
    live: Signal<bool>,
    on_frame: EventHandler<ClientFrame>,
) -> Element {
    let lang = use_context::<Signal<crate::lang::Lang>>();
    let word = move |msg: Msg| say(lang(), msg);
    let asked = use_signal(|| false);
    // What was already waiting before this page connected. The stream
    // carries what happens next and nothing earlier.
    use_effect(move || {
        let mut asked = asked;
        if live() && !asked() {
            asked.set(true);
            on_frame.call(ClientFrame::Query(channels::Query::ApprovalQueue));
        }
    });
    let clusters = inbox(items);
    let waiting: usize = clusters.iter().map(Cluster::count).sum();
    rsx! {
        section { class: "approvals",
            crate::panel::Panel {
                title: if clusters.is_empty() { word(Msg::ApprovalNothingWaiting).to_owned() }
                    else { word(Msg::ApprovalTitle).to_owned() },
                figure: (waiting > 0).then(|| waiting.to_string()),
                scope: word(Msg::ApprovalScope).to_owned(),
                source: word(Msg::ApprovalSource).to_owned(),
            if clusters.is_empty() {
                crate::panel::Empty {
                    status: word(Msg::ApprovalNoneEscalated).to_owned(),
                    what: word(Msg::ApprovalNoneEscalatedWhat).to_owned(),
                }
            }
            for cluster in clusters {
                article {
                    key: "{cluster.summary}",
                    class: if cluster.answer_individually { "cluster tainted" } else { "cluster" },
                    header {
                        span { class: "what", "{cluster.summary}" }
                        span { class: "count",
                            {fill(word(Msg::ApprovalWaitingCount),
                                  &[("count", &cluster.count().to_string())])}
                        }
                        if cluster.answer_individually {
                            span { class: "note",
                                "{word(Msg::ApprovalTainted)}"
                            }
                        }
                    }
                    for item in cluster.members.clone() {
                        div { key: "{item.id.as_str()}", class: "item",
                            span { class: "desc", "{item.action_desc}" }
                            span { class: "actor", "{item.actor}" }
                            button {
                                class: "allow",
                                onclick: {
                                    let id = item.id.clone();
                                    move |_| on_frame.call(answer_command(&id, PolicyVerdict::Allow))
                                },
                                "{word(Msg::ApprovalAllow)}"
                            }
                            button {
                                class: "deny",
                                onclick: {
                                    let id = item.id.clone();
                                    move |_| on_frame.call(answer_command(&id, PolicyVerdict::Deny))
                                },
                                "{word(Msg::ApprovalRefuse)}"
                            }
                        }
                    }
                }
            }
            }
        }
    }
}

/// Whether a standing policy could be built from this item.
///
/// One class admits policies; the rest are never waivable. Kept as a
/// judgement without a control beside it: `CreatePolicy` is on the wire
/// and no city executes it, so the button was offering a person an
/// action whose only outcome was a refusal.
///
/// The rule is the half worth keeping, and it is the half that decides
/// whether the control returns: when an executor lands, this already
/// says which items may show it.
#[must_use]
pub fn policy_admits(item: &ApprovalItem) -> bool {
    !item.tainted && matches!(item.cluster_key.class, ApprovalClass::AgentQuestion)
}

fn answer_command(id: &channels::ApprovalId, verdict: PolicyVerdict) -> ClientFrame {
    ClientFrame::Command(Box::new(channels::WireCommand::Approve {
        idem: channels::IdemKey::derive(
            &channels::RunId::CITY,
            channels::Seq::FIRST,
            id.as_str().as_bytes(),
        ),
        item: id.clone(),
        verdict,
    }))
}
