// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The reads a query leaves for after the view lock whose answer is not
//! this city's own record: the person's settings file, this city's
//! first-run guide, the release registry, the doctor's upstream check
//! and this machine's search path.
//!
//! Apart from `prepared`, which routes every read to the module that
//! does it, because each of these answers a fact about how this city and
//! this machine are set up rather than a line the Ledger holds: a file
//! the person edits while the page is open, a guide the next write
//! would reset, a release published
//! since, a program installed into a search path. Each is read with the
//! snapshot let go, and each decides its own refusal here, beside the
//! read it belongs to - an unreadable file is not an empty one.
//!
//! `GithubLogin`, `McpHealth`, `Toolkits` and the provider questions
//! (`Config`, `EndpointView`) are not here: each hands its whole answer
//! to the module that owns it, so the routing match is already the only
//! line those need.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use kernel::Address;

use super::{Prepared, unavailable};

/// What a read of now - a tool server's handshake, the broker's shelf -
/// needs from the views, copied out so the read runs with the snapshot
/// let go.
pub struct LiveAsk {
    pub(crate) city_root: PathBuf,
    pub(crate) city: Option<Address>,
    pub(crate) vault: Option<Arc<Mutex<gateway::Custodian>>>,
}

impl Prepared {
    /// What the person settled about how they read their cities, read
    /// from their own file at the moment of asking.
    ///
    /// An unreadable file is not an empty one: a page drawn from an
    /// empty record would tell somebody their settings were reset.
    pub(super) fn preferences_answer() -> wire::Answer {
        match crate::person::read() {
            Ok(settled) => wire::Answer::Preferences(Box::new(settled)),
            Err(_) => unavailable("Preferences".to_owned()),
        }
    }

    /// The first-run guide's progress of the city at this root.
    ///
    /// An unreadable file is not a guide the next write would reset.
    pub(super) fn guide_answer(city_root: PathBuf) -> wire::Answer {
        match crate::guide::read(&city_root) {
            Ok(progress) => wire::Answer::Guide(progress),
            Err(_) => unavailable("Guide".to_owned()),
        }
    }

    /// The release page, which leaves this machine and only on a press
    /// (`crates/wire/Spec.lean` §8-36), through the registry a served
    /// city handed the views.
    pub(super) fn release_answer(newest: Option<fn() -> wire::ReleaseAnswer>) -> wire::Answer {
        match newest {
            Some(newest) => wire::Answer::Release(Box::new(newest())),
            None => unavailable("NewestRelease".to_owned()),
        }
    }

    /// The harness page, which walks this machine's search path, and so
    /// is read after the views are released (`crates/sprawling/Spec.lean`
    /// §8-100). A city nobody served was handed no reach.
    pub(super) fn harnesses_answer(
        reach: Option<crate::views::lines::HarnessReach>,
    ) -> wire::Answer {
        match reach {
            Some(reach) => crate::views::lines::harnesses_answer(reach),
            None => unavailable("Harnesses".to_owned()),
        }
    }

    /// One item's publisher, asked for the newest version of a named
    /// item, which leaves this machine too.
    pub(super) fn upstream_answer(
        ask: Option<fn(&str) -> wire::DoctorUpstream>,
        item: String,
    ) -> wire::Answer {
        match ask {
            Some(newest) => wire::Answer::Upstream(Box::new(newest(&item))),
            None => unavailable(format!("UpstreamVersion({item})")),
        }
    }
}
