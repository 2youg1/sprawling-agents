// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::Need::{Optional, Required};
use super::Takes::{Nothing, Value};
use super::{AT, AfterDashes, Effect, Flag, INCLUDE_CONFIDENTIAL, JSON, LOG, Row, Verb, flag};

pub(super) const CALL: Row = Row {
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
        JSON,
    ],
    says: "send one wire frame, print every frame back",
    effect: Effect::Changes,
    after_dashes: AfterDashes::Refused,
};

pub(super) const DISPATCH: Row = Row {
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
};

pub(super) const GAUGE: Row = Row {
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
    says: "measure a served city, a process tree, or a command run n times: lines for reading on a terminal, JSON lines otherwise",
    effect: Effect::Changes,
    after_dashes: AfterDashes::Command,
};

pub(super) const ENROL: Row = Row {
    verb: Verb::Enrol,
    name: "enrol",
    aliases: &["enroll"],
    positionals: &[("realm/name", Required)],
    flags: &[AT],
    says: "read a credential from stdin, hand it to a city",
    effect: Effect::Changes,
    after_dashes: AfterDashes::Refused,
};

pub(super) const WHOSE: Row = Row {
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
};

pub(super) const CHECK: Row = Row {
    verb: Verb::Check,
    name: "check",
    aliases: &[],
    positionals: &[("city", Required)],
    flags: &[],
    says: "read every TOML file a city holds; print each error as path:line:column",
    effect: Effect::ReadsOnly,
    after_dashes: AfterDashes::Refused,
};

pub(super) const SECRETS: Row = Row {
    verb: Verb::Secrets,
    name: "secrets",
    aliases: &[],
    positionals: &[("city", Required)],
    flags: &[],
    says: "list checkpoint scan findings from the ledger, never the matched bytes",
    effect: Effect::ReadsOnly,
    after_dashes: AfterDashes::Refused,
};

pub(super) const VIEW: Row = Row {
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
        flag(
            "--shells",
            Nothing,
            "one JSON line per shell interpreter: its calls and failures by class",
        ),
    ],
    says: "read a city's ledger lines or its run tree, read-only",
    effect: Effect::ReadsOnly,
    after_dashes: AfterDashes::Refused,
};

pub(super) const PLAYBACK_EXPORT: Row = Row {
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
        flag(
            "--since",
            Value("utc"),
            "only lines at or after this moment, as 2026-05-14T09:31:07Z",
        ),
        flag("--until", Value("utc"), "only lines before this moment"),
        flag(
            "--day",
            Value("yyyy-mm-dd"),
            "only lines of this UTC day; crossed with --since and --until",
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
};

pub(super) const PLAYBACK_CHECK: Row = Row {
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
};

pub(super) const FORK: Row = Row {
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
};

pub(super) const ADOPT: Row = Row {
    verb: Verb::Adopt,
    name: "adopt",
    aliases: &[],
    positionals: &[("city", Required), ("addr", Required)],
    flags: &[],
    says: "take an existing directory in as a building",
    effect: Effect::Changes,
    after_dashes: AfterDashes::Refused,
};

pub(super) const REPLAY: Row = Row {
    verb: Verb::Replay,
    name: "replay",
    aliases: &[],
    positionals: &[("ledger-dir", Required)],
    flags: &[],
    says: "verify a chain offline, read-only",
    effect: Effect::ReadsOnly,
    after_dashes: AfterDashes::Refused,
};

pub(super) const EXPORT: Row = Row {
    verb: Verb::Export,
    name: "export",
    aliases: &[],
    positionals: &[("city", Required), ("bundle-dir", Required)],
    flags: &[],
    says: "pack a whole city",
    effect: Effect::ReadsOnly,
    after_dashes: AfterDashes::Refused,
};

pub(super) const RESTORE: Row = Row {
    verb: Verb::Restore,
    name: "restore",
    aliases: &[],
    positionals: &[("bundle-dir", Required), ("city", Required)],
    flags: &[],
    says: "unpack a bundle on another machine",
    effect: Effect::Changes,
    after_dashes: AfterDashes::Refused,
};

pub(super) const DOCTOR: Row = Row {
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
};

pub(super) const DESKTOP: Row = Row {
    verb: Verb::Desktop,
    name: "desktop",
    aliases: &[],
    positionals: &[("scope", Optional)],
    flags: &[],
    says: "serve this machine's desktop as an MCP server on stdin and stdout, within the windows <scope> allows",
    effect: Effect::Changes,
    after_dashes: AfterDashes::Refused,
};

pub(super) const INSTALL: Row = Row {
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
};

pub(super) const STATUS: Row = Row {
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
};
