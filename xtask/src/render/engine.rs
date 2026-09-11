// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The engine half of the render gate: find a browser, open the gallery
//! out of the bundle a person runs, and read back what was drawn.

use std::path::{Path, PathBuf};
use std::process::Command;

use super::Drawn;
use crate::report::XtaskError;
use crate::walk;

/// Where the instrumented copy and the throwaway browser profile go.
const WORK: &str = "target/render";

/// The window the gallery is judged in, in CSS pixels.
///
/// One size rather than a sweep: the properties asserted here are true at
/// every width, and a second viewport would double the run time to
/// re-check the same facts. A narrow-window rule (what wraps, what
/// collapses) is a different gate and does not exist yet.
const VIEWPORT: (u32, u32) = (1440, 1200);

/// The element the probe writes its measurements into.
const SINK: &str = "sprawling-render";

/// How long the probe waits for the client to mount before measuring.
///
/// Virtual time, not wall time: `--virtual-time-budget` advances timers as
/// fast as the work allows, so this is a number of frames' worth of
/// scheduling rather than a second of somebody's life.
const SETTLE_MS: u32 = 1200;
const BUDGET_MS: u32 = 8000;

/// The engine to render with, in the three tiers section 8-13 fixes.
///
/// Each tier is an authority that already exists. The gate does not keep
/// a fourth list of its own, and it does not probe for what `doctor`
/// already knows.
pub(super) fn browser() -> Option<PathBuf> {
    if let Some(named) = std::env::var_os("SPRAWLING_BROWSER") {
        let path = PathBuf::from(named);
        if path.is_file() {
            return Some(path);
        }
    }
    if let Some(installed) = in_components() {
        return Some(installed);
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

/// What `doctor` installed, read as the filesystem convention it is
/// (kernel-SPEC.md section 8-22, P4.02) rather than as a second probe for
/// where a browser lives on this machine.
fn in_components() -> Option<PathBuf> {
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"))?;
    let components = PathBuf::from(home).join(".sprawling").join("components");
    const UNDER: [&str; 4] = [
        "firefox/firefox.exe",
        "firefox/firefox",
        "chromium/chrome.exe",
        "chromium/chrome",
    ];
    UNDER
        .into_iter()
        .map(|leaf| components.join(leaf))
        .find(|path| path.is_file())
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

/// Open one route of the built client and read back every element the
/// properties are about.
///
/// The bundle is copied and instrumented rather than probed in place: the
/// probe is a `<script>` this repository does not ship, and writing it
/// into `target/web-dist` would leave it in whatever `just dist` packages
/// next.
pub(super) fn measure(
    root: &Path,
    browser: &Path,
    bundle: &Path,
    route: &str,
) -> Result<Vec<Drawn>, XtaskError> {
    let work = root.join(WORK);
    std::fs::create_dir_all(&work).map_err(|source| XtaskError::Io {
        path: walk::rel(root, &work),
        source,
    })?;
    let page = bundle.join("index.html");
    let body = walk::read_text(&page)?;
    // Beside the bundle, so that `./assets/…` still resolves: the copy is
    // named for the gate, and `just dist` writes the directory fresh.
    let instrumented = bundle.join("sprawling-render.html");
    std::fs::write(&instrumented, instrument(&body)).map_err(|source| XtaskError::Io {
        path: walk::rel(root, &instrumented),
        source,
    })?;
    let profile = work.join("profile");
    let output = Command::new(browser)
        .arg("--headless=new")
        .arg("--disable-gpu")
        .arg("--no-sandbox")
        .arg("--hide-scrollbars")
        // The bundle is a module script, and a module fetched from
        // `file://` is a cross-origin fetch. Serving it would mean a
        // server in the gate; this flag is the same statement without
        // one, and it applies to a throwaway profile.
        .arg("--allow-file-access-from-files")
        .arg(format!("--user-data-dir={}", profile.display()))
        .arg(format!("--window-size={},{}", VIEWPORT.0, VIEWPORT.1))
        .arg(format!("--virtual-time-budget={BUDGET_MS}"))
        .arg("--dump-dom")
        .arg(format!("{}{route}", url_of(&instrumented)))
        .output()
        .map_err(|err| XtaskError::Cmd {
            cmd: format!("{} --dump-dom", browser.display()),
            msg: err.to_string(),
        })?;
    let dom = String::from_utf8_lossy(&output.stdout);
    let _ = std::fs::remove_file(&instrumented);
    let Some(records) = sink(&dom) else {
        return Err(XtaskError::Cmd {
            cmd: format!("{} --dump-dom {route}", browser.display()),
            msg: "the probe wrote nothing; the engine rendered no page or ran no script".to_owned(),
        });
    };
    Ok(records.split(" ; ").filter_map(parse).collect())
}

/// The measurements the probe left in the document, if it ran.
fn sink(dom: &str) -> Option<&str> {
    let open = format!("<pre id=\"{SINK}\">");
    let start = dom.find(&open)?.checked_add(open.len())?;
    let rest = dom.get(start..)?;
    let end = rest.find("</pre>")?;
    rest.get(..end)
}

/// `tag role name left top width height`, as the probe writes it. The name
/// is percent-encoded, because it is the one field that holds a person's
/// words and those contain spaces.
fn parse(record: &str) -> Option<Drawn> {
    let mut field = record.split_whitespace();
    let tag = field.next()?.to_owned();
    let role = field.next()?.to_owned();
    let name = decode(field.next()?);
    Some(Drawn {
        tag,
        role,
        name,
        left: field.next()?.parse().ok()?,
        top: field.next()?.parse().ok()?,
        width: field.next()?.parse().ok()?,
        height: field.next()?.parse().ok()?,
        depth: field.next()?.parse().ok()?,
        parent: field.next()?.parse().ok()?,
    })
}

/// Percent-decoding, which is all the probe needs on this side.
fn decode(field: &str) -> String {
    let bytes = field.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut index: usize = 0;
    while let Some(byte) = bytes.get(index) {
        if *byte == b'%'
            && let Some(pair) =
                field.get(index.saturating_add(1)..index.saturating_add(3).min(field.len()))
            && let Ok(value) = u8::from_str_radix(pair, 16)
        {
            out.push(value);
            index = index.saturating_add(3);
            continue;
        }
        out.push(*byte);
        index = index.saturating_add(1);
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Append the probe to a copy of the page.
fn instrument(body: &str) -> String {
    match body.rfind("</body>") {
        Some(at) => {
            let (head, tail) = body.split_at(at);
            format!("{head}{}{tail}", probe())
        }
        None => format!("{body}{}", probe()),
    }
}

/// The measuring script.
///
/// It writes one line per element into a `<pre>` the dump then carries
/// back, because `--dump-dom` returns the document and nothing else:
/// anything the gate wants to know has to be in the document when it is
/// dumped.
///
/// The accessible name is taken the way a reader gets it — an explicit
/// `aria-label`, the element a `aria-labelledby` points at, a `title`, an
/// `alt`, or the text the element actually contains. It is not a computed
/// accessibility tree and does not claim to be; it is what the four
/// authoring mistakes this gate exists for all show up in.
fn probe() -> String {
    format!(
        r#"<pre id="{SINK}"></pre>
<script>
setTimeout(function () {{
  var out = [];
  var seen = [];
  function named(node) {{
    var label = node.getAttribute('aria-label');
    if (label) return label;
    var by = node.getAttribute('aria-labelledby');
    if (by) {{
      var target = document.getElementById(by);
      if (target) return (target.textContent || '').trim();
    }}
    var title = node.getAttribute('title');
    if (title) return title;
    var alt = node.getAttribute('alt');
    if (alt) return alt;
    return (node.textContent || '').trim();
  }}
  var all = document.querySelectorAll(
    'main, nav, aside, header, footer, section, h1, h2, h3, button, a, input, textarea, select, [role]'
  );
  for (var i = 0; i < all.length; i++) {{
    var node = all[i];
    var rect = node.getBoundingClientRect();
    var depth = 0;
    for (var up = node.parentElement; up; up = up.parentElement) depth++;
    var parent = -1;
    for (var s = 0; s < seen.length; s++) {{ if (seen[s].contains(node)) parent = s; }}
    seen.push(node);
    out.push([
      node.tagName,
      node.getAttribute('role') || '-',
      encodeURIComponent(named(node).slice(0, 80)) || '-',
      Math.round(rect.left), Math.round(rect.top),
      Math.round(rect.width), Math.round(rect.height),
      depth, parent
    ].join(' '));
  }}
  document.getElementById('{SINK}').textContent = out.join(' ; ');
}}, {SETTLE_MS});
</script>
"#
    )
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
