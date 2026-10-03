// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What this machine has, as the city found it when it started.
//!
//! Every state here is an enum rather than a sentence. The terminal
//! report is one machine's prose and a browser is two languages, so a
//! wire that carried the wording would make the page's words the
//! server's to choose. What an item is for travels as its `name`, the
//! id the page looks its own clause up by. The `said` fields carry what
//! a platform or a program said, which no reader can derive.

use serde::{Deserialize, Serialize};

/// This machine, item by item, with a verdict for each tier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DoctorAnswer {
    /// Every item this city needs, in the order the requirement table
    /// states them.
    pub items: Vec<DoctorItem>,
    /// One verdict per tier: a person who only wants to run a city is
    /// not told about what changing this code would need.
    pub tiers: Vec<DoctorVerdict>,
    /// The confinement a command an agent asks for runs under, and the
    /// axes that arm does not hold. An agent has to be able to read
    /// that it is not in a box with the network closed before it acts.
    pub sandbox: DoctorSandbox,
    /// Where credentials rest on this machine, and how long they stay.
    pub custody: DoctorCustody,
    /// Where this machine lets the core's threads stand.
    pub core: DoctorCore,
    /// Whether real-time scanning stands in front of the city
    /// directory's writes (`crates/wire/spec/Answer/Doctor.lean` D25).
    pub scanning: super::scanning::DoctorScanning,
}

/// The level this machine gives the core's threads under the person's
/// setting (`crates/sprawling/Spec.lean` §8-93). The dispatched commands are not here:
/// they always start one level below, and lowering is never refused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorCore {
    /// One step above normal.
    Raised,
    /// Normal, because the person's `[core] priority` asks for it.
    HeldBySetting,
    /// Normal, because the platform refused the raise; `said` is the
    /// platform's own words.
    Refused { said: String },
    /// Normal, lowered after keeping a core busy.
    LoweredByValve,
    /// The doctor could not ask; `said` is why.
    Unasked { said: String },
}

/// Who needs an item, and therefore which verdict it counts towards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorTier {
    /// Enough to run a city on this machine.
    Use,
    /// Enough to change this code and close its checks.
    Develop,
}

/// Whether a tier can be reached without the item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorNeed {
    Required,
    /// Absent is a fact rather than a fault.
    Optional,
}

/// One item, and this machine's answer about it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DoctorItem {
    /// The requirement table's name for the item, which is also its id:
    /// a page says what the item is for in its own words, looked up by
    /// this name.
    pub name: String,
    pub tier: DoctorTier,
    pub need: DoctorNeed,
    /// The item's own site, so a page can link a name to the people who
    /// publish it. `None` where there is no one site: a platform's
    /// shell, and this project's own connector.
    pub homepage: Option<String>,
    pub state: DoctorState,
    pub install: DoctorInstall,
    /// The version this repository pins the item at, read from the file
    /// that pins it; `None` for an item nothing pins.
    pub pinned: Option<String>,
    /// The pack a page draws this item inside, with one install control
    /// for the pack's missing members; `None` for an item drawn alone.
    pub pack: Option<DoctorPack>,
}

/// A set of items a page draws as one row: each member is still its own
/// item here, because each is detected, judged and installed on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorPack {
    /// The cargo subcommands this repository's recipes and workflows call.
    RustTools,
}

/// The newest release of one item upstream, as its publisher states it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DoctorUpstream {
    /// The requirement table's name for the item, as `DoctorItem.name`.
    pub item: String,
    pub newest: DoctorNewest,
}

/// What asking the item's publisher came to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorNewest {
    /// The city has asked the publisher and has no answer yet; asking
    /// again later reads what arrived. The question never waits on the
    /// network, because a session answers its questions one at a time.
    Asking,
    /// The newest stable version, as a dotted number.
    Read { version: String },
    /// This item has no single upstream version to read, and why.
    Unread { why: DoctorUnread },
    /// The source was asked and did not answer; `said` is where the
    /// call stopped.
    Refused { said: String },
}

/// Why an item has no upstream version to compare with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorUnread {
    /// It ships inside the Rust toolchain, so the toolchain's version is
    /// the one that moves.
    WithToolchain,
    /// A family of browsers, each brand with its own releases.
    ManyBrands,
    /// A driver whose version follows the browser beside it.
    MatchesBrowser,
    /// A part of this project, released with it.
    ThisProject,
    /// Its publisher offers no machine-readable release to ask.
    NoSource,
    /// The requirement table carries no item by that name.
    UnknownItem,
}

/// Whether the item is here, and in what condition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorState {
    /// Here, and it started when asked its version.
    Present {
        at: String,
        version: DoctorVersion,
    },
    /// Here, and unusable. Counts as missing for a verdict, and as its
    /// own fault for a person.
    Broken {
        at: String,
        fault: DoctorFault,
    },
    Absent {
        absence: DoctorAbsence,
    },
}

/// What a present program said when asked its version. Only the first
/// arm carries text; the other three are facts about how it did not
/// answer, and none of them makes the program absent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorVersion {
    Said {
        text: String,
    },
    /// It started, wrote nothing, and exited.
    Silent,
    /// Its first line was not text.
    Unreadable,
    /// It had not written a line when the deadline passed.
    Late,
}

/// Why a thing that is here cannot be used.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorFault {
    /// The operating system refused to start it.
    WillNotStart {
        said: String,
    },
    /// The component directory exists and the component file does not:
    /// an install that stopped half way.
    HalfWritten,
    Unreadable {
        said: String,
    },
}

/// Which kind of not being here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorAbsence {
    NotOnSearchPath,
    /// A person pointed at a file, and the file is not there.
    VariableNamesNothing {
        variable: String,
        path: String,
    },
    NoComponent {
        dir: String,
    },
    NoHome,
    NotInThisBuild,
}

/// What getting the item would cost on this machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorInstall {
    /// A command this city may run once the person has agreed to it.
    Command { spelled: String },
    /// A command printed and never run: a script piped into a shell is
    /// code nobody read.
    Print { spelled: String },
    /// Nothing here can install it; the text says what a person does.
    Manual { how: String },
    /// This machine is not one of the three this project names recipes
    /// for, so nothing can be said about installing anything on it.
    UnknownPlatform,
}

/// Whether one tier is reachable on this machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DoctorVerdict {
    pub tier: DoctorTier,
    /// The required items of this tier that are absent or broken, in
    /// table order. Empty means ready.
    pub missing: Vec<String>,
}

/// Which backend a host command runs under on this machine, and what
/// that arm promises.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DoctorSandbox {
    pub arm: DoctorSandboxArm,
    /// The arm's name in the closed set a setting chooses from
    /// (`crates/wire/spec/Answer/Doctor.lean` D26).
    pub named: SandboxArm,
    /// One row per axis, in the order the axes are declared. The rows
    /// are stated rather than left to the page to infer: an arm that
    /// holds some axes and not others is the whole reason this report
    /// exists.
    pub coverage: Vec<DoctorGuarantee>,
}

/// The backend itself. Closed, so a page has a word for every arm and
/// a new arm is a compile error at every reader.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorSandboxArm {
    /// Namespaces of its own, made by a wrapper program.
    LinuxNamespaces,
    /// A Windows job object: the process tree ends together and the
    /// limits hold, and the network is not isolated.
    WindowsJobObject,
    /// The floor every platform has: the command runs in a copy of the
    /// working tree.
    CopiedTree,
    /// No arm at all, and what this machine is missing.
    Unavailable { missing: DoctorSandboxMissing },
}

/// The names a sandbox arm is chosen by, one per mechanism family, so
/// that every name has an arm to fill on Windows, macOS and Linux
/// (`crates/wire/spec/Answer/Doctor.lean` D26).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum SandboxArm {
    /// The host itself: no axis is held.
    None,
    CopiedTree,
    /// The platform's own mechanism: namespaces on Linux, Seatbelt on
    /// macOS, a job object on Windows.
    Native,
    Container,
    /// Python inside a WebAssembly sandbox.
    Python,
}

/// What a machine lacks when it can give no confinement at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorSandboxMissing {
    ScratchDirectory,
}

/// One axis of confinement, and whether this machine's arm holds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DoctorGuarantee {
    pub axis: DoctorGuaranteeAxis,
    pub kept: DoctorCoverage,
}

/// What a confinement can promise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorGuaranteeAxis {
    Filesystem,
    Network,
    ProcessTree,
    User,
    Resources,
}

/// Whether the arm holds the axis. Two words rather than a boolean,
/// because a page has to word both and a wire that carried `true` would
/// make every reader choose its own word for `false`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorCoverage {
    Kept,
    NotKept,
}

/// Where this machine's credentials rest, and how long they stay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DoctorCustody {
    pub store: DoctorCustodyStore,
    pub keeps: DoctorCustodyLifetime,
    /// The platform service's own refusal, when it did not keep the
    /// value it was asked to keep. `None` is a service that worked, and
    /// a store this city chose itself.
    pub refusal: Option<String>,
}

/// Which store a city writes secrets to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorCustodyStore {
    /// The platform's own credential service.
    PlatformService,
    /// An encrypted file on this machine, opened once per start with a
    /// passphrase.
    EncryptedFile,
    /// This process only.
    SessionMemory,
}

/// How long a value the store keeps stays reachable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorCustodyLifetime {
    AcrossReboots,
    WithPassphrase,
    UntilReboot,
    ThisProcess,
}
