// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use browser::survey::probe::SETTLE_MS;

use super::case::{Attempt, LOGS, Took, released, removed, twice};
use super::pages::{Frame, Shot};
use crate::render::engine::launch::{self, AfterExit, Launch, Stderr, Tree};
use crate::render::engine::{BUDGET_MS, preloads, url_of};
use crate::render::probe::{PENDING, POLL_MS};
use crate::report::XtaskError;
use crate::walk;

const COPY: &str = "sprawling-shots.html";

const SINK: &str = "sprawling-shots";

const MOST_FOLDS: usize = 512;

pub(super) const CASE_PATIENCE: Duration = Duration::from_secs(45);

const TREE_PATIENCE: Duration = Duration::from_secs(2);

#[derive(Clone, Copy)]
pub(super) enum Source<'a> {
    Bundle(&'a Path),

    Origin(&'a str),
}

pub(super) struct Camera<'a> {
    browser: &'a Path,
    profiles: PathBuf,
    launches: Cell<u32>,
    logs: PathBuf,

    strays: RefCell<Vec<String>>,
    source: Source<'a>,
}

impl<'a> Camera<'a> {
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

    #[expect(clippy::disallowed_methods, reason = "developer tool (child D4)")]
    fn engine(&self, frame: Frame, scheme: u8, profile: &Path) -> Command {
        let mut command = Command::new(self.browser);
        command
            .arg("--headless=new")
            .arg("--disable-gpu")
            .arg("--no-sandbox")
            .arg("--no-first-run")
            .arg("--no-default-browser-check")
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
    hold(main, read);
  }}
  function read(main) {{
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
  function hold(main, then) {{
    var loads = 0;
    var drawn = false;
    function load() {{
      if (drawn) {{ return; }}
      loads += 1;
      var image = new Image();
      image.onload = image.onerror = load;
      image.src = './{COPY}?hold=' + loads;
    }}
    function settle() {{
      if (waited + {twice} >= {BUDGET_MS}) {{ return; }}
      waited += {POLL_MS};
      setTimeout(function () {{
        then(main);
        settle();
      }}, {POLL_MS});
    }}
    var seen = new IntersectionObserver(function () {{
      seen.disconnect();
      requestAnimationFrame(function () {{
        requestAnimationFrame(function () {{
          drawn = true;
          then(main);
        }});
      }});
    }});
    seen.observe(main);
    load();
    settle();
  }}
  setTimeout(place, {SETTLE_MS});
}})();
</script>
"#,
        twice = POLL_MS.saturating_mul(2),
    )
}

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

    const FRAME_LATE: &str = r#"<!doctype html>
<html><head></head><body>
<main style="display:block;height:400px;overflow:auto"><div id="view" style="height:2000px"></div></main>
<script>
setTimeout(function () {
  var view = document.getElementById('view');
  var seen = new IntersectionObserver(function () {
    seen.disconnect();
    requestAnimationFrame(function () {
      var state = document.createElement('section');
      state.setAttribute('aria-label', 'measured in view');
      view.appendChild(state);
    });
  });
  seen.observe(view);
}, SETTLE);
</script>
</body></html>
"#;

    const TIMER_LATE: &str = r#"<!doctype html>
<html><head></head><body>
<main style="display:block;height:400px;overflow:auto"><div id="view" style="height:2000px"></div></main>
<script>
setTimeout(function () {
  var state = document.createElement('section');
  state.setAttribute('aria-label', 'written late');
  document.getElementById('view').appendChild(state);
}, LATE);
</script>
</body></html>
"#;

    const LATE_MS: u32 = 3000;

    #[test]
    fn the_reading_carries_a_state_written_late_in_the_budget() {
        reads_the_state(
            TIMER_LATE.replace("LATE", &LATE_MS.to_string()),
            "written late",
        );
    }

    #[test]
    #[ignore = "a hosted runner's engine draws no frame in a dump run; `just shots-frames` runs this on a machine whose engine draws"]
    fn a_frame_after_the_scroll_writes_a_state_the_reading_carries() {
        reads_the_state(
            FRAME_LATE.replace("SETTLE", &SETTLE_MS.to_string()),
            "measured in view",
        );
    }

    fn reads_the_state(page: String, wanted: &str) {
        let Some(browser) = crate::render::engine::browser() else {
            return;
        };
        let dir = std::env::temp_dir().join(format!("xtask-shots-hold-{}", std::process::id()));
        let bundle = dir.join("bundle");
        std::fs::create_dir_all(bundle.join("assets")).unwrap();
        std::fs::write(bundle.join("index.html"), page).unwrap();
        let camera = Camera::open(&browser, &dir.join("work"), Source::Bundle(&bundle)).unwrap();
        let read = camera.folds(
            "fixture",
            Frame {
                width: 800,
                height: 600,
            },
        );
        camera.close().unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        let (folds, _) = read.unwrap();
        assert!(
            folds.iter().flatten().any(|state| state == wanted),
            "{wanted} is not in {folds:?}"
        );
    }
}
