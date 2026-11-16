// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::B3Hash;
use serde::Deserialize;

use super::check::Verdict;

const OBSERVED_MAX_BYTES: usize = 1024 * 1024;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Observation {
    page: String,

    paths: Vec<String>,

    requests: Vec<String>,

    navigations: Vec<String>,

    popups: Vec<String>,

    unresolved: Vec<String>,
}

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
