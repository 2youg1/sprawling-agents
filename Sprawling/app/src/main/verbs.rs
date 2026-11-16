// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Verb {
    Mei,
    Up,
    Init,
    Serve,
    Resume,
    Call,
    Dispatch,
    Gauge,
    Enrol,
    Whose,
    Check,
    View,
    Secrets,
    PlaybackExport,
    PlaybackCheck,
    Fork,
    Adopt,
    Replay,
    Export,
    Restore,
    Doctor,
    Desktop,
    Install,
    Status,
    PrivacyStatus,
    PrivacyInspect,
    PrivacyApply,
    PrivacyRestore,
    PrivacyRestoreAll,
    PrivacyReconcile,
    PrivacyElevatedWrite,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Effect {
    ReadsOnly,
    Changes,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Takes {
    Nothing,
    Value(&'static str),

    Level,
}

#[derive(Debug)]
pub(super) struct Flag {
    pub(super) name: &'static str,

    pub(super) short: Option<&'static str>,
    pub(super) takes: Takes,
    pub(super) says: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AfterDashes {
    Refused,
    Command,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Need {
    Required,
    Optional,
}

#[derive(Debug)]
pub(super) struct Row {
    pub(super) verb: Verb,
    pub(super) name: &'static str,
    pub(super) aliases: &'static [&'static str],
    pub(super) positionals: &'static [(&'static str, Need)],
    pub(super) flags: &'static [Flag],
    pub(super) says: &'static str,
    pub(super) effect: Effect,
    pub(super) after_dashes: AfterDashes,
}

pub(super) const fn flag(name: &'static str, takes: Takes, says: &'static str) -> Flag {
    Flag {
        name,
        short: None,
        takes,
        says,
    }
}

use Need::{Optional, Required};
use Takes::{Level, Nothing, Value};

#[path = "verbs/early.rs"]
mod early;
#[path = "verbs/later.rs"]
mod later;

pub(super) const MEI_REPLAY: Flag = flag(
    "--replay",
    Value("jsonl"),
    "display a saved Mei stderr stream without starting a model",
);
pub(super) const LOG: Flag = flag("--log", Level, "the floor of what the diagnostics write");
pub(super) const OPEN: Flag = flag("--open", Nothing, "open the WebUI once the port answers");
pub(super) const NO_OPEN: Flag = flag(
    "--no-open",
    Nothing,
    "leave the screen alone (so does SPRAWLING_OPEN=never)",
);
pub(super) const AT: Flag = flag("--at", Value("addr"), "the served city to talk to");
pub(super) const JSON: Flag = flag("--json", Nothing, "write a refusal as one line of json");

pub(super) const INCLUDE_CONFIDENTIAL: Flag = flag(
    "--include-confidential",
    Nothing,
    "read confidential buildings too; the bundle and stderr say so",
);

pub(super) const SERVED: &[Flag] = &[
    OPEN,
    NO_OPEN,
    flag("--console", Nothing, "open the CLI in this terminal"),
    flag(
        "--no-console",
        Nothing,
        "leave this terminal out of the city",
    ),
    flag(
        "--supervise",
        Nothing,
        "serve in a child process, resume and serve again after a crash",
    ),
    LOG,
    flag(
        "--web-dir",
        Value("dir"),
        "read the client from <dir> on every request",
    ),
];

pub(super) fn forwarded(args: &[String]) -> Vec<String> {
    let decided = [
        "--supervise",
        "--open",
        "--no-open",
        "--console",
        "--no-console",
    ];
    let mut kept = Vec::new();
    let mut words = args.iter();
    while let Some(word) = words.next() {
        let Some(flag) = SERVED.iter().find(|flag| flag.name == word) else {
            continue;
        };
        if decided.contains(&flag.name) {
            continue;
        }
        kept.push(word.clone());
        match flag.takes {
            Takes::Value(_) | Takes::Level => kept.extend(words.next().cloned()),
            Takes::Nothing => {}
        }
    }
    kept
}

pub(super) const VERBS: &[Row] = &[
    early::MEI,
    early::PRIVACY_STATUS,
    early::PRIVACY_INSPECT,
    early::PRIVACY_APPLY,
    early::PRIVACY_RESTORE,
    early::PRIVACY_RESTORE_ALL,
    early::PRIVACY_RECONCILE,
    early::PRIVACY_ELEVATED_WRITE,
    early::UP,
    early::INIT,
    early::SERVE,
    early::RESUME,
    later::CALL,
    later::DISPATCH,
    later::GAUGE,
    later::ENROL,
    later::WHOSE,
    later::CHECK,
    later::SECRETS,
    later::VIEW,
    later::PLAYBACK_EXPORT,
    later::PLAYBACK_CHECK,
    later::FORK,
    later::ADOPT,
    later::REPLAY,
    later::EXPORT,
    later::RESTORE,
    later::DOCTOR,
    later::DESKTOP,
    later::INSTALL,
    later::STATUS,
];

pub(super) fn row(verb: Verb) -> Option<&'static Row> {
    VERBS.iter().find(|row| row.verb == verb)
}

pub(super) fn usage(row: &Row) -> String {
    let positionals = row.positionals.iter().map(|(name, need)| match need {
        Required => format!(" <{name}>"),
        Optional => format!(" [{name}]"),
    });
    let flags = row.flags.iter().map(|flag| match flag.takes {
        Nothing => format!(" [{}]", spelled(flag)),
        Value(what) => format!(" [{} <{what}>]", spelled(flag)),
        Level => format!(" [{} <level>]", spelled(flag)),
    });
    let command = match row.after_dashes {
        AfterDashes::Refused => "",
        AfterDashes::Command => " [-- <program> [arg...]]",
    };
    let name = row.name;
    format!(
        "sprawling {name}{}{command}",
        positionals.chain(flags).collect::<String>()
    )
}

pub(super) fn help(row: &Row) -> String {
    let flags: String = row
        .flags
        .iter()
        .map(|flag| match flag.takes {
            Level => format!(
                "\n  {:<16}{}: {}",
                flag.name,
                flag.says,
                super::router::level_names()
            ),
            Nothing | Value(_) => format!("\n  {:<16}{}", flag.name, flag.says),
        })
        .collect();
    let effect = match row.effect {
        Effect::ReadsOnly => "reads only; changes nothing",
        Effect::Changes => "changes a city or this machine",
    };
    format!("usage: {}\n\n{} ({effect}){flags}", usage(row), row.says)
}

pub(super) fn overview() -> String {
    let width = VERBS
        .iter()
        .map(|row| row.name.chars().count())
        .max()
        .unwrap_or(0)
        .saturating_add(2);
    let lines: String = VERBS
        .iter()
        .map(|row| {
            let name = row.name;
            format!("\n  {name:<width$}{}", row.says)
        })
        .collect();
    format!("commands:{lines}\n\nsprawling help <verb> explains one.")
}

fn spelled(flag: &Flag) -> String {
    match flag.short {
        Some(short) => format!("{short}/{}", flag.name),
        None => flag.name.to_owned(),
    }
}
