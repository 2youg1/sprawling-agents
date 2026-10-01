// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a browser saw a playback page do, as the skill recorded it
//! (accounting-SPEC.md 8-13).
//!
//! The product never runs the page it checks. An agent opens it in the
//! browser its host already has, walks the paths it names, and writes
//! down every request, navigation, popup and dead evidence link it saw.
//! This module reads that record and asks one thing of it the product
//! can verify: that it speaks of these bytes. Whether a browser really
//! ran is the recorder's word.

use kernel::B3Hash;
use serde::Deserialize;

use super::check::Verdict;

/// The most bytes an observation record may have.
const OBSERVED_MAX_BYTES: usize = 1024 * 1024;

/// One observation record, field for field.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Observation {
    /// BLAKE3 of the page's bytes, as hex.
    page: String,
    /// The interaction paths the browser walked.
    paths: Vec<String>,
    /// Requests the page sent beyond itself; `data:` and `blob:` are not
    /// requests.
    requests: Vec<String>,
    /// Navigations away from the page.
    navigations: Vec<String>,
    /// Windows the page opened.
    popups: Vec<String>,
    /// Evidence links that pointed at nothing once followed.
    unresolved: Vec<String>,
}

/// The browser item for `record` about the page whose digest is `page`,
/// and the paths it covered when it passed.
pub(super) fn judge(record: &[u8], page: B3Hash) -> (Verdict, Vec<String>) {
    if record.len() > OBSERVED_MAX_BYTES {
        return unable(format!(
            "the observation is {} bytes, over {OBSERVED_MAX_BYTES}",
            record.len()
        ));
    }
    let seen: Observation = match serde_json::from_slice(record) {
        Ok(seen) => seen,
        Err(err) => return unable(format!("the observation does not read: {err}")),
    };
    if !seen.page.eq_ignore_ascii_case(&page.to_string()) {
        return unable(format!(
            "the observation speaks of the page {}, and this page is {page}",
            seen.page
        ));
    }
    if seen.paths.is_empty() {
        return unable("the observation names no path it walked".to_owned());
    }
    let behaviour = [
        ("a request to", &seen.requests),
        ("a navigation to", &seen.navigations),
        ("a popup at", &seen.popups),
        ("an evidence link to nothing:", &seen.unresolved),
    ]
    .into_iter()
    .find_map(|(what, list)| list.first().map(|first| format!("{what} {first}")));
    match behaviour {
        Some(found) => (Verdict::Failed { found }, Vec::new()),
        None => (Verdict::Passed, seen.paths),
    }
}

fn unable(why: String) -> (Verdict, Vec<String>) {
    (Verdict::Unable { why }, Vec::new())
}
