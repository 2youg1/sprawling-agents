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
    Address, ApprovalId, Autonomy, DialectKind, Effort, GitOid, IdemKey, McpServer, ModelTag,
    PolicyVerdict, RunId, SandboxLimits, Sealed, Seq, SessionName,
};
use serde::{Deserialize, Serialize};

use crate::carried_name::{ModeTag, ProviderName, TemplateName, UploadId};

pub const COMMAND_NAMES: [&str; 24] = [
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
];

/// Uninhabited on purpose. A value of this type cannot be produced, so
/// `Command<NoSecret>` has no reachable `PutSecret` variant. This is the
/// compile-time half of "a remote connection cannot spell that frame";
/// `Deserialize` supplies the runtime half for bytes that try anyway.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoSecret {}

impl Serialize for NoSecret {
    fn serialize<S: serde::Serializer>(&self, _serializer: S) -> Result<S::Ok, S::Error> {
        match *self {}
    }
}

impl<'de> Deserialize<'de> for NoSecret {
    fn deserialize<D: serde::Deserializer<'de>>(_deserializer: D) -> Result<Self, D::Error> {
        Err(serde::de::Error::custom(
            "a credential cannot be enrolled over a connection; enrol it on the host",
        ))
    }
}

/// The schema half of the same statement: `false` is the schema no value
/// satisfies, so a client generated from it types the field as `never`.
#[cfg(feature = "schema")]
impl schemars::JsonSchema for NoSecret {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("NoSecret")
    }

    fn json_schema(_generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::Schema::from(false)
    }
}

/// Which step of a subscription login a `Login` frame carries.
///
/// The authorization code arrives by hand: the provider shows it to the
/// person after they approve, and the person brings it back. That is
/// the flow the profile table describes, and it needs no listening port
/// of its own.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum LoginStep {
    /// Mint the authorization URL for a person to open.
    Begin,
    /// Redeem the code that person brought back.
    Code { code: String },
}

/// What a Halt, Release or Autonomy change applies to. Unlike modes and
/// providers, this set is the protocol's own and has no upstream owner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum HaltScope {
    City,
    Building(Address),
    Workshop(Address),
}

/// Which of the three documents that govern a city a `PutDocument`
/// frame carries.
///
/// A closed set rather than a path, because where these files live is
/// the city's answer and not the sender's: all three sit in the city's
/// own reserved subtree, which no write domain reaches. A frame naming
/// its own path would be a way to write anywhere inside the one place a
/// resident may not edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum GovernedDocument {
    /// Who the Mayor is: `MAYOR.md`.
    Mayor,
    /// What the clerk answers by: `CLERK.md`.
    Clerk,
    /// How this person wants their city run: `PREFERENCES.md`. It
    /// belongs to no resident, which is why it sits beside the other two
    /// rather than at somebody's address.
    Preferences,
}

/// What a `Pursue` command does to a pursuit.
///
/// `Clear` and `Pause` are different actions and both exist: pausing
/// keeps the goal so it can be taken up again, and clearing throws it
/// away. Cancelling a *run* is a third thing again, and it has its own
/// command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum PursuitStep {
    /// Declare one, replacing any goal this building already had.
    Set {
        goal: String,
    },
    Pause,
    Resume,
    Clear,
}

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
    /// Register a provider the person just entered. The credential is
    /// already in the vault by the time this frame exists: what travels
    /// here is the reference to it, which is why this command has a byte
    /// form and `PutSecret` does not.
    /// Asks a base URL what it serves, and attaches nothing.
    ///
    /// A person cannot choose from a list they have not seen, and the
    /// list used to arrive only as a side effect of attaching - so
    /// looking at what a key buys meant registering it first. The answer
    /// lands as `endpoint_probed`, which the page folds like any other
    /// fact about this city.
    ProbeEndpoint {
        name: ProviderName,
        base_url: String,
        dialect: DialectKind,
        secret: Option<String>,
        auth_header: Option<String>,
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
        idem: IdemKey,
    },
    /// Point one tag at one model of an attached endpoint, with the two
    /// facts no model list returns.
    SelectModel {
        endpoint: ProviderName,
        model: String,
        tag: ModelTag,
        context_tokens: u64,
        max_output_tokens: u64,
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
    Auth {
        token: String,
    },
}
