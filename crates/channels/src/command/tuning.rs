// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a person settles about one endpoint besides its address.
//!
//! Six values that are entered on one form and read together by every
//! call that endpoint carries: what to call it, how long it may take,
//! how many times a failed request is made again, how long a streamed
//! answer may run, the headers it adds, and the body fields it writes.
//! They travel as one value because a form that sent them separately
//! would let a probe and the attachment behind it disagree about what
//! they are talking to.
//!
//! **Every figure is optional, and absent means the city's own.** A
//! zero is a deadline no request can meet and a retry count nobody
//! meant, so the wire has no way to spell one by accident.
//!
//! An override's value travels as the text a person typed. What that
//! text means as JSON is `gateway`'s answer, because `gateway` is what
//! writes it into a request body; this crate carries the bytes and
//! decides nothing about them.

use kernel::Proxying;
use serde::{Deserialize, Serialize};

/// One header every request to this endpoint carries.
///
/// The value may be a `secret:realm/name` reference, which the vault
/// redeems at the moment the header is written; anything else is sent
/// as it stands. Plaintext credentials never reach this type — the
/// enrolment door is the only way a secret enters the city.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct HeaderPair {
    pub name: String,
    pub value: String,
}

/// One field written into every request body this endpoint receives.
///
/// `pointer` is a JSON pointer (`/temperature`, `/reasoning/effort`),
/// and missing object segments along it are created. `value` is the
/// text a person typed, never a parsed number: a float in a frame is a
/// float in the record that frame produces, and this city keeps floats
/// out of its ledger.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct BodyOverride {
    pub pointer: String,
    pub value: String,
}

/// Everything a person settles about one endpoint beyond its address.
///
/// `Default` is "nothing was settled", which is what a form that never
/// opened its advanced section means, and what every caller that has no
/// opinion sends.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct EndpointTuning {
    /// What to call this endpoint on screen. Absent means the id it was
    /// filed under, which is what a person who named it once meant.
    pub label: Option<String>,
    /// How long one request to this endpoint may take.
    pub timeout_ms: Option<u64>,
    /// How many times a request that failed for a reason worth retrying
    /// is made again. Zero is a legal answer and means "once, then
    /// report".
    pub request_max_retries: Option<u32>,
    /// How long a streamed answer may run. Longer than `timeout_ms` on
    /// purpose: a model that is still writing is not a model that has
    /// stopped answering.
    pub stream_idle_timeout_ms: Option<u64>,
    pub headers: Vec<HeaderPair>,
    pub overrides: Vec<BodyOverride>,
    /// Which of this endpoint's calls go through the machine's proxy.
    /// Absent is the city's own rule, which keeps a call to an address
    /// on this machine off the proxy; the other two settings exist
    /// because that rule is right about the common machine and wrong
    /// about some real ones.
    pub proxying: Option<Proxying>,
}
