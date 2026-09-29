// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The vendors this city knows by host, as the settings page offers
//! them (channels-SPEC.md 8-51).
//!
//! A person picks a vendor rather than copying its address out of the
//! vendor's documentation. The table is `gateway::provider::preset`'s;
//! this shape only carries it, and the page decides which faces it
//! offers from the same answer rather than from a table of its own.

use kernel::DialectKind;
use serde::{Deserialize, Serialize};

/// Every host the city knows, in the table's own order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct KnownHostsAnswer {
    pub hosts: Vec<KnownHost>,
}

/// One vendor's host, and the faces it documents, main face first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct KnownHost {
    pub host: String,
    pub faces: Vec<KnownFace>,
}

/// One face and the base URL it is called at: the URL attaching this
/// face would store, so a form that fills it in changes nothing when
/// the city normalises it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct KnownFace {
    pub dialect: DialectKind,
    pub base_url: String,
}
