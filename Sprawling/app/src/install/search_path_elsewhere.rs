// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::{PathOutcome, on_search_path};
use kernel::AxError;

fn line(dir: &str) -> String {
    format!("export PATH=\"{dir}:$PATH\"")
}

fn current() -> String {
    std::env::var("PATH").unwrap_or_default()
}

pub(super) fn extend(dir: &str) -> Result<(PathOutcome, Option<String>), AxError> {
    if on_search_path(&current(), dir) {
        return Ok((PathOutcome::Unchanged, None));
    }
    Ok((PathOutcome::SelfService(line(dir)), None))
}

pub(super) fn retract(_dir: &str) -> Result<PathOutcome, AxError> {
    Ok(PathOutcome::Unchanged)
}
