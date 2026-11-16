// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

mod pattern;

use crate::refusal::{Refusal, RefusalCode};
use crate::tools::ToolName;

pub(crate) use pattern::Pattern;
use serde::Deserialize;
use std::path::Path;

pub(crate) struct Reach<'a> {
    pub(crate) tool: ToolName,
    pub(crate) title: Option<&'a str>,
    pub(crate) process: Option<&'a str>,
}

const fn names_a_window(tool: ToolName) -> bool {
    match tool {
        ToolName::Snapshot | ToolName::Act | ToolName::Screenshot | ToolName::Record => true,
        ToolName::Windows | ToolName::Clipboard => false,
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScopeFile {
    #[serde(default)]
    windows: Vec<String>,
    #[serde(default)]
    processes: Vec<String>,
    #[serde(default)]
    record: bool,
    #[serde(default)]
    clipboard: bool,

    #[serde(default)]
    sound: Option<String>,
}

pub(crate) enum Scope {

    Closed {
        code: RefusalCode,
        because: String,
    },
    Open(Allowance),
}

#[derive(Debug)]
pub(crate) struct Allowance {
    windows: Vec<Pattern>,
    processes: Vec<Pattern>,
    record: bool,
    clipboard: bool,
    #[cfg_attr(
        not(any(windows, test)),
        expect(
            dead_code,
            reason = "only the Windows arm records, so only it reads the device back"
        )
    )]
    sound: Option<String>,
}

#[derive(Debug)]
pub(crate) struct Admitted<'a> {
    #[cfg_attr(
        not(any(windows, test)),
        expect(
            dead_code,
            reason = "only the Windows arm reports windows, so only it reads the allowlist back (`Sprawling/desktop/Spec.lean` D1)"
        )
    )]
    allowance: &'a Allowance,
}

impl Admitted<'_> {

    #[cfg(any(windows, test))]
    pub(crate) fn visible(&self, title: &str, process: &str) -> bool {
        self.allowance.visible(title, process)
    }

    #[cfg(any(windows, test))]
    pub(crate) fn sound(&self) -> Result<&str, Refusal> {
        self.allowance.sound.as_deref().ok_or_else(|| {
            Refusal::new(
                RefusalCode::GateDenied,
                "record a window",
                "this scope names no sound device, and this server does not choose one",
                "name the device in DESKTOP.toml as `sound = \"<device>\"`; `ffmpeg \
                 -hide_banner -list_devices true -f dshow -i dummy` lists the names this \
                 machine has",
            )
        })
    }
}

impl Scope {

    pub(crate) fn read(path: Option<&Path>) -> Scope {
        let Some(path) = path else {
            return Scope::Closed {
                code: RefusalCode::GateDenied,
                because: "no scope file was named".to_owned(),
            };
        };
        match std::fs::read_to_string(path) {
            Ok(text) => Scope::parse(&text),
            Err(err) => Scope::Closed {
                code: RefusalCode::GateDenied,
                because: format!("{}: {err}", path.display()),
            },
        }
    }

    pub(crate) fn parse(text: &str) -> Scope {
        match toml::from_str::<ScopeFile>(text) {
            Ok(file) => Scope::Open(Allowance {
                windows: file.windows.iter().map(|line| Pattern::new(line)).collect(),
                processes: file
                    .processes
                    .iter()
                    .map(|line| Pattern::new(line))
                    .collect(),
                record: file.record,
                clipboard: file.clipboard,
                sound: file.sound,
            }),
            Err(err) => Scope::Closed {
                code: RefusalCode::ConfigInvalid,
                because: format!("the scope file cannot be read: {err}"),
            },
        }
    }

    pub(crate) fn admits(&self, reach: &Reach<'_>) -> Result<Admitted<'_>, Refusal> {
        match self {
            Scope::Closed { code, because } => Err(Refusal::new(
                *code,
                "use the desktop",
                format!("this server has no scope: {because}"),
                "write a DESKTOP.toml listing the windows this server may touch, and give its \
                 path as this server's first argument",
            )),
            Scope::Open(allowance) => allowance.admits(reach),
        }
    }
}

impl Allowance {
    fn admits(&self, reach: &Reach<'_>) -> Result<Admitted<'_>, Refusal> {
        let admitted = Admitted { allowance: self };
        if reach.tool == ToolName::Record && !self.record {
            return Err(switched_off("record"));
        }
        if reach.tool == ToolName::Clipboard {
            if self.clipboard {
                return Ok(admitted);
            }
            return Err(switched_off("clipboard"));
        }
        if !names_a_window(reach.tool) {
            return Ok(admitted);
        }
        match (reach.title, reach.process) {
            (None, None) => Err(Refusal::new(
                RefusalCode::GateDenied,
                "use the desktop",
                format!(
                    "`{}` was asked for without naming a window",
                    reach.tool.as_str()
                ),
                "name the window by its `title` or its `process`; this scope lists windows, so \
                 the whole screen would show what it leaves out",
            )),
            (title, process) => {
                if let Some(title) = title {
                    admitted_by(&self.windows, title, "title")?;
                }
                if let Some(process) = process {
                    admitted_by(&self.processes, process, "process")?;
                }
                Ok(admitted)
            }
        }
    }

    #[cfg(any(windows, test))]
    fn visible(&self, title: &str, process: &str) -> bool {
        self.windows.iter().any(|glob| glob.matches(title))
            || self.processes.iter().any(|glob| glob.matches(process))
    }
}

fn switched_off(switch: &str) -> Refusal {
    Refusal::new(
        RefusalCode::GateDenied,
        "use the desktop",
        format!("this scope does not switch `{switch}` on"),

        &format!("set `{switch} = true` in DESKTOP.toml if this machine's operator wants it"),
    )
}

fn admitted_by(allowed: &[Pattern], named: &str, kind: &str) -> Result<(), Refusal> {
    if allowed.iter().any(|glob| glob.matches(named)) {
        return Ok(());
    }
    Err(Refusal::new(
        RefusalCode::GateDenied,
        "use the desktop",
        format!("no {kind} pattern in this scope matches `{named}`"),
        "add a pattern for it to DESKTOP.toml, or work on a window this scope already lists",
    ))
}

#[cfg(test)]
mod tests;
