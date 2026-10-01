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
    Dispatch,
    Gauge,
    Enrol,
    Whose,
    Check,
    View,
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
    /// The one-letter spelling a hand types often, read as `name`.
    pub(super) short: Option<&'static str>,
    pub(super) takes: Takes,
    pub(super) says: &'static str,
}

/// Whether a verb takes the words after the first `--` as a command of
/// its own, handed over without reading any of them as a flag
/// (sprawling-SPEC.md 8-129-4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AfterDashes {
    Refused,
    Command,
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
    pub(super) after_dashes: AfterDashes,
}

const fn flag(name: &'static str, takes: Takes, says: &'static str) -> Flag {
    Flag {
        name,
        short: None,
        takes,
        says,
    }
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
/// The person's one way to widen a playback bundle, on both verbs, so an
/// export and its check read as the same person.
const INCLUDE_CONFIDENTIAL: Flag = flag(
    "--include-confidential",
    Nothing,
    "read confidential buildings too; the bundle and stderr say so",
);
/// `up` forwards its line to the same `serve_city` that `serve` runs, so the
/// two rows share one flag set and cannot drift apart.
const SERVED: &[Flag] = &[
    OPEN,
    NO_OPEN,
    flag("--console", Nothing, "enter the city's console"),
    flag("--no-console", Nothing, "do not enter the console"),
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

/// The flags of a served line a supervised child receives as given, each
/// with the value `SERVED` says it takes; the four the supervisor decides
/// again for every child, and `--supervise` itself, are left out.
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
        if let Takes::Value(_) = flag.takes {
            kept.extend(words.next().cloned());
        }
    }
    kept
}

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
        after_dashes: AfterDashes::Refused,
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
        after_dashes: AfterDashes::Refused,
    },
    Row {
        verb: Verb::Serve,
        name: "serve",
        aliases: &[],
        positionals: &[("city", Required), ("addr", Optional)],
        flags: SERVED,
        says: "serve a city that already exists",
        effect: Effect::Changes,
        after_dashes: AfterDashes::Refused,
    },
    Row {
        verb: Verb::Resume,
        name: "resume",
        aliases: &[],
        positionals: &[("city", Required)],
        flags: &[],
        says: "after a restart: verify, close what was lost, report",
        effect: Effect::Changes,
        after_dashes: AfterDashes::Refused,
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
            flag(
                "--until",
                Value("kind"),
                "a command ends on the first event of this kind",
            ),
            flag("--json", Nothing, "write a refusal as one line of json"),
        ],
        says: "send one wire frame, print every frame back",
        effect: Effect::Changes,
        after_dashes: AfterDashes::Refused,
    },
    Row {
        verb: Verb::Dispatch,
        name: "dispatch",
        aliases: &[],
        positionals: &[("addr", Required), ("task", Required)],
        flags: &[
            AT,
            flag("--token", Value("token"), "the pairing token"),
            flag(
                "--quiet-ms",
                Value("n"),
                "how long a silent city ends the wait",
            ),
            flag("--detach", Nothing, "print the run id once it starts"),
            Flag {
                short: Some("-m"),
                ..flag(
                    "--model",
                    Value("id"),
                    "run on this registered model, not main's",
                )
            },
        ],
        says: "send one task, print its events until the run freezes",
        effect: Effect::Changes,
        after_dashes: AfterDashes::Refused,
    },
    Row {
        verb: Verb::Gauge,
        name: "gauge",
        aliases: &["top"],
        positionals: &[],
        flags: &[
            AT,
            flag("--token", Value("token"), "the pairing token"),
            flag(
                "--pid",
                Value("pid"),
                "a running process and its descendants",
            ),
            flag(
                "--every",
                Value("ms"),
                "the beat, 250 to 60000 ms; 1000 when not given",
            ),
            flag(
                "--samples",
                Value("n"),
                "runs of the command, or beats of the process",
            ),
        ],
        says: "measure a served city, a process tree, or a command run n times: lines a person reads on a terminal, JSON lines otherwise",
        effect: Effect::Changes,
        after_dashes: AfterDashes::Command,
    },
    Row {
        verb: Verb::Enrol,
        name: "enrol",
        aliases: &["enroll"],
        positionals: &[("realm/name", Required)],
        flags: &[AT],
        says: "read a credential from stdin, hand it to a city",
        effect: Effect::Changes,
        after_dashes: AfterDashes::Refused,
    },
    Row {
        verb: Verb::Whose,
        name: "whose",
        aliases: &[],
        positionals: &[("city", Required), ("oid", Required)],
        flags: &[flag(
            "--trace",
            Nothing,
            "also the calls its run made since its previous commit",
        )],
        says: "which run wrote a commit this city made",
        effect: Effect::ReadsOnly,
        after_dashes: AfterDashes::Refused,
    },
    Row {
        verb: Verb::Check,
        name: "check",
        aliases: &[],
        positionals: &[("city", Required)],
        flags: &[],
        says: "read every TOML file a city holds; print each error as path:line:column",
        effect: Effect::ReadsOnly,
        after_dashes: AfterDashes::Refused,
    },
    Row {
        verb: Verb::View,
        name: "view",
        aliases: &[],
        positionals: &[("city", Required)],
        flags: &[
            flag("--tail", Value("n"), "only the last n lines that pass"),
            flag("--from", Value("seq"), "only lines at or after this seq"),
            flag("--run", Value("run"), "only this run's lines"),
            flag("--kind", Value("kind"), "only lines of this event kind"),
            flag("--who", Value("addr"), "only lines whose address starts so"),
            flag("--grep", Value("text"), "only lines holding this text"),
            flag(
                "--since",
                Value("utc"),
                "only lines at or after this moment, as 2026-05-14T09:31:07Z",
            ),
            flag("--until", Value("utc"), "only lines before this moment"),
            flag("--runs", Nothing, "one JSON line per run, with its parents"),
        ],
        says: "read a city's ledger lines or its run tree, read-only",
        effect: Effect::ReadsOnly,
        after_dashes: AfterDashes::Refused,
    },
    Row {
        verb: Verb::PlaybackExport,
        name: "playback export",
        aliases: &[],
        positionals: &[("city", Required)],
        flags: &[
            flag("--from", Value("seq"), "only lines at or after this seq"),
            flag(
                "--through",
                Value("seq"),
                "only lines at or before this seq",
            ),
            flag("--run", Value("run"), "only this run's lines"),
            flag(
                "--building",
                Value("addr"),
                "only lines addressed within this building",
            ),
            INCLUDE_CONFIDENTIAL,
            flag(
                "--page",
                Value("template"),
                "write a playback page: this template with the bundle in its data block",
            ),
            flag(
                "--out",
                Value("file"),
                "write a new file instead of stdout; never overwrites",
            ),
        ],
        says: "export a stretch of a city's history as a playback bundle or page",
        effect: Effect::Changes,
        after_dashes: AfterDashes::Refused,
    },
    Row {
        verb: Verb::PlaybackCheck,
        name: "playback check",
        aliases: &[],
        positionals: &[("file", Required)],
        flags: &[
            flag(
                "--bundle",
                Value("file"),
                "compare with another bundle, byte for byte",
            ),
            flag(
                "--city",
                Value("city"),
                "recompute the bundle from its city and compare",
            ),
            INCLUDE_CONFIDENTIAL,
            flag(
                "--observed",
                Value("file"),
                "what a browser saw the page do, as the playback skill records it",
            ),
        ],
        says: "check a playback bundle or page, five items reported apart",
        effect: Effect::ReadsOnly,
        after_dashes: AfterDashes::Refused,
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
        after_dashes: AfterDashes::Refused,
    },
    Row {
        verb: Verb::Adopt,
        name: "adopt",
        aliases: &[],
        positionals: &[("city", Required), ("addr", Required)],
        flags: &[],
        says: "take an existing directory in as a building",
        effect: Effect::Changes,
        after_dashes: AfterDashes::Refused,
    },
    Row {
        verb: Verb::Replay,
        name: "replay",
        aliases: &[],
        positionals: &[("ledger-dir", Required)],
        flags: &[],
        says: "verify a chain offline, read-only",
        effect: Effect::ReadsOnly,
        after_dashes: AfterDashes::Refused,
    },
    Row {
        verb: Verb::Export,
        name: "export",
        aliases: &[],
        positionals: &[("city", Required), ("bundle-dir", Required)],
        flags: &[],
        says: "pack a whole city",
        effect: Effect::ReadsOnly,
        after_dashes: AfterDashes::Refused,
    },
    Row {
        verb: Verb::Restore,
        name: "restore",
        aliases: &[],
        positionals: &[("bundle-dir", Required), ("city", Required)],
        flags: &[],
        says: "unpack a bundle on another machine",
        effect: Effect::Changes,
        after_dashes: AfterDashes::Refused,
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
        after_dashes: AfterDashes::Refused,
    },
    Row {
        verb: Verb::Desktop,
        name: "desktop",
        aliases: &[],
        positionals: &[("scope", Optional)],
        flags: &[],
        says: "serve this machine's desktop as an MCP server on stdin and stdout, within the windows <scope> allows",
        effect: Effect::Changes,
        after_dashes: AfterDashes::Refused,
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
        after_dashes: AfterDashes::Refused,
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
        after_dashes: AfterDashes::Refused,
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
        Nothing => format!(" [{}]", spelled(flag)),
        Value(what) => format!(" [{} <{what}>]", spelled(flag)),
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

/// Every verb, one line each, the descriptions aligned past the longest
/// name.
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

/// A flag as help prints it: `-m/--model`, or `--at` when it has no short.
fn spelled(flag: &Flag) -> String {
    match flag.short {
        Some(short) => format!("{short}/{}", flag.name),
        None => flag.name.to_owned(),
    }
}
