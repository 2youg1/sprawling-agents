// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Everything a client may ask the city to do, and the two invariants
//! that live in the type system rather than in a check.
//!
//! - `Command` is generic over the carrier of a secret. `WireCommand`
//!   fixes that carrier to an uninhabited type, so a frame arriving from
//!   a socket cannot be a `PutSecret` — not "is rejected", but has no
//!   representation. Credentials are enrolled on the host machine, and
//!   that constraint is held by construction.
//! - Every state-changing Command owns an `IdemKey` field. There is no
//!   constructor that omits it, so "double-clicking twice opens two
//!   runs" is not reachable from this type.
//!
//! The enum is not `#[non_exhaustive]`: the wire version is the
//! versioning mechanism, so the assembly layer must handle every variant
//! and a new one fails to compile until somebody decides what it does.
//! That is what keeps a button off the client until the city can answer
//! the frame behind it.

//! Command kinds: names, steps, the wire enum.

use kernel::{
    Address, ApprovalId, Autonomy, Ceiling, DialectKind, Effort, GitOid, IdemKey, McpServer,
    ModelTag, PolicyVerdict, RunId, SandboxLimits, Sealed, Seq, SessionName,
};
use serde::{Deserialize, Serialize};

use crate::carried_name::{ModeTag, ProviderName, TemplateName, ToolkitSlug, UploadId};
use crate::command::step::{GovernedDocument, HaltScope, LoginStep, PursuitStep};
use crate::command::tuning::EndpointTuning;

pub const COMMAND_NAMES: [&str; 28] = [
    "Dispatch",
    "Wake",
    "Login",
    "ProbeEndpoint",
    "AttachEndpoint",
    "SelectModel",
    "Fork",
    "Attach",
    "CreateBuilding",
    "ConfigureBuilding",
    "PutSecret",
    "Steer",
    "Cancel",
    "Takeover",
    "Rollback",
    "Halt",
    "Release",
    "BatchByBuilding",
    "Approve",
    "CreatePolicy",
    "SetAutonomy",
    "Pursue",
    "PutDocument",
    "Auth",
    "Reveal",
    "DoctorInstall",
    "DoctorRefresh",
    "ConnectToolkit",
];

/// Commands change state, require authorization, and are idempotent.
///
/// Deliberately *not* `#[non_exhaustive]`: the schema hash is this type's
/// version mechanism, so the assembly layer must handle every one of them
/// and a new one fails to compile until somebody decides what it does.
/// That is the rule that keeps a button off the client until the city can
/// answer the frame behind it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "schema", schemars(rename = "Command"))]
pub enum Command<Secret = Sealed<String>> {
    Dispatch {
        addr: Address,
        task: String,
        goal: String,
        mode: ModeTag,
        idem: IdemKey,
        /// What this session is called, when it is a new one.
        ///
        /// `Some` means `addr` names a building and the city opens a
        /// room of this name under it; `None` means `addr` already
        /// names the room, which is how an earlier session is
        /// continued. Two dispatches to one room are one session with a
        /// history, and that is a different thing from two sessions
        /// sharing a folder.
        session: Option<SessionName>,
        /// How hard the model is asked to think in this session.
        ///
        /// `None` leaves the layer above to answer, which is the city's
        /// own configuration and, failing that, the provider's default.
        /// A value is written into the session's own configuration when
        /// its room is opened, so it is chosen once and holds for every
        /// run in that room (city-SPEC.md section 8-14).
        effort: Option<Effort>,
    },
    /// One step of a subscription login. Which step is named rather
    /// than inferred: beginning and redeeming are different actions
    /// with different failure modes, and a page that means one must not
    /// be readable as the other.
    Login {
        provider: ProviderName,
        step: LoginStep,
        idem: IdemKey,
    },
    /// Asks a base URL what it serves, and attaches nothing.
    ///
    /// A person cannot choose from a list they have not seen, and the
    /// list used to arrive only as a side effect of attaching - so
    /// looking at what a key buys meant registering it first. The answer
    /// lands as `endpoint_probed`, which the page folds like any other
    /// fact about this city.
    ///
    /// It carries the same `tuning` the attachment will, because a
    /// probe that reached a gateway without the header that gateway
    /// requires answers 401 for a key that is in fact good.
    ProbeEndpoint {
        name: ProviderName,
        base_url: String,
        dialect: DialectKind,
        secret: Option<String>,
        auth_header: Option<String>,
        tuning: EndpointTuning,
        idem: IdemKey,
    },
    /// What a building's runs may reach: the sandbox's limits, the
    /// external servers its tools come from, and the windows on this
    /// person's own machine its desktop connector may touch.
    ///
    /// None of the three had a surface, so a person could read what they
    /// were governed by and not change it. Each field is optional and an
    /// absent one leaves that section alone; an empty `mcp` list is a
    /// building that reaches no server, which is a different statement
    /// from not saying.
    ///
    /// `desktop` is the allowlist's text and not a parsed value, and
    /// that is deliberate: the authority on that file's syntax is the
    /// connector that reads it at start-up, and that connector fails
    /// closed. A second parser on this side would be a second authority
    /// (city-SPEC.md 8-26).
    ConfigureBuilding {
        addr: Address,
        sandbox: Option<SandboxLimits>,
        mcp: Option<Vec<McpServer>>,
        desktop: Option<String>,
        idem: IdemKey,
    },
    AttachEndpoint {
        name: ProviderName,
        /// The base URL as a provider's documentation prints it; the
        /// dialect owns the path that hangs off it.
        base_url: String,
        dialect: DialectKind,
        /// `secret:<realm>/<name>`, or absent for a local server that
        /// asks for no credential.
        secret: Option<String>,
        /// Header name for providers that do not take a bearer token.
        auth_header: Option<String>,
        /// Which of the models this endpoint serves the city admits. An
        /// empty list admits everything it serves, which is what a
        /// person who did not look at the list meant.
        admit: Vec<String>,
        /// What this endpoint is called, how long it may take, and what
        /// every request to it carries. The city keeps it beside the
        /// registration, so a call made a week later is made the way
        /// the person set it up.
        tuning: EndpointTuning,
        idem: IdemKey,
    },
    /// Point one tag at one model of an attached endpoint, with the two
    /// facts no model list returns.
    SelectModel {
        endpoint: ProviderName,
        model: String,
        tag: ModelTag,
        /// The model's window. Zero states no window, and the context
        /// reminder then stays silent rather than measuring against a
        /// number nobody gave it.
        context_tokens: u64,
        /// The model's output ceiling. Absent when the person did not
        /// state one: the city then takes the catalogue's figure, and
        /// where the catalogue has no row the model is registered
        /// without a ceiling rather than with a zero one.
        max_output_tokens: Option<Ceiling>,
        idem: IdemKey,
    },
    Fork {
        run: RunId,
        at_seq: Seq,
        addr: Option<Address>,
        idem: IdemKey,
    },
    Attach {
        upload: UploadId,
        notify: Vec<RunId>,
        idem: IdemKey,
    },
    CreateBuilding {
        addr: Address,
        template: TemplateName,
        idem: IdemKey,
    },
    /// The one Command with no byte form. `Secret` is `Sealed<String>` in
    /// process and uninhabited on the wire.
    PutSecret {
        realm: String,
        name: String,
        value: Secret,
    },
    Steer {
        run: RunId,
        text: String,
        idem: IdemKey,
    },
    Cancel {
        run: RunId,
        idem: IdemKey,
    },
    Takeover {
        run: RunId,
        idem: IdemKey,
    },
    Rollback {
        checkpoint: GitOid,
        idem: IdemKey,
    },
    Halt {
        scope: HaltScope,
        idem: IdemKey,
    },
    /// Show a person this address in their own file manager. The
    /// grammar is the guard: an `Address` cannot climb out of the city,
    /// so there is no way to spell a request for anything else.
    Reveal {
        at: Address,
        idem: IdemKey,
    },
    /// Install one thing this machine lacks, named as the doctor's
    /// answer names it.
    ///
    /// Only a recipe this city may run is run. A recipe that has to be
    /// printed, and one that has no command at all, are refused with
    /// what the person does instead: a script piped into a shell is
    /// code nobody read, and that rule does not soften because the
    /// request arrived from a page rather than from a terminal.
    DoctorInstall {
        item: String,
        idem: IdemKey,
    },
    /// Look at this machine again, in place of the snapshot taken when
    /// the city was served.
    ///
    /// [`Query::Doctor`](crate::Query::Doctor) answers that snapshot,
    /// which is what a page must not be given after it has just
    /// installed something. Probing is seconds of starting programs, so
    /// it happens here, where the city already serialises work, rather
    /// than inside a read.
    DoctorRefresh {
        idem: IdemKey,
    },
    Release {
        scope: HaltScope,
        idem: IdemKey,
    },
    BatchByBuilding {
        addr: Address,
        idem: IdemKey,
    },
    Approve {
        item: ApprovalId,
        verdict: PolicyVerdict,
        idem: IdemKey,
    },
    CreatePolicy {
        from_item: ApprovalId,
        idem: IdemKey,
    },
    SetAutonomy {
        scope: HaltScope,
        autonomy: Autonomy,
        idem: IdemKey,
    },
    /// A goal the city keeps working towards until the work runs out.
    ///
    /// One frame with four steps rather than four frames: a person who
    /// can set a goal can pause it, and splitting that into separate
    /// commands would let a client offer one without the other.
    Pursue {
        addr: Address,
        step: PursuitStep,
        idem: IdemKey,
    },
    /// Something happened outside. The city never asks whether anything
    /// did; the service holding the connection pushes, and this is the
    /// shape a push takes once it is inside.
    ///
    /// It carries no address. Where an arrival lands is the watch
    /// table's answer and then triage's, so a caller that could name a
    /// room would be a caller that could reach past the routing a person
    /// wrote.
    Wake {
        source: String,
        subject: String,
        body: String,
        idem: IdemKey,
    },
    /// Writes one of the three documents that govern this city.
    ///
    /// The body replaces the file whole rather than patching it: these
    /// are documents a person edits in one box and saves once, and a
    /// partial write would leave the city governed by half a sentence.
    /// No version travels with it for the same reason — there is no
    /// second writer to lose a race against.
    PutDocument {
        which: GovernedDocument,
        body: String,
        idem: IdemKey,
    },
    /// Presenting a pairing token. Read-only, hence no `IdemKey`; the token
    /// is plain here because a token that must cross a wire has, by
    /// definition, no secrecy left to protect in transit - it is sealed the
    /// moment it lands (see `server::decide_handshake`).
    /// Connects one outside application through the broker that holds
    /// its OAuth.
    ///
    /// Answers with the whole shelf rather than with a bare url. The row
    /// the person pressed comes back as `Standing::Awaiting` carrying
    /// its consent page, and every other row comes back with it: the
    /// broker was asked, so the reading is fresh for all of them, and a
    /// half-refreshed list is a list that disagrees with itself.
    ///
    /// **The consent page is opened by the client, never by the city.**
    /// The person is sitting at the client; the city may be running on
    /// a machine in another room, and a browser opened there is a
    /// browser nobody is looking at. This is also why the url keeps
    /// travelling in the answer instead of being spent once - a blocked
    /// popup leaves a person who still needs the link.
    ///
    /// It carries an `IdemKey` like every other state change, and here
    /// that key is what stops a second press from opening a second
    /// account on the same application.
    ConnectToolkit {
        toolkit: ToolkitSlug,
        idem: IdemKey,
    },
    Auth {
        token: String,
    },
}
