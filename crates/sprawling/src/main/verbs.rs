// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The command table: every verb this binary accepts, the positional
//! arguments and flags it reads, one line about what it does, and
//! whether running it changes anything (sprawling-SPEC.md 8-89).
//!
//! The parser (`grammar`), the overview a person reads, and each verb's
//! own help all read this table, so a flag cannot be accepted in one
//! place and undocumented in another.

/// A verb this binary carries out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Verb {
    Up,
    Init,
    Serve,
    Resume,
    Call,
    Enrol,
    Whose,
    Check,
    Fork,
    Adopt,
    Replay,
    Export,
    Restore,
    Doctor,
    Install,
    Status,
}

/// Whether running a verb can change a city or this machine. `--help`
/// and `--version` answer before any verb runs, which is what keeps a
/// verb marked `Changes` from acting on a request for its help.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Effect {
    ReadsOnly,
    Changes,
}

/// Whether a flag carries the word after it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Takes {
    Nothing,
    Value(&'static str),
}

/// One flag a verb reads.
#[derive(Debug)]
pub(super) struct Flag {
    pub(super) name: &'static str,
    pub(super) takes: Takes,
    pub(super) says: &'static str,
}

/// Whether a positional argument may be left out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Need {
    Required,
    Optional,
}

/// One row of the table.
#[derive(Debug)]
pub(super) struct Row {
    pub(super) verb: Verb,
    pub(super) name: &'static str,
    pub(super) aliases: &'static [&'static str],
    pub(super) positionals: &'static [(&'static str, Need)],
    pub(super) flags: &'static [Flag],
    pub(super) says: &'static str,
    pub(super) effect: Effect,
}

const fn flag(name: &'static str, takes: Takes, says: &'static str) -> Flag {
    Flag { name, takes, says }
}

use Need::{Optional, Required};
use Takes::{Nothing, Value};

const LOG: Flag = flag(
    "--log",
    Value("level"),
    "trace, debug, effect, notice, warn, or off",
);
const OPEN: Flag = flag("--open", Nothing, "open the WebUI once the port answers");
const NO_OPEN: Flag = flag(
    "--no-open",
    Nothing,
    "leave the screen alone (so does SPRAWLING_OPEN=never)",
);
const AT: Flag = flag("--at", Value("addr"), "the served city to talk to");
/// `up` forwards its line to the same `serve_city` that `serve` runs, so the
/// two rows share one flag set and cannot drift apart.
const SERVED: &[Flag] = &[
    OPEN,
    NO_OPEN,
    flag("--console", Nothing, "enter the city's console"),
    flag("--no-console", Nothing, "do not enter the console"),
    LOG,
    flag(
        "--web-dir",
        Value("dir"),
        "read the client from <dir> on every request",
    ),
];

/// The table. Its order is the order the overview prints.
pub(super) const VERBS: &[Row] = &[
    Row {
        verb: Verb::Up,
        name: "up",
        aliases: &[],
        positionals: &[("city", Optional), ("addr", Optional)],
        flags: SERVED,
        says: "raise a city here if needed, serve it, open the WebUI",
        effect: Effect::Changes,
    },
    Row {
        verb: Verb::Init,
        name: "init",
        aliases: &[],
        positionals: &[("city", Required)],
        flags: &[flag(
            "--adopt",
            Nothing,
            "every folder already there becomes a building",
        )],
        says: "raise a city: writes the genesis record",
        effect: Effect::Changes,
    },
    Row {
        verb: Verb::Serve,
        name: "serve",
        aliases: &[],
        positionals: &[("city", Required), ("addr", Optional)],
        flags: SERVED,
        says: "serve a city that already exists",
        effect: Effect::Changes,
    },
    Row {
        verb: Verb::Resume,
        name: "resume",
        aliases: &[],
        positionals: &[("city", Required)],
        flags: &[],
        says: "after a restart: verify, close what was lost, report",
        effect: Effect::Changes,
    },
    Row {
        verb: Verb::Call,
        name: "call",
        aliases: &[],
        positionals: &[("frame|-", Optional)],
        flags: &[
            AT,
            flag("--token", Value("token"), "the pairing token"),
            flag(
                "--quiet-ms",
                Value("n"),
                "how long a silence ends the answer",
            ),
        ],
        says: "send one wire frame, print every frame back",
        effect: Effect::Changes,
    },
    Row {
        verb: Verb::Enrol,
        name: "enrol",
        aliases: &["enroll"],
        positionals: &[("realm/name", Required)],
        flags: &[AT],
        says: "read a credential from stdin, hand it to a city",
        effect: Effect::Changes,
    },
    Row {
        verb: Verb::Whose,
        name: "whose",
        aliases: &[],
        positionals: &[("city", Required), ("oid", Required)],
        flags: &[],
        says: "which run wrote a commit this city made",
        effect: Effect::ReadsOnly,
    },
    Row {
        verb: Verb::Check,
        name: "check",
        aliases: &[],
        positionals: &[("city", Required)],
        flags: &[],
        says: "read every TOML file a city holds; print each error as path:line:column",
        effect: Effect::ReadsOnly,
    },
    Row {
        verb: Verb::Fork,
        name: "fork",
        aliases: &[],
        positionals: &[
            ("city", Required),
            ("run", Required),
            ("seq", Required),
            ("addr", Required),
        ],
        flags: &[],
        says: "branch a lineage from one step of a run",
        effect: Effect::Changes,
    },
    Row {
        verb: Verb::Adopt,
        name: "adopt",
        aliases: &[],
        positionals: &[("city", Required), ("addr", Required)],
        flags: &[],
        says: "take an existing directory in as a building",
        effect: Effect::Changes,
    },
    Row {
        verb: Verb::Replay,
        name: "replay",
        aliases: &[],
        positionals: &[("ledger-dir", Required)],
        flags: &[],
        says: "verify a chain offline, read-only",
        effect: Effect::ReadsOnly,
    },
    Row {
        verb: Verb::Export,
        name: "export",
        aliases: &[],
        positionals: &[("city", Required), ("bundle-dir", Required)],
        flags: &[],
        says: "pack a whole city",
        effect: Effect::ReadsOnly,
    },
    Row {
        verb: Verb::Restore,
        name: "restore",
        aliases: &[],
        positionals: &[("bundle-dir", Required), ("city", Required)],
        flags: &[],
        says: "unpack a bundle on another machine",
        effect: Effect::Changes,
    },
    Row {
        verb: Verb::Doctor,
        name: "doctor",
        aliases: &[],
        positionals: &[("city", Optional)],
        flags: &[
            flag(
                "--install",
                Nothing,
                "offer each missing item, one at a time",
            ),
            flag(
                "--explain",
                Value("code"),
                "connect a refusal code to this machine",
            ),
            flag("--no-color", Nothing, "print without colour"),
        ],
        says: "what this machine has against what a city needs",
        effect: Effect::ReadsOnly,
    },
    Row {
        verb: Verb::Install,
        name: "install",
        aliases: &[],
        positionals: &[],
        flags: &[flag(
            "--uninstall",
            Nothing,
            "take this binary back off PATH",
        )],
        says: "put this binary on your PATH",
        effect: Effect::Changes,
    },
    Row {
        verb: Verb::Status,
        name: "status",
        aliases: &["version"],
        positionals: &[],
        flags: &[
            flag("--deps", Nothing, "every crate this binary is built from"),
            flag(
                "--check",
                Nothing,
                "ask npm whether a newer release exists (reaches the network)",
            ),
            LOG,
        ],
        says: "this binary: version, client, what it is built from",
        effect: Effect::ReadsOnly,
    },
];

/// The row a verb owns. Every `Verb` has exactly one row; a verb with
/// none reads as a verb with no arguments, which the parser then refuses.
pub(super) fn row(verb: Verb) -> Option<&'static Row> {
    VERBS.iter().find(|row| row.verb == verb)
}

/// One verb's usage line: `sprawling serve <city> [addr] [--open] …`.
pub(super) fn usage(row: &Row) -> String {
    let positionals = row.positionals.iter().map(|(name, need)| match need {
        Required => format!(" <{name}>"),
        Optional => format!(" [{name}]"),
    });
    let flags = row.flags.iter().map(|flag| match flag.takes {
        Nothing => format!(" [{}]", flag.name),
        Value(what) => format!(" [{} <{what}>]", flag.name),
    });
    let name = row.name;
    format!(
        "sprawling {name}{}",
        positionals.chain(flags).collect::<String>()
    )
}

/// One verb's help: its usage, what it does, one line per flag.
pub(super) fn help(row: &Row) -> String {
    let flags: String = row
        .flags
        .iter()
        .map(|flag| format!("\n  {:<16}{}", flag.name, flag.says))
        .collect();
    let effect = match row.effect {
        Effect::ReadsOnly => "reads only; changes nothing",
        Effect::Changes => "changes a city or this machine",
    };
    format!("usage: {}\n\n{} ({effect}){flags}", usage(row), row.says)
}

/// Every verb, one line each.
pub(super) fn overview() -> String {
    let lines: String = VERBS
        .iter()
        .map(|row| {
            let name = row.name;
            format!("\n  {name:<9}{}", row.says)
        })
        .collect();
    format!("commands:{lines}\n\nsprawling help <verb> explains one.")
}
