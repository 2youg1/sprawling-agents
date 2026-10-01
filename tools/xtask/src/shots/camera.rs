// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The engine half of `shots`: open one route in one window, and either
//! read how far its main region scrolls or photograph one fold of it
//! (tools/xtask/Spec.lean §8-44).
//!
//! **The engine is the render gate's.** Which browser, how its dump is
//! waited for, and how a bundle opened from `file://` gets its lazily
//! loaded chunks inside the virtual-time budget are answered in
//! `render::engine`, so the two tools open a page the same way.

use std::path::{Path, PathBuf};
use std::process::Command;

use browser::survey::probe::SETTLE_MS;

use super::pages::{Frame, Shot};
use crate::render::engine::{BUDGET_MS, dump, preloads, url_of};
use crate::render::probe::{PENDING, POLL_MS};
use crate::report::XtaskError;
use crate::walk;

/// The copy of the bundle's page that carries the placing script,
/// written beside it so `./assets/...` still resolves.
const COPY: &str = "sprawling-shots.html";

/// The element the placing script writes what it measured into.
const SINK: &str = "sprawling-shots";

/// The most folds one page is photographed in. A main region taller
/// than this many windows is a page that does not stop growing, and
/// photographing it would not end either.
const MOST_FOLDS: usize = 200;

/// Where the pages come from.
#[derive(Clone, Copy)]
pub(super) enum Source<'a> {
    /// The bundle `just build-web` wrote, opened from disk with no city.
    Bundle(&'a Path),
    /// A served city's origin, whose pages carry real data and take no
    /// script, so each is photographed as one fold.
    Origin(&'a str),
}

/// An engine readied for one run: its browser, a fresh profile, and
/// where the pages come from.
pub(super) struct Camera<'a> {
    browser: &'a Path,
    profile: PathBuf,
    source: Source<'a>,
}

impl<'a> Camera<'a> {
    /// A fresh profile under `work`, so every page is drawn at its
    /// default lighting, and in bundle mode the instrumented copy.
    pub(super) fn open(
        browser: &'a Path,
        work: &Path,
        source: Source<'a>,
    ) -> Result<Camera<'a>, XtaskError> {
        let profile = work.join("profile");
        match std::fs::remove_dir_all(&profile) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(source) => {
                return Err(XtaskError::Io {
                    path: profile.display().to_string(),
                    source,
                });
            }
        }
        if let Source::Bundle(bundle) = source {
            let body = walk::read_text(&bundle.join("index.html"))?;
            let chunks = walk::files_with_ext(&bundle.join("assets"), &["js"])?;
            let names = chunks
                .iter()
                .filter_map(|chunk| chunk.file_name().and_then(|name| name.to_str()));
            let copy = bundle.join(COPY);
            std::fs::write(&copy, instrument(&body, &preloads(&body, names))).map_err(
                |source| XtaskError::Io {
                    path: copy.display().to_string(),
                    source,
                },
            )?;
        }
        Ok(Camera {
            browser,
            profile,
            source,
        })
    }

    /// Removes the instrumented copy, so the next `just dist` does not
    /// package it.
    pub(super) fn close(self) -> Result<(), XtaskError> {
        match self.source {
            Source::Bundle(bundle) => {
                let copy = bundle.join(COPY);
                std::fs::remove_file(&copy).map_err(|source| XtaskError::Io {
                    path: copy.display().to_string(),
                    source,
                })
            }
            Source::Origin(_) => Ok(()),
        }
    }

    /// Per fold of `route` in `frame`, the names of the states whose
    /// section begins in it. A page whose main region the script could
    /// not read is one fold with no states named.
    pub(super) fn folds(&self, route: &str, frame: Frame) -> Result<Vec<Vec<String>>, XtaskError> {
        if let Source::Origin(_) = self.source {
            return Ok(vec![Vec::new()]);
        }
        let mut command = self.engine(frame, 0);
        command.arg("--dump-dom").arg(self.url(route, 0));
        let dumped = dump(
            command,
            &format!("{} --dump-dom {route}", self.browser.display()),
        )?;
        Ok(folds_in(&String::from_utf8_lossy(&dumped)))
    }

    /// Photographs one shot into `picture`.
    pub(super) fn shoot(&self, shot: &Shot, picture: &Path) -> Result<(), XtaskError> {
        let mut command = self.engine(shot.frame, shot.lighting.scheme());
        command
            .arg(format!("--screenshot={}", picture.display()))
            .arg(self.url(&shot.route, shot.fold));
        dump(
            command,
            &format!("{} --screenshot {}", self.browser.display(), shot.file()),
        )
        .map(drop)
    }

    /// The engine in one window and one colour scheme, before the page.
    fn engine(&self, frame: Frame, scheme: u8) -> Command {
        let mut command = Command::new(self.browser);
        command
            .arg("--headless=new")
            .arg("--disable-gpu")
            .arg("--no-sandbox")
            .arg("--hide-scrollbars")
            .arg("--use-mock-keychain")
            .arg("--allow-file-access-from-files")
            .arg(format!("--user-data-dir={}", self.profile.display()))
            .arg(format!("--blink-settings=preferredColorScheme={scheme}"))
            .arg(format!("--window-size={},{}", frame.width, frame.height))
            .arg(format!("--virtual-time-budget={BUDGET_MS}"));
        command
    }

    fn url(&self, route: &str, fold: usize) -> String {
        match self.source {
            Source::Bundle(bundle) => {
                format!("{}?fold={fold}#/{route}", url_of(&bundle.join(COPY)))
            }
            Source::Origin(origin) => format!("{}/#/{route}", origin.trim_end_matches('/')),
        }
    }
}

/// The page with the preloads in its head and the placing script at the
/// end of its body.
fn instrument(body: &str, preloaded: &str) -> String {
    let body = match body.find("</head>") {
        Some(at) => {
            let (head, tail) = body.split_at(at);
            format!("{head}{preloaded}{tail}")
        }
        None => format!("{preloaded}{body}"),
    };
    match body.rfind("</body>") {
        Some(at) => {
            let (head, tail) = body.split_at(at);
            format!("{head}{}{tail}", script())
        }
        None => format!("{body}{}", script()),
    }
}

/// Once no view is pending, scrolls the main region to the fold the URL
/// names, and writes its scroll height, its visible height, and where
/// each top-level labelled section begins.
fn script() -> String {
    format!(
        r#"<pre id="{SINK}" hidden></pre>
<script>
(function () {{
  var fold = Number(new URLSearchParams(location.search).get('fold') || '0');
  var waited = {SETTLE_MS};
  function place() {{
    var main = document.querySelector('main');
    if (!main || document.querySelector('{PENDING}')) {{
      if (waited + {twice} < {BUDGET_MS}) {{
        waited += {POLL_MS};
        setTimeout(place, {POLL_MS});
      }}
      return;
    }}
    main.scrollTop = fold * main.clientHeight;
    var origin = main.getBoundingClientRect().top - main.scrollTop;
    var lines = [main.scrollHeight + ' ' + main.clientHeight];
    main.querySelectorAll('section[aria-label]').forEach(function (section) {{
      if (section.parentElement && section.parentElement.closest('section[aria-label]')) {{
        return;
      }}
      lines.push(Math.round(section.getBoundingClientRect().top - origin) + '\t' +
        section.getAttribute('aria-label'));
    }});
    document.getElementById('{SINK}').textContent = lines.join('\n');
  }}
  setTimeout(place, {SETTLE_MS});
}})();
</script>
"#,
        twice = POLL_MS.saturating_mul(2),
    )
}

/// What the placing script wrote, read back out of a dumped page: per
/// fold, the states that begin in it.
fn folds_in(dom: &str) -> Vec<Vec<String>> {
    let written = dom
        .find(&format!("id=\"{SINK}\""))
        .and_then(|at| dom.get(at..))
        .and_then(|rest| rest.split_once('>'))
        .and_then(|(_, rest)| rest.split_once("</pre>"))
        .map(|(text, _)| unescape(text))
        .unwrap_or_default();
    let mut lines = written.lines();
    let measured = lines.next().and_then(|first| {
        let (height, visible) = first.split_once(' ')?;
        Some((
            height.parse::<usize>().ok()?,
            visible.parse::<usize>().ok()?,
        ))
    });
    let Some((height, visible)) = measured.filter(|(_, visible)| *visible > 0) else {
        return vec![Vec::new()];
    };
    let mut folds = vec![Vec::new(); height.div_ceil(visible).clamp(1, MOST_FOLDS)];
    for (top, label) in lines.filter_map(|line| line.split_once('\t')) {
        let fold = top
            .parse::<usize>()
            .ok()
            .and_then(|top| top.checked_div(visible));
        if let Some(states) = fold.and_then(|fold| folds.get_mut(fold)) {
            states.push(label.to_owned());
        }
    }
    folds
}

/// A text node as a dumped page escapes it, read back.
fn unescape(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", "\u{a0}")
        .replace("&amp;", "&")
}
