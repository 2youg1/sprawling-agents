// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

pub(crate) mod launch;
pub(super) mod served;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use super::pass::{HEIGHT, Pass, Reported};
use super::probe::{CONDITIONS, DECLARED, FAILED, SINK, declared, script};
use crate::report::XtaskError;
use crate::walk;
use browser::survey::probe::Read;
use browser::survey::{Page, PaintSource};

const WORK: &str = "target/render";

const FORCED_COLOURS: &str = "--force-high-contrast";

pub(crate) const BUDGET_MS: u32 = 8000;

const PATIENCE: Duration = Duration::from_secs(300);
const PROFILE_RELEASE: Duration = Duration::from_secs(5);

const CHROMIUM: &str = include_str!("../../../../Sprawling/app/src/doctor/family/chromium.tsv");

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
            .filter_map(installation_path)
            .find(|path| path.is_file())
            .or_else(|| on_path(program))
    })
}

fn installation_path(place: &str) -> Option<PathBuf> {
    let path = PathBuf::from(place);
    if path.is_absolute() {
        return Some(path);
    }
    let relative = place.strip_prefix(r"%LOCALAPPDATA%\")?;
    let user = PathBuf::from(std::env::var_os("LOCALAPPDATA")?);
    user.is_absolute().then(|| user.join(relative))
}

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

fn on_path(program: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    std::env::split_paths(&paths)
        .flat_map(|dir| [dir.join(program), dir.join(format!("{program}.exe"))])
        .find(|candidate| candidate.is_file())
}

pub(super) struct Opening<'a> {
    pub(super) root: &'a Path,
    pub(super) browser: &'a Path,
    pub(super) bundle: &'a Path,
    pub(super) route: &'a str,
}

pub(super) struct Measured {
    pub(super) page: Page,
    pub(super) reported: Reported,
}

#[expect(clippy::disallowed_methods, reason = "developer tool (child D4)")]
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
    let profile = work.join(format!("profile-{}-{}", pass.slug(), std::process::id()));
    released(&profile)?;

    let instrumented = bundle.join(format!(
        "sprawling-render-{}-{}.html",
        std::process::id(),
        pass.slug()
    ));
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
    let mut command = Command::new(browser);
    if pass.draws_forced_colours() {
        command.arg(FORCED_COLOURS);
    }
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
    let cleaned = clean_up(&profile, &instrumented);
    let dumped = match (dumped, cleaned) {
        (Ok(done), Ok(())) => done.stdout,
        (Err(err), Ok(())) => return Err(err),
        (Ok(_), Err(err)) => return Err(err),
        (Err(engine), Err(cleanup)) => {
            return Err(XtaskError::Cmd {
                cmd: format!("{} --dump-dom {route}", browser.display()),
                msg: format!("{engine}; cleaning the per-run profile also failed: {cleanup}"),
            });
        }
    };
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
    read_measured(
        Read {
            sink: records.to_owned(),
            declared: words.to_owned(),
            conditions: conditions.to_owned(),
        },
        pass,
        &format!("{} --dump-dom {route}", browser.display()),
    )
}

pub(super) fn read_measured(
    read: Read,
    pass: &Pass,
    command: &str,
) -> Result<Measured, XtaskError> {
    let vocabulary = declared(&read.declared);
    if vocabulary.is_empty() {
        return Err(XtaskError::Cmd {
            cmd: command.to_owned(),
            msg: "the probe read no declared words off the root element; the engine does not \
                  enumerate custom properties, so nothing can be judged against them"
                .to_owned(),
        });
    }
    let Some((forced, scheme)) = read.conditions() else {
        return Err(XtaskError::Cmd {
            cmd: command.to_owned(),
            msg: format!(
                "the probe wrote `{}`, which names neither a forced-colour \
                 mode nor a colour scheme",
                read.conditions,
            ),
        });
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

fn clean_up(profile: &Path, instrumented: &Path) -> Result<(), XtaskError> {
    let mut failed = Vec::new();
    if let Err(err) = remove_file(instrumented) {
        failed.push(err.to_string());
    }
    if let Err(err) = released(profile) {
        failed.push(err.to_string());
    }
    if failed.is_empty() {
        return Ok(());
    }
    Err(XtaskError::Cmd {
        cmd: "clean up a render run".to_owned(),
        msg: failed.join("; "),
    })
}

fn remove_file(path: &Path) -> Result<(), XtaskError> {
    std::fs::remove_file(path).or_else(|source| {
        if source.kind() == std::io::ErrorKind::NotFound {
            Ok(())
        } else {
            Err(XtaskError::Io {
                path: path.display().to_string(),
                source,
            })
        }
    })
}

fn released(profile: &Path) -> Result<(), XtaskError> {
    let pause = Duration::from_millis(250);
    let mut waited = Duration::ZERO;
    loop {
        match std::fs::remove_dir_all(profile) {
            Ok(()) => return Ok(()),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(source) if waited >= PROFILE_RELEASE => {
                return Err(XtaskError::Io {
                    path: profile.display().to_string(),
                    source,
                });
            }
            Err(_) => {
                std::thread::sleep(pause);
                waited = waited.saturating_add(pause);
            }
        }
    }
}

fn sink<'a>(dom: &'a str, id: &str) -> Option<&'a str> {
    let open = format!("<pre id=\"{id}\">");
    let start = dom.find(&open)?.checked_add(open.len())?;
    let rest = dom.get(start..)?;
    let end = rest.find("</pre>")?;
    rest.get(..end)
}

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
