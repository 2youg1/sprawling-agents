// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The engine half of the render gate: find a browser, open the gallery
//! out of the bundle a person runs, and read back what was drawn.

pub(crate) mod launch;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use super::pass::{HEIGHT, Pass, Reported};
use super::probe::{CONDITIONS, DECLARED, FAILED, SINK, declared, script};
use crate::report::XtaskError;
use crate::walk;
use browser::survey::probe::Read;
use browser::survey::{Page, PaintSource};

/// Where the instrumented copy and the throwaway browser profile go.
const WORK: &str = "target/render";

/// The engine switch that puts a page in a forced-colour mode.
///
/// Verified on this repository's own engine rather than taken from a
/// document: run headless with it and `(forced-colors: active)` matches,
/// run without it and it does not. The alternative - the devtools
/// protocol's media emulation - needs a socket, and section 8-13 already
/// settled that this gate opens no socket.
const FORCED_COLOURS: &str = "--force-high-contrast";

/// How long the engine is given to reach the moment the probe measures
/// at, in the same virtual time the probe's own wait is counted in.
pub(crate) const BUDGET_MS: u32 = 8000;

/// How long an opening's engine may run before the gate calls it stuck:
/// two minutes, fifteen times the virtual budget, so only an engine that
/// will never finish reaches it.
const PATIENCE: Duration = Duration::from_secs(120);

/// The Chromium family as the doctor states it: one brand per line,
/// with the program a shell resolves and where each platform installs
/// it. Rendered by the doctor's own test from its family table, so the
/// gate and the doctor look in the same places (section 8-13).
const CHROMIUM: &str = include_str!("../../../../crates/sprawling/src/doctor/family/chromium.tsv");

/// The engine to render with, in the three tiers section 8-13 fixes.
///
/// Each tier is an authority that already exists. The gate does not keep
/// a list of its own, and it does not probe for what `doctor` already
/// knows.
pub(crate) fn browser() -> Option<PathBuf> {
    if let Some(named) = std::env::var_os("SPRAWLING_BROWSER") {
        let path = PathBuf::from(named);
        if path.is_file() {
            return Some(path);
        }
    }
    if let Some(installed) = in_components() {
        return Some(installed);
    }
    chromium_members().find_map(|(program, places)| {
        places
            .map(PathBuf::from)
            .find(|path| path.is_file())
            .or_else(|| on_path(program))
    })
}

/// Each brand of the doctor's Chromium family, in its order: the
/// program name, and the places this platform installs it.
fn chromium_members() -> impl Iterator<Item = (&'static str, std::str::Split<'static, char>)> {
    let column = if cfg!(windows) {
        2
    } else if cfg!(target_os = "macos") {
        3
    } else {
        4
    };
    CHROMIUM
        .lines()
        .filter(|line| !line.starts_with('#'))
        .filter_map(move |line| {
            let fields: Vec<&'static str> = line.split('\t').collect();
            let program = fields.get(1)?;
            let places = fields.get(column)?;
            Some((*program, places.split('|')))
        })
}

/// What `doctor` installed, read as the filesystem convention it is
/// (`crates/kernel/Spec.lean` §8-22) rather than as a second probe for
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

/// Where `program` is on `PATH`, under its own name or, on Windows,
/// with the `.exe` a shell adds.
fn on_path(program: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    std::env::split_paths(&paths)
        .flat_map(|dir| [dir.join(program), dir.join(format!("{program}.exe"))])
        .find(|candidate| candidate.is_file())
}

/// Open one route of the built client and read back every element the
/// properties are about.
///
/// The bundle is copied and instrumented rather than probed in place: the
/// probe is a `<script>` this repository does not ship, and writing it
/// into the bundle directory would leave it in whatever `just dist`
/// packages next.
/// Where one opening finds what it opens: the same four paths for every
/// pass of a run.
pub(super) struct Opening<'a> {
    pub(super) root: &'a Path,
    pub(super) browser: &'a Path,
    pub(super) bundle: &'a Path,
    pub(super) route: &'a str,
}

/// What one opening read back: the page as the instrument measures it,
/// and what the page said about the conditions it drew in.
pub(super) struct Measured {
    pub(super) page: Page,
    pub(super) reported: Reported,
}

pub(super) fn measure(opening: &Opening, pass: &Pass) -> Result<Measured, XtaskError> {
    let Opening {
        root,
        browser,
        bundle,
        route,
    } = *opening;
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
    let chunks = walk::files_with_ext(&bundle.join("assets"), &["js"])?;
    let names = chunks
        .iter()
        .filter_map(|chunk| chunk.file_name().and_then(|name| name.to_str()));
    let preloaded = preloads(&body, names);
    std::fs::write(&instrumented, instrument(&body, &preloaded, pass)).map_err(|source| {
        XtaskError::Io {
            path: walk::rel(root, &instrumented),
            source,
        }
    })?;
    let profile = work.join("profile");
    let mut command = Command::new(browser);
    if pass.draws_forced_colours() {
        command.arg(FORCED_COLOURS);
    }
    command
        .arg("--headless=new")
        .arg("--disable-gpu")
        .arg("--no-sandbox")
        .arg("--hide-scrollbars")
        // On macOS an engine that reaches for the login keychain waits for
        // a prompt nobody on a runner can answer.
        .arg("--use-mock-keychain")
        // The bundle is a module script, and a module fetched from
        // `file://` is a cross-origin fetch. Serving it would mean a
        // server in the gate; this flag is the same statement without
        // one, and it applies to a throwaway profile.
        .arg("--allow-file-access-from-files")
        .arg(format!("--user-data-dir={}", profile.display()))
        .arg(format!("--window-size={},{HEIGHT}", pass.width))
        .arg(format!("--virtual-time-budget={BUDGET_MS}"))
        .arg("--dump-dom")
        .arg(format!("{}#/{route}", url_of(&instrumented)));
    let dumped = launch::run(
        command,
        &launch::Launch {
            cmd: &format!("{} --dump-dom {route}", browser.display()),
            stderr: launch::Stderr::Discarded,
            patience: PATIENCE,
            after_exit: launch::AfterExit::Drain,
        },
    );
    let _ = std::fs::remove_file(&instrumented);
    let dumped = dumped?.stdout;
    let dom = String::from_utf8_lossy(&dumped);
    if let Some(thrown) = sink(&dom, FAILED).filter(|said| !said.trim().is_empty()) {
        return Err(XtaskError::Cmd {
            cmd: format!("{} --dump-dom {route}", browser.display()),
            msg: format!("the page failed before the probe could measure it: {thrown}"),
        });
    }
    let (Some(records), Some(conditions), Some(words)) = (
        sink(&dom, SINK),
        sink(&dom, CONDITIONS),
        sink(&dom, DECLARED),
    ) else {
        return Err(XtaskError::Cmd {
            cmd: format!("{} --dump-dom {route}", browser.display()),
            msg: "the probe wrote nothing; the engine rendered no page or ran no script".to_owned(),
        });
    };
    let vocabulary = declared(words);
    if vocabulary.is_empty() {
        // Fail closed: an empty vocabulary would read as a page that
        // declares nothing, and every colour on it would be reported as
        // undeclared. A gate that cannot find its subject must say so
        // rather than judge against nothing.
        return Err(XtaskError::Cmd {
            cmd: format!("{} --dump-dom {route}", browser.display()),
            msg: "the probe read no declared words off the root element; the engine does not \
                  enumerate custom properties, so nothing can be judged against them"
                .to_owned(),
        });
    }
    let mut said = conditions.split_whitespace();
    let (Some(forced), Some(scheme)) = (said.next(), said.next()) else {
        return Err(XtaskError::Cmd {
            cmd: format!("{} --dump-dom {route}", browser.display()),
            msg: format!(
                "the probe wrote `{conditions}`, which names neither a forced-colour \
                 mode nor a colour scheme"
            ),
        });
    };
    // The pass demanded the paint source and the page is asserted
    // against it below; a resident surveying a page it did not ask for
    // reads the same three strings and takes the page's own word.
    let read = Read {
        sink: records.to_owned(),
        declared: words.to_owned(),
        conditions: conditions.to_owned(),
    };
    Ok(Measured {
        page: read.page(if pass.draws_forced_colours() {
            PaintSource::TheSystem
        } else {
            PaintSource::ThePage
        }),
        reported: Reported::read(forced, scheme),
    })
}

/// What the probe left in one of its two elements, if it ran.
fn sink<'a>(dom: &'a str, id: &str) -> Option<&'a str> {
    let open = format!("<pre id=\"{id}\">");
    let start = dom.find(&open)?.checked_add(open.len())?;
    let rest = dom.get(start..)?;
    let end = rest.find("</pre>")?;
    rest.get(..end)
}

/// A `modulepreload` link for every script chunk in the bundle that the
/// page does not name itself, which is every chunk a view fetches through
/// a dynamic `import()`.
///
/// A module fetched from `file://` after the document has loaded does not
/// hold the engine's virtual time, so a lazily loaded view never lands
/// inside the budget however long the probe waits. A preload is part of
/// the document load, which does hold virtual time; the later `import()`
/// then finds the module already fetched. Only the gate's copy carries
/// these links, so the shipped page stays lazy.
pub(crate) fn preloads<'a>(body: &str, chunks: impl IntoIterator<Item = &'a str>) -> String {
    chunks
        .into_iter()
        .filter(|chunk| !body.contains(&format!("./assets/{chunk}")))
        .map(|chunk| {
            format!(
                "<link rel=\"modulepreload\" crossorigin href=\"./assets/{chunk}\">
"
            )
        })
        .collect()
}

/// Add the preloads to the head and append the probe to a copy of the page.
fn instrument(body: &str, preloaded: &str, pass: &Pass) -> String {
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
            format!("{head}{}{tail}", script(pass))
        }
        None => format!("{body}{}", script(pass)),
    }
}

/// A `file://` URL for a path, in the form every engine accepts.
pub(crate) fn url_of(path: &Path) -> String {
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

#[cfg(test)]
mod tests {
    use super::preloads;

    #[test]
    fn a_chunk_the_page_does_not_load_is_preloaded() {
        let body = r#"<script type="module" crossorigin src="./assets/index-A.js"></script>"#;
        assert_eq!(
            preloads(body, ["gallery-B.js", "index-A.js"]),
            "<link rel=\"modulepreload\" crossorigin href=\"./assets/gallery-B.js\">
"
        );
    }
}
