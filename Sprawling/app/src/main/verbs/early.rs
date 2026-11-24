// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::Need::{Optional, Required};
use super::{AfterDashes, Effect, JSON, MEI_REPLAY, Row, SERVED, Verb};

pub(super) const MEI: Row = Row {
    verb: Verb::Mei,
    name: "mei",
    aliases: &[],
    positionals: &[],
    flags: &[MEI_REPLAY],
    says: "run the academic single-shot harness in this terminal, or replay its messages",
    effect: Effect::Changes,
    after_dashes: AfterDashes::Refused,
};

pub(super) const PRIVACY_STATUS: Row = Row {
    verb: Verb::PrivacyStatus,
    name: "privacy status",
    aliases: &[],
    positionals: &[],
    flags: &[],
    says: "read local privacy operation receipts without changing history or the OS",
    effect: Effect::ReadsOnly,
    after_dashes: AfterDashes::Refused,
};

pub(super) const PRIVACY_INSPECT: Row = Row {
    verb: Verb::PrivacyInspect,
    name: "privacy inspect",
    aliases: &[],
    positionals: &[("control", Optional)],
    flags: &[],
    says: "print what each privacy control reads now and what this app owns of it",
    effect: Effect::ReadsOnly,
    after_dashes: AfterDashes::Refused,
};

pub(super) const PRIVACY_APPLY: Row = Row {
    verb: Verb::PrivacyApply,
    name: "privacy apply",
    aliases: &[],
    positionals: &[("control", Required), ("expected", Required)],
    flags: &[JSON],
    says: "write one privacy control's value if it still reads what inspect printed",
    effect: Effect::Changes,
    after_dashes: AfterDashes::Refused,
};

pub(super) const PRIVACY_RESTORE: Row = Row {
    verb: Verb::PrivacyRestore,
    name: "privacy restore",
    aliases: &[],
    positionals: &[("control", Required), ("expected", Required)],
    flags: &[JSON],
    says: "write back the original this app replaced, if the control still reads its value",
    effect: Effect::Changes,
    after_dashes: AfterDashes::Refused,
};

pub(super) const PRIVACY_RESTORE_ALL: Row = Row {
    verb: Verb::PrivacyRestoreAll,
    name: "privacy restore-all",
    aliases: &[],
    positionals: &[],
    flags: &[JSON],
    says: "restore every privacy control this app still owns, one at a time",
    effect: Effect::Changes,
    after_dashes: AfterDashes::Refused,
};

pub(super) const PRIVACY_RECONCILE: Row = Row {
    verb: Verb::PrivacyReconcile,
    name: "privacy reconcile",
    aliases: &[],
    positionals: &[("expected", Required)],
    flags: &[JSON],
    says: "record what an unresolved privacy operation left, without writing",
    effect: Effect::Changes,
    after_dashes: AfterDashes::Refused,
};

pub(super) const PRIVACY_ELEVATED_WRITE: Row = Row {
    verb: Verb::PrivacyElevatedWrite,
    name: "privacy elevated-write",
    aliases: &[],
    positionals: &[("write", Required)],
    flags: &[],
    says: "write one machine-scope privacy value; started only by the privacy page through UAC",
    effect: Effect::Changes,
    after_dashes: AfterDashes::Refused,
};

pub(super) const UP: Row = Row {
    verb: Verb::Up,
    name: "up",
    aliases: &[],
    positionals: &[("city", Optional), ("addr", Optional)],
    flags: SERVED,
    says: "raise a city here if needed, serve it, open the WebUI",
    effect: Effect::Changes,
    after_dashes: AfterDashes::Refused,
};

pub(super) const INIT: Row = Row {
    verb: Verb::Init,
    name: "init",
    aliases: &[],
    positionals: &[("city", Required)],
    flags: &[],
    says: "raise a city: writes the genesis record",
    effect: Effect::Changes,
    after_dashes: AfterDashes::Refused,
};

pub(super) const SERVE: Row = Row {
    verb: Verb::Serve,
    name: "serve",
    aliases: &[],
    positionals: &[("city", Required), ("addr", Optional)],
    flags: SERVED,
    says: "serve a city that already exists",
    effect: Effect::Changes,
    after_dashes: AfterDashes::Refused,
};

pub(super) const RESUME: Row = Row {
    verb: Verb::Resume,
    name: "resume",
    aliases: &[],
    positionals: &[("city", Required)],
    flags: &[],
    says: "after a restart: verify, close what was lost, report",
    effect: Effect::Changes,
    after_dashes: AfterDashes::Refused,
};
