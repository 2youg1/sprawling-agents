// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;
use std::process::Command;

use browser::survey::probe::{Read, SETTLE_MS, body};
use serde::Deserialize;
use serde_json::json;

use super::super::pass::{HEIGHT, Pass};
use super::super::probe::PENDING;
use super::{Measured, Opening, PATIENCE, launch, read_measured};
use crate::report::XtaskError;

const COMMAND: &str = "cargo xtask render";

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
    entry: Option<&Path>,
    passes: &[&'p Pass],
) -> Result<Vec<(&'p Pass, Measured)>, XtaskError> {
    let profile = tempfile::tempdir().map_err(|error| problem(error.to_string()))?;
    let defer = match entry {
        Some(_) => guide_phrases(opening.root)?,
        None => Vec::new(),
    };
    let requested: Vec<_> = passes
        .iter()
        .map(|pass| {
            json!({
                "width": pass.width,
                "forced": pass.draws_forced_colours(),
                "probe": body(Some(pass.theme())),
            })
        })
        .collect();
    let input = json!({
        "browser": opening.browser,
        "entry": entry,
        "index": opening.bundle.join("index.html"),
        "profile": profile.path(),
        "route": opening.route,
        "height": HEIGHT,
        "passes": requested,
        "defer": defer,
        "pending": PENDING,
        "settle": SETTLE_MS,
    });
    let request = profile.path().join("render-request.json");
    std::fs::write(&request, input.to_string()).map_err(|error| problem(error.to_string()))?;
    let mut runner = Command::new("bun");
    runner
        .args(["--eval", include_str!("devtools.mjs"), "--"])
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
    let receipt: Receipt = serde_json::from_slice(&ran.stdout).map_err(|error| {
        problem(format!(
            "Chromium's Bun driver returned no readable render receipt: {error}"
        ))
    })?;
    let snapshots = match receipt {
        Receipt::Measured { results } => results,
        Receipt::Failed { error } => return Err(problem(error)),
    };
    if snapshots.len() != passes.len() {
        return Err(problem(
            "the browser did not report every requested condition",
        ));
    }
    passes
        .iter()
        .copied()
        .zip(snapshots)
        .map(|(pass, read)| Ok((pass, read_measured(read, pass, COMMAND)?)))
        .collect()
}

fn guide_phrases(root: &Path) -> Result<Vec<String>, XtaskError> {
    let spoken = crate::walk::read_text(&root.join("RefRain/src/lang.json"))?;
    let phrases: serde_json::Value =
        serde_json::from_str(&spoken).map_err(|error| problem(error.to_string()))?;
    Ok(phrases
        .get("guide_put_off_rest")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| problem("the guide defer action has no phrase table entry"))?
        .values()
        .filter_map(serde_json::Value::as_str)
        .map(str::to_owned)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::super::super::pass::wanted;
    use super::super::browser;
    use super::*;

    const FRAME_VIEW: &str = r#"<!doctype html>
<html lang="en">
<style>:root { color-scheme: dark; --color-test: #fff; }</style>
<main aria-label="Render fixture"><div data-pending></div></main>
<script>
let remaining = 60;
function nextFrame() {
  remaining -= 1;
  if (remaining > 0) requestAnimationFrame(nextFrame);
  else {
    const heading = document.createElement("h1");
    heading.textContent = "Browser frame ready";
    document.querySelector("[data-pending]").replaceWith(heading);
  }
}
requestAnimationFrame(nextFrame);
</script>
</html>"#;

    #[test]
    fn a_view_rendered_after_browser_frames_is_measured() -> Result<(), XtaskError> {
        let Some(browser) = browser() else {
            return Ok(());
        };
        let bundle = tempfile::tempdir().map_err(|error| problem(error.to_string()))?;
        std::fs::write(bundle.path().join("index.html"), FRAME_VIEW)
            .map_err(|error| problem(error.to_string()))?;
        let opening = Opening {
            root: bundle.path(),
            browser: &browser,
            bundle: bundle.path(),
            route: "fixture",
        };
        let passes = wanted()?;
        let pass = passes.first().ok_or_else(|| problem("no fixture pass"))?;
        let measured = measure(&opening, None, std::slice::from_ref(pass))?;
        assert!(measured.iter().any(|(_, result)| {
            result
                .page
                .drawn
                .iter()
                .any(|element| element.name == "Browser frame ready")
        }));
        Ok(())
    }
}
