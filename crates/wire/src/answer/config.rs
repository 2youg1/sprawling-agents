// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What one address is actually governed by, and which file said so.

use kernel::config::SearchConfiguration;
use kernel::{Address, Effort, Proxying, ServerLabel};
use serde::{Deserialize, Serialize};

/// Which file settled one value.
///
/// **Every value in a [`ConfigAnswer`] carries one.** A settings page
/// that was told only the resolved figure could not say whether the
/// person is looking at their own entry, at something the building
/// inherited, or at the city's — so pressing "reset" and pressing
/// nothing looked the same, and the page had to climb the ladder a
/// second time to find out. This is that answer, stated once by the
/// layer that resolved it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
/// The rungs are named as `city::Layer` names them, and the two lists
/// are held together by the one match that turns the city's rung into
/// this one: a wire that called the building's file `resident` while
/// the ladder called the room's file that is a wire whose readers
/// disagree with the run about which file they are looking at.
pub enum ConfigLayer {
    /// No file states the value, and the city's built-in figure is in
    /// force. A layer rather than an absence, so the page that draws
    /// the value is told the figure instead of keeping a copy of it.
    Default,
    /// The city's own `CONFIG.toml`, which covers every building.
    City,
    /// The building's file, which covers every room under it.
    Building,
    /// The room's own file, which covers one session.
    Resident,
}

/// `[model] effort`, and the file that settled it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SettledEffort {
    pub effort: Effort,
    pub from: ConfigLayer,
}

/// `[context] second_threshold`, and the file that settled it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SettledSecond {
    /// A whole percent of the window, already taken by
    /// `kernel::config::SecondThreshold`'s one construction point.
    pub percent: u64,
    pub from: ConfigLayer,
    pub domain: SecondDomain,
}

/// The whole percents a file may state for the second rung, both ends
/// included: the two figures `kernel::config::SecondThreshold`'s one
/// construction point reads, answered so a page that states the span
/// does not spell it a second time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SecondDomain {
    pub min: u64,
    pub max: u64,
}

/// What an endpoint that settled nothing is called with.
///
/// **These are the gateway's figures, read out rather than restated.**
/// A form that shipped its own numbers as placeholder text would make
/// an endpoint attached through the form and one attached by import
/// behave differently while the person had filled in nothing; the form
/// draws what this carries, and the numbers have one home.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct TuningDefaults {
    /// Where these figures were stated. No file on the ladder states
    /// them today, so this is [`ConfigLayer::Default`]; it is answered
    /// rather than implied so a page never decides for itself that a
    /// figure is the build's own.
    pub from: ConfigLayer,
    /// How long one settled request may take, in total.
    pub timeout_ms: u64,
    /// The ceiling on making a failed request again, as
    /// `kernel::Retries::stated` spells it: absent is "until this city
    /// is halted", which is what this build asks for and what no
    /// number says. Zero is a real answer and means "once, then
    /// report".
    pub request_max_retries: Option<u32>,
    /// How long a streamed answer may stay silent before the call is
    /// abandoned. Absent means a stream is held to the same bound as a
    /// settled call, which is the only figure this city can state
    /// without inventing one.
    pub stream_idle_timeout_ms: Option<u64>,
    /// Which calls go through the machine's proxy.
    pub proxying: Proxying,
    /// How many more times one account is asked the same request before
    /// the next account takes it, where an endpoint lists two or more.
    pub account_retries: kernel::account_recovery::AccountRetries,
}

/// What one address is governed by, value by value, with the file each
/// value came from.
///
/// Answered for an address rather than for the city, because the ladder
/// has three rungs and only an address says which room's rung is in
/// play. An address naming a building answers with two rungs climbed
/// and no room entry, which is what a building with no open session is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ConfigAnswer {
    /// The address the ladder was climbed for.
    pub addr: Address,
    /// How hard the model is asked to think here. Absent means no
    /// layer stated it and the provider's own default answers, which
    /// is a statement this city deliberately makes rather than filling
    /// in a level nobody chose.
    pub effort: Option<SettledEffort>,
    /// Where the context reminder's second rung sits at this address.
    /// Always answered: where no file states it, the figure is
    /// `kernel::consts_policy::CTX_REMINDER_SECOND_DEFAULT` and the
    /// layer is [`ConfigLayer::Default`].
    pub second: SettledSecond,
    /// Where the context reminder's first rung sits, as a whole percent
    /// of the window: `kernel::consts_policy::CTX_REMINDER_FIRST_PERCENT`.
    /// No layer moves it, so no layer travels with it. Always answered by
    /// this build; absent only in a frame from a city written before it
    /// was (`crates/wire/Spec.lean` D13).
    #[serde(default)]
    pub first: Option<u64>,
    pub tuning: TuningDefaults,
    /// Which outside service `web_search` reaches here, and what the
    /// settings page edits (`crates/wire/spec/Answer/Config.lean` §8-86).
    pub search: SettledSearch,
}

/// `[search]` at one address, beside the city's own statement of it.
///
/// The value in force and the value the page edits are answered apart:
/// the page edits the city's file only, and a building that states its
/// own `[search]` would otherwise hand the page the building's value to
/// edit as the city's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SettledSearch {
    /// What `web_search` reaches at this address.
    pub configuration: SearchConfiguration,
    /// The file that stated it; [`ConfigLayer::Default`] when none has a
    /// `[search]` table.
    pub from: ConfigLayer,
    /// What the city's own file states, which is what the page edits;
    /// absent when that file has no `[search]` table.
    pub city: Option<SearchConfiguration>,
    /// The address the default supplier is reached at, read from the one
    /// place it is declared, so the page draws it without a copy.
    pub default_url: String,
    /// Whether each key of every supplier `city` lists is there.
    pub account_status: Vec<SupplierAccounts>,
}

/// The key of each account of one listed search supplier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SupplierAccounts {
    pub supplier: ServerLabel,
    pub accounts: Vec<super::AccountStatus>,
}
