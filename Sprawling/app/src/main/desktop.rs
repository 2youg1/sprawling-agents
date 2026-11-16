// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;
use std::process::ExitCode;

pub(super) fn verb(scope: Option<&str>) -> ExitCode {
    match ::desktop::serve_stdio(scope.map(Path::new)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {

            eprintln!("sprawling desktop: {err}");
            ExitCode::FAILURE
        }
    }
}
