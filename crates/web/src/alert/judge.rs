// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The judgement: what needs a person, and only once per fact.

use std::collections::BTreeSet;

use channels::{AxError, EventKind, EventRecord};

use crate::lang::{Lang, Msg, say};

/// What the city said back to the person who asked for something.
///
/// Deliberately **not** an [`Alert`]: an `Alert` is a standing fact and
/// is raised once however often it is seen, while a refusal answers one
/// action. Pressing the same button twice with the same wrong URL is two
/// questions and deserves two answers, so this never goes through
/// [`Alerts`] and is never deduplicated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refused {
    /// The stable code, which is what a person quotes when they ask.
    pub code: String,
    /// What the city would not do, in its own words.
    pub what: String,
    /// The way out. Never empty on screen: a refusal that leaves a
    /// person with nothing to try is the failure this path exists to
    /// remove, so the absence is stated rather than rendered as a gap.
    pub recovery: String,
}

/// Turns a refusal into the three things a person needs from it.
///
/// Before this existed the client received `ServerFrame::Refusal` and
/// did nothing with it, so a mistyped base URL produced a page that said
/// nothing at all and a line in a log file nobody was reading.
///
/// The action and the subject are quoted rather than translated: they
/// are what the city said, and this client's language switch governs
/// what this client writes (web-SPEC.md section 8-37). Only the sentence
/// standing in for a missing recovery is ours to say.
#[must_use]
pub fn refused(lang: Lang, error: &AxError) -> Refused {
    let recovery = error.recovery();
    Refused {
        code: error.code().as_str().to_owned(),
        what: crate::lang::fill(
            say(lang, Msg::AlertCannot),
            &[("action", error.action()), ("subject", error.subject())],
        ),
        recovery: if recovery.is_empty() {
            say(lang, Msg::AlertNoRecovery).to_owned()
        } else {
            recovery.to_owned()
        },
    }
}

/// Why a person is needed. Exhaustive: a new reason has to be added here,
/// which is where somebody is forced to ask whether it really requires a
/// human or merely worries the author.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlertKind {
    /// An ApprovalItem is waiting.
    AwaitingApproval,
    /// A Run was frozen: budget, watchdog, or a limit.
    RunFrozen,
    /// A provider is degraded or gone.
    ProviderTrouble,
    /// A gate refused something that cannot proceed without a ruling.
    Refused,
}

impl AlertKind {
    /// What a notification is titled. A message rather than a string:
    /// this is the one line a person reads with the tab closed, and it
    /// reads in the language they chose.
    #[must_use]
    pub fn title(self) -> Msg {
        match self {
            Self::AwaitingApproval => Msg::AlertAwaitingApproval,
            Self::RunFrozen => Msg::AlertRunFrozen,
            Self::ProviderTrouble => Msg::AlertProviderTrouble,
            Self::Refused => Msg::AlertRefused,
        }
    }

    /// Whether this kind interrupts a person who is not looking at the tab.
    ///
    /// Only two do. A refusal is already visible where the person is
    /// working, and a degraded provider is a condition rather than an
    /// event - interrupting for either teaches people to dismiss
    /// notifications, which costs the two that matter.
    #[must_use]
    pub fn interrupts(self) -> bool {
        matches!(self, Self::AwaitingApproval | Self::RunFrozen)
    }
}

/// One thing needing a person.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alert {
    pub kind: AlertKind,
    /// Stable across repeats of the same fact. Two alerts with one key are
    /// one fact seen twice.
    pub key: String,
    pub message: String,
}

/// What the interface should do about an alert.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Raise {
    /// Show it in place, and mark the row.
    Mark,
    /// Show it, and interrupt: a browser notification.
    Interrupt,
    /// Already raised; do nothing at all.
    Silent,
}

/// The single alerting authority: remembers what has been raised so nothing
/// is raised twice.
#[derive(Debug, Default)]
pub struct Alerts {
    raised: BTreeSet<String>,
}

impl Alerts {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Decides what to do with one alert.
    ///
    /// The first sighting of a key may interrupt; every later sighting of
    /// the same key is silent, until [`Alerts::clear`] says the fact is
    /// over. This is what keeps a Run that stays frozen for an hour from
    /// notifying once a second.
    pub fn raise(&mut self, alert: &Alert) -> Raise {
        if !self.raised.insert(alert.key.clone()) {
            return Raise::Silent;
        }
        if alert.kind.interrupts() {
            Raise::Interrupt
        } else {
            Raise::Mark
        }
    }

    /// The fact is over: the approval was answered, the Run resumed. The key
    /// may raise again if it comes back, because the second occurrence is a
    /// new fact rather than an echo of the first.
    pub fn clear(&mut self, key: &str) -> bool {
        self.raised.remove(key)
    }
}

/// What one event asks of a person, if it asks anything.
///
/// Exhaustive by omission on purpose: the default is that an event needs
/// nobody. Adding a kind here is where somebody has to argue that it
/// really requires a human rather than merely worrying the author.
#[must_use]
pub fn alert_for(lang: Lang, record: &EventRecord) -> Option<Alert> {
    let map = record.data().as_map();
    match record.kind() {
        EventKind::ApprovalRequested => {
            // The payload is the item, so the key is the item's own id -
            // the same identity the answer will carry.
            let id = map.get("id").and_then(serde_json::Value::as_str)?;
            Some(Alert {
                kind: AlertKind::AwaitingApproval,
                key: format!("approval/{id}"),
                // The city's own words when it wrote any; ours only
                // when it did not.
                message: map
                    .get("action_desc")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_else(|| say(lang, Msg::AlertSomethingWaiting))
                    .to_owned(),
            })
        }
        EventKind::RunFrozen | EventKind::BudgetLimit => Some(Alert {
            kind: AlertKind::RunFrozen,
            key: format!("run/{}", record.run()),
            message: say(lang, Msg::AlertRunStopped).to_owned(),
        }),
        EventKind::ProviderDegraded | EventKind::EndpointLost => Some(Alert {
            kind: AlertKind::ProviderTrouble,
            key: PROVIDER_KEY.to_owned(),
            message: say(lang, Msg::AlertProviderNotAnswering).to_owned(),
        }),
        _ => None,
    }
}

/// Which fact one event ends, if it ends one.
///
/// A fact that ends may be raised again later, and the second time is a
/// real second time rather than an echo of the first.
#[must_use]
pub fn cleared_by(record: &EventRecord) -> Option<String> {
    match record.kind() {
        EventKind::ApprovalResolved => record
            .data()
            .as_map()
            .get("id")
            .and_then(serde_json::Value::as_str)
            .map(|id| format!("approval/{id}")),
        EventKind::EndpointAttached => Some(PROVIDER_KEY.to_owned()),
        _ => None,
    }
}

/// The key every provider complaint shares: one sick provider is one fact,
/// however many calls notice it.
const PROVIDER_KEY: &str = "provider";

/// Folds one event into the alert state, and says what to do about it.
///
/// Called beside `Snapshot::apply`, from the one place events arrive, so
/// "what needs a person" is decided in the same pass as "what happened"
/// rather than by a second reader of the stream.
pub fn absorb(lang: Lang, alerts: &mut Alerts, record: &EventRecord) -> Raise {
    if let Some(key) = cleared_by(record) {
        alerts.clear(&key);
    }
    match alert_for(lang, record) {
        Some(alert) => alerts.raise(&alert),
        None => Raise::Silent,
    }
}
