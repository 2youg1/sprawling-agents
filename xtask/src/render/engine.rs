// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The engine half of the render gate: find a browser, instrument a
//! copy of the screen, draw it, and read the boxes back.

use std::path::{Path, PathBuf};
use std::process::Command;

use super::Box;
use crate::report::XtaskError;
use crate::walk;

/// Where the instrumented copies and the throwaway browser profile go.
const WORK: &str = "target/render";

/// The window the screens are judged in, in CSS pixels.
///
/// One size rather than a sweep: the properties asserted here are true at
/// every width, and a second viewport would double the run time to
/// re-check the same three facts. A narrow-window rule (what wraps, what
/// collapses) is a different gate and does not exist yet.
const VIEWPORT: (u32, u32) = (1440, 1200);

/// The element the probe writes its measurements into.
const SINK: &str = "sprawling-render";

/// What it takes to render one screen: the tree the screens live in, the
/// engine that draws them, and the scratch directory the instrumented
/// copies go to. The three always travel together, so they have a name.
pub(super) struct Engine<'tree> {
    root: &'tree Path,
    browser: PathBuf,
    work: PathBuf,
}

/// The engine to render with: named by the environment, or the first one
/// of the three desktops' own browsers that is installed.
pub(super) fn browser() -> Option<PathBuf> {
    if let Some(named) = std::env::var_os("SPRAWLING_BROWSER") {
        let path = PathBuf::from(named);
        if path.is_file() {
            return Some(path);
        }
    }
    const FIXED: [&str; 6] = [
        r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
        r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
        r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
        "/usr/bin/chromium",
    ];
    for candidate in FIXED {
        let path = PathBuf::from(candidate);
        if path.is_file() {
            return Some(path);
        }
    }
    on_path(&[
        "google-chrome",
        "chromium",
        "chromium-browser",
        "microsoft-edge",
    ])
}

/// The first of these names that is executable somewhere on `PATH`.
fn on_path(names: &[&str]) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&paths) {
        for name in names {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

impl<'tree> Engine<'tree> {
    /// Prepare the scratch directory the instrumented copies go to.
    pub(super) fn new(root: &'tree Path, browser: PathBuf) -> Result<Self, XtaskError> {
        let work = root.join(WORK);
        std::fs::create_dir_all(&work).map_err(|source| XtaskError::Io {
            path: walk::rel(root, &work),
            source,
        })?;
        Ok(Self {
            root,
            browser,
            work,
        })
    }

    /// Render one screen and read its boxes back.
    pub(super) fn measure(&self, screen: &Path, rel: &str) -> Result<Vec<Box>, XtaskError> {
        let name = screen
            .file_name()
            .and_then(|held| held.to_str())
            .unwrap_or("screen.html");
        let body = walk::read_text(screen)?;
        let source = screen.parent().unwrap_or(self.root);
        let instrumented = self.work.join(name);
        std::fs::write(&instrumented, instrument(&body, source)).map_err(|source| {
            XtaskError::Io {
                path: walk::rel(self.root, &instrumented),
                source,
            }
        })?;
        let profile = self.work.join("profile");
        let output = Command::new(&self.browser)
            .arg("--headless=new")
            .arg("--disable-gpu")
            .arg("--no-sandbox")
            .arg("--hide-scrollbars")
            .arg(format!("--user-data-dir={}", profile.display()))
            .arg(format!("--window-size={},{}", VIEWPORT.0, VIEWPORT.1))
            .arg("--virtual-time-budget=4000")
            .arg("--dump-dom")
            .arg(url_of(&instrumented))
            .output()
            .map_err(|err| XtaskError::Cmd {
                cmd: format!("{} --dump-dom", self.browser.display()),
                msg: err.to_string(),
            })?;
        let dom = String::from_utf8_lossy(&output.stdout);
        let Some(records) = sink(&dom) else {
            return Err(XtaskError::Cmd {
                cmd: format!("{} --dump-dom {rel}", self.browser.display()),
                msg: "the probe wrote nothing; the engine rendered no page or ran no script"
                    .to_owned(),
            });
        };
        let boxes: Vec<Box> = records.split(" ; ").filter_map(parse_box).collect();
        if boxes.is_empty() {
            return Err(XtaskError::Cmd {
                cmd: format!("{} --dump-dom {rel}", self.browser.display()),
                msg: "the page rendered no centre column and no panel".to_owned(),
            });
        }
        Ok(boxes)
    }
}

/// The measurements the probe left in the document, if it ran.
pub(super) fn sink(dom: &str) -> Option<&str> {
    let open = format!("<pre id=\"{SINK}\">");
    let start = dom.find(&open)?.checked_add(open.len())?;
    let rest = dom.get(start..)?;
    let end = rest.find("</pre>")?;
    rest.get(..end)
}

/// `kind tag class left top width height`, as the probe writes it.
pub(super) fn parse_box(record: &str) -> Option<Box> {
    let mut field = record.split_whitespace();
    let kind = field.next()?.to_owned();
    let tag = field.next()?.to_owned();
    let class = field.next()?.to_owned();
    let left = field.next()?.parse().ok()?;
    let top = field.next()?.parse().ok()?;
    let width = field.next()?.parse().ok()?;
    let height = field.next()?.parse().ok()?;
    Some(Box {
        kind,
        tag,
        class,
        left,
        top,
        width,
        height,
    })
}

/// Rewrite the stylesheet links to absolute file URLs and append the
/// probe.
///
/// The links are rewritten rather than the copy being written beside the
/// screen: a temporary file inside `crates/web/screens` is a file the
/// other gates walk, and one left behind by an interrupted run would be
/// judged as a screen.
fn instrument(body: &str, source: &Path) -> String {
    let mut out = String::with_capacity(body.len().saturating_add(PROBE.len()));
    let mut rest = body;
    while let Some(at) = rest.find("href=\"") {
        let Some(head) = rest.get(..at) else { break };
        let Some(tail) = rest.get(at.saturating_add(6)..) else {
            break;
        };
        let Some(close) = tail.find('"') else { break };
        let Some(value) = tail.get(..close) else {
            break;
        };
        out.push_str(head);
        out.push_str("href=\"");
        if value.ends_with(".css") {
            out.push_str(&url_of(&source.join(value)));
        } else {
            out.push_str(value);
        }
        out.push('"');
        rest = tail.get(close.saturating_add(1)..).unwrap_or("");
    }
    out.push_str(rest);
    match out.rfind("</body>") {
        Some(at) => {
            let (head, tail) = out.split_at(at);
            format!("{head}{PROBE}{tail}")
        }
        None => format!("{out}{PROBE}"),
    }
}

/// A `file://` URL for a path, in the form every engine accepts.
fn url_of(path: &Path) -> String {
    let absolute = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let text = absolute.display().to_string();
    let cleaned = text
        .strip_prefix(r"\\?\")
        .unwrap_or(&text)
        .replace('\\', "/");
    if cleaned.starts_with('/') {
        format!("file://{cleaned}")
    } else {
        format!("file:///{cleaned}")
    }
}

/// The measuring script.
///
/// It writes one line per box into a `<pre>` the dump then carries back,
/// because `--dump-dom` returns the document and nothing else: anything
/// the gate wants to know has to be in the document when it is dumped.
const PROBE: &str = r#"<pre id="sprawling-render"></pre>
<script>
(function(){
  var out = [];
  function box(kind, node) {
    var r = node.getBoundingClientRect();
    var cls = (node.getAttribute('class') || '-').trim().split(/\s+/).join('.');
    out.push([kind, node.tagName, cls,
              Math.round(r.left), Math.round(r.top),
              Math.round(r.width), Math.round(r.height)].join(' '));
  }
  var centre = document.querySelector('.centre');
  if (centre) {
    box('centre', centre);
    for (var i = 0; i < centre.children.length; i++) { box('region', centre.children[i]); }
  }
  var panels = document.querySelectorAll('.panel');
  for (var p = 0; p < panels.length; p++) {
    box('panel', panels[p]);
    var parts = panels[p].children;
    for (var j = 0; j < parts.length; j++) { box('part', parts[j]); }
  }
  document.getElementById('sprawling-render').textContent = out.join(' ; ');
})();
</script>
"#;
