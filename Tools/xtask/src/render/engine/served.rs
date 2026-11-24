// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;
use std::process::Command;

use browser::survey::probe::Read;
use serde::Deserialize;
use serde_json::json;

use super::super::pass::{HEIGHT, Pass};
use super::super::probe::script;
use super::{Measured, Opening, PATIENCE, launch, read_measured};
use crate::report::XtaskError;

const COMMAND: &str = "xtask render --served-entry";

#[derive(Deserialize)]
#[serde(untagged)]
enum Receipt {
    Measured { results: Vec<Read> },
    Failed { error: String },
}

fn problem(message: impl Into<String>) -> XtaskError {
    XtaskError::Cmd {
        cmd: COMMAND.to_owned(),
        msg: message.into(),
    }
}

#[expect(clippy::disallowed_methods, reason = "developer tool (child D4)")]
pub(in crate::render) fn measure<'p>(
    opening: &Opening<'_>,
    entry: &Path,
    passes: Vec<&'p Pass>,
) -> Result<Vec<(&'p Pass, Measured)>, XtaskError> {
    let profile = tempfile::tempdir().map_err(|error| problem(error.to_string()))?;
    let spoken = crate::walk::read_text(&opening.root.join("RefRain/src/lang.json"))?;
    let phrases: serde_json::Value =
        serde_json::from_str(&spoken).map_err(|error| problem(error.to_string()))?;
    let defer = phrases
        .get("guide_put_off_rest")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| problem("the guide defer action has no phrase table entry"))?
        .values()
        .filter_map(serde_json::Value::as_str)
        .collect::<Vec<_>>();
    let requested: Vec<_> = passes
        .iter()
        .map(|pass| {
            json!({
                "width": pass.width,
                "forced": pass.draws_forced_colours(),
                "probe": script(pass),
            })
        })
        .collect();
    let input = json!({
        "browser": opening.browser,
        "entry": entry,
        "profile": profile.path(),
        "route": opening.route,
        "height": HEIGHT,
        "passes": requested,
        "defer": defer,
    });
    let request = profile.path().join("render-request.json");
    std::fs::write(&request, input.to_string()).map_err(|error| problem(error.to_string()))?;
    let mut runner = Command::new("bun");
    runner
        .args(["--eval", include_str!("served.mjs"), "--"])
        .arg(request)
        .current_dir(opening.root);
    let ran = launch::run(
        runner,
        &launch::Launch {
            cmd: COMMAND,
            stderr: launch::Stderr::Discarded,
            patience: PATIENCE,
            after_exit: launch::AfterExit::Drain,
        },
    )?;
    let receipt: Receipt =
        serde_json::from_slice(&ran.stdout).map_err(|error| problem(error.to_string()))?;
    let snapshots = match receipt {
        Receipt::Measured { results } => results,
        Receipt::Failed { error } => return Err(problem(error)),
    };
    if snapshots.len() != passes.len() {
        return Err(problem(
            "the served browser did not report every requested condition",
        ));
    }
    passes
        .into_iter()
        .zip(snapshots)
        .map(|(pass, read)| Ok((pass, read_measured(read, pass, COMMAND)?)))
        .collect()
}
