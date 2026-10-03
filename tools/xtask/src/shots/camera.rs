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
//!
//! **Each launch is a case of its own.** It gets a fresh profile that is
//! removed after it, a stderr file under `target/shots/engine/`, and a
//! patience past which its process tree is killed and the case is run
//! once more; the next launch starts only once the tree of this one has
//! let go of its output.

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use browser::survey::probe::SETTLE_MS;

use super::pages::{Frame, Shot};
use crate::render::engine::launch::{self, AfterExit, Launch, Stderr, Tree};
use crate::render::engine::{BUDGET_MS, preloads, url_of};
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

/// How long one case (one measuring or one picture) may take before its
/// tree is killed and the case is run again. The slowest case seen on a
/// four-core Windows machine took 15 s; three times that leaves room for
/// a busy runner and still ends a stalled run in minutes, not at the
/// job's timeout.
pub(super) const CASE_PATIENCE: Duration = Duration::from_secs(45);

/// How long the helpers of a launch may hold its output after the
/// engine itself exited: the two seconds the render gate drains for.
/// On Edge under Windows a helper routinely holds it longer, and a run
/// of some six hundred launches pays this bound on each of them.
const TREE_PATIENCE: Duration = Duration::from_secs(2);

/// How long a profile directory is given to come free for removal.
const PROFILE_RELEASE: Duration = Duration::from_secs(5);

/// The directory under the run's output that holds each case's stderr.
const LOGS: &str = "engine";

/// Which try of a case a launch is.
#[derive(Clone, Copy)]
pub(super) enum Attempt {
    First,
    Retry,
}

impl Attempt {
    /// What the stderr file of this try ends in.
    fn log_suffix(self) -> &'static str {
        match self {
            Attempt::First => ".log",
            Attempt::Retry => ".retry.log",
        }
    }
}

/// How a case came to succeed.
#[derive(Debug)]
pub(super) enum Took {
    First,
    /// Its first try failed for this reason, and the second succeeded.
    Retried(String),
}

/// Where the pages come from.
#[derive(Clone, Copy)]
pub(super) enum Source<'a> {
    /// The bundle `just build-web` wrote, opened from disk with no city.
    Bundle(&'a Path),
    /// A served city's origin, whose pages carry real data and take no
    /// script, so each is photographed as one fold.
    Origin(&'a str),
}

/// An engine readied for one run: its browser, where each launch's
/// profile and stderr go, and where the pages come from.
pub(super) struct Camera<'a> {
    browser: &'a Path,
    profiles: PathBuf,
    launches: Cell<u32>,
    logs: PathBuf,
    /// Each launch whose helpers outlived it, or whose profile they kept.
    strays: RefCell<Vec<String>>,
    source: Source<'a>,
}

impl<'a> Camera<'a> {
    /// A directory for this run's profiles under the system's temporary
    /// directory, so every page is drawn at its default lighting, a fresh
    /// stderr directory under `work`, and in bundle mode the instrumented
    /// copy.
    pub(super) fn open(
        browser: &'a Path,
        work: &Path,
        source: Source<'a>,
    ) -> Result<Camera<'a>, XtaskError> {
        let profiles = std::env::temp_dir().join(format!("sprawling-shots-{}", std::process::id()));
        removed(&profiles)?;
        removed(&work.join("profile"))?;
        let logs = work.join(LOGS);
        removed(&logs)?;
        std::fs::create_dir_all(&logs).map_err(|source| XtaskError::Io {
            path: logs.display().to_string(),
            source,
        })?;
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
            profiles,
            launches: Cell::new(0),
            logs,
            strays: RefCell::new(Vec::new()),
            source,
        })
    }

    /// Removes the profiles and the instrumented copy, so the next
    /// `just dist` does not package it, and returns each launch whose
    /// helpers outlived it. A stray helper does not fail the run: its
    /// profile was its own, so no later launch met it.
    pub(super) fn close(self) -> Result<Vec<String>, XtaskError> {
        let mut strays = self.strays.into_inner();
        if let Err(err) = released(&self.profiles) {
            strays.push(format!("the profiles were left behind: {err}"));
        }
        match self.source {
            Source::Bundle(bundle) => {
                let copy = bundle.join(COPY);
                std::fs::remove_file(&copy).map_err(|source| XtaskError::Io {
                    path: copy.display().to_string(),
                    source,
                })?;
                Ok(strays)
            }
            Source::Origin(_) => Ok(strays),
        }
    }

    /// Per fold of `route` in `frame`, the names of the states whose
    /// section begins in it. A page whose main region the script could
    /// not read is one fold with no states named.
    pub(super) fn folds(
        &self,
        route: &str,
        frame: Frame,
    ) -> Result<(Vec<Vec<String>>, Took), XtaskError> {
        if let Source::Origin(_) = self.source {
            return Ok((vec![Vec::new()], Took::First));
        }
        let case = format!("{}-{}-folds", route.replace('/', "-"), frame.width);
        let (dumped, took) = twice(&case, |attempt| {
            self.launch(&case, attempt, &|profile| {
                let mut command = self.engine(frame, 0, profile);
                command.arg("--dump-dom").arg(self.url(route, 0));
                command
            })
        })?;
        Ok((folds_in(&String::from_utf8_lossy(&dumped)), took))
    }

    /// Photographs one shot into `picture`; a try that exits without a
    /// picture counts as a failed one.
    pub(super) fn shoot(&self, shot: &Shot, picture: &Path) -> Result<Took, XtaskError> {
        let file = shot.file();
        let case = file.trim_end_matches(".png");
        twice(case, |attempt| {
            self.launch(case, attempt, &|profile| {
                let mut command = self.engine(shot.frame, shot.lighting.scheme(), profile);
                command
                    .arg(format!("--screenshot={}", picture.display()))
                    .arg(self.url(&shot.route, shot.fold));
                command
            })?;
            if std::fs::metadata(picture).is_ok_and(|meta| meta.len() > 0) {
                Ok(())
            } else {
                Err(XtaskError::Cmd {
                    cmd: format!("{} --screenshot {file}", self.browser.display()),
                    msg: "the engine exited without writing the picture".to_owned(),
                })
            }
        })
        .map(|((), took)| took)
    }

    /// One try of one case: a fresh profile, the engine's stderr kept in
    /// the case's file, and the profile removed once the tree let go.
    fn launch(
        &self,
        case: &str,
        attempt: Attempt,
        prepare: &dyn Fn(&Path) -> Command,
    ) -> Result<Vec<u8>, XtaskError> {
        let launches = self.launches.get().saturating_add(1);
        self.launches.set(launches);
        let profile = self.profiles.join(launches.to_string());
        let log = self.logs.join(format!("{case}{}", attempt.log_suffix()));
        let ran = launch::run(
            prepare(&profile),
            &Launch {
                cmd: &format!("{} ({case})", self.browser.display()),
                stderr: Stderr::KeptIn(&log),
                patience: CASE_PATIENCE,
                after_exit: AfterExit::AwaitTree(TREE_PATIENCE),
            },
        );
        let mut strays = self.strays.borrow_mut();
        if let Ok(launch::Ran {
            tree: Tree::Stray(why),
            ..
        }) = &ran
        {
            strays.push(format!("{case}: {why}"));
        }
        if let Err(err) = released(&profile) {
            strays.push(format!("{case}: {err}"));
        }
        ran.map(|ran| ran.stdout)
    }

    /// The engine in one window and one colour scheme, with its own
    /// profile, before the page.
    fn engine(&self, frame: Frame, scheme: u8, profile: &Path) -> Command {
        let mut command = Command::new(self.browser);
        command
            .arg("--headless=new")
            .arg("--disable-gpu")
            .arg("--no-sandbox")
            .arg("--hide-scrollbars")
            .arg("--use-mock-keychain")
            .arg("--allow-file-access-from-files")
            .arg(format!("--user-data-dir={}", profile.display()))
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

/// Runs `attempt` once, and once more when it fails; the second
/// failure is the case's, named with both reasons.
fn twice<T>(
    case: &str,
    mut attempt: impl FnMut(Attempt) -> Result<T, XtaskError>,
) -> Result<(T, Took), XtaskError> {
    match attempt(Attempt::First) {
        Ok(done) => Ok((done, Took::First)),
        Err(first) => match attempt(Attempt::Retry) {
            Ok(done) => Ok((done, Took::Retried(first.to_string()))),
            Err(again) => Err(XtaskError::Cmd {
                cmd: format!("cargo xtask shots, case {case}"),
                msg: format!(
                    "the case failed twice: first {first}; then {again}; read what the engine \
                     said in target/shots/{LOGS}/{case}.log and {case}.retry.log"
                ),
            }),
        },
    }
}

/// Removes `dir` and everything under it; a directory that is not
/// there is already removed.
fn removed(dir: &Path) -> Result<(), XtaskError> {
    match std::fs::remove_dir_all(dir) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(XtaskError::Io {
            path: dir.display().to_string(),
            source,
        }),
    }
}

/// Removes a launch's profile, giving a process still inside it up to
/// [`PROFILE_RELEASE`] to let go of its files.
fn released(profile: &Path) -> Result<(), XtaskError> {
    let pause = Duration::from_millis(250);
    let mut waited = Duration::ZERO;
    loop {
        match removed(profile) {
            Ok(()) => return Ok(()),
            Err(err) if waited >= PROFILE_RELEASE => return Err(err),
            Err(_) => {
                std::thread::sleep(pause);
                waited = waited.saturating_add(pause);
            }
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

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;

    fn failing(why: &str) -> XtaskError {
        XtaskError::Cmd {
            cmd: "engine".to_owned(),
            msg: why.to_owned(),
        }
    }

    #[test]
    fn a_case_that_fails_once_is_retried_and_says_why() {
        let (done, took) = twice("talk-01-1440-dark", |attempt| match attempt {
            Attempt::First => Err(failing("stalled")),
            Attempt::Retry => Ok(7),
        })
        .unwrap();
        assert_eq!(done, 7);
        assert!(matches!(took, Took::Retried(why) if why.contains("stalled")));
    }

    #[test]
    fn a_case_that_fails_twice_is_named_with_both_reasons() {
        let mut tries = 0;
        let err = twice::<()>("gallery-105-1440-dark", |attempt| {
            tries += 1;
            Err(failing(match attempt {
                Attempt::First => "stalled",
                Attempt::Retry => "stalled again",
            }))
        })
        .unwrap_err()
        .to_string();
        assert_eq!(tries, 2);
        assert!(err.contains("case gallery-105-1440-dark"), "{err}");
        assert!(
            err.contains("first command `engine` failed: stalled;"),
            "{err}"
        );
        assert!(err.contains("stalled again"), "{err}");
        assert!(err.contains("gallery-105-1440-dark.retry.log"), "{err}");
    }
}
