// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Command kinds: names, steps, the wire enum.

use kernel::model::{Mode, Window};
use kernel::{
    Address, ApprovalId, Autonomy, Ceiling, DialectKind, Effort, IdemKey, McpServer, ModelTag,
    Origin, ResidentId, Ruling, RunId, SandboxLimits, Sealed, SessionName,
};
use serde::{Deserialize, Serialize};

use crate::carried_name::{ProviderName, TemplateName, ToolkitSlug};
use crate::command::shelf::Shelf;
use crate::command::step::{
    Carry, GovernedDocument, HaltScope, LoginStep, PursuitStep, SpineDocument,
};
use crate::command::tuning::EndpointTuning;
use crate::named_frames::named_frames;
use crate::preference::PreferencePatch;

named_frames! {
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
        mode: Mode,
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
        /// Which registered model this one dispatch calls, by its id.
        ///
        /// `None` takes the model behind the `main` tag. A value names a
        /// model the city registered under some tag, so it arrives with
        /// the endpoint and the window it was registered with; an id the
        /// city never registered is refused before anything is written
        /// (channels-SPEC.md section 8-48).
        model: Option<String>,
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
        /// Where the context reminder's second rung sits, as one layer
        /// states it: a raw percent on the wire. The domain is
        /// `kernel::config::SecondThreshold`'s one construction point,
        /// and a deserializer that enforced it here would be the second
        /// place that rule lives.
        context_second_threshold: Option<u64>,
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
        /// The model's window. Absent when nobody stated one, and the
        /// context reminder then stays silent rather than measuring a
        /// conversation against a number nobody gave it.
        context_tokens: Option<Window>,
        /// The model's output ceiling. Absent when the person did not
        /// state one: the city then takes the catalogue's figure, and
        /// where the catalogue has no row the model is registered
        /// without a ceiling rather than with a zero one.
        max_output_tokens: Option<Ceiling>,
        idem: IdemKey,
    },
    /// Starts a new session at an address: same room, same person, a
    /// fresh conversation in it.
    ///
    /// **A session is a stretch of a room, not the room.** A room's
    /// first run writes down the model it calls and how hard it thinks,
    /// and every later run there refuses to move either, because a
    /// provider caches a conversation's prefix only while the shape of
    /// the calls behind it holds still. That rule is right, and without
    /// this verb it was also a dead end: a person who changed the model
    /// could no longer dispatch into the room at all. What this frame
    /// does is let the room start a new stretch, which is a thing a
    /// person asks for on purpose rather than a change made behind
    /// their back (sprawling-SPEC.md 8-82).
    ///
    /// It carries [`Carry`] rather than a flag: what a new session
    /// keeps from the one before it has two named answers, and the
    /// default is the one that keeps nothing.
    ///
    /// Writes `session_opened`.
    OpenSession {
        addr: Address,
        carry: Carry,
        /// What this session continues, when it continues something.
        ///
        /// A branch is a session and not a second verb: forking is
        /// starting a session whose first run begins from a line of
        /// another conversation, and everything else about it - the
        /// model it may choose, the effort, the handoff it carries -
        /// is what [`Carry`] and the room already say
        /// (sprawling-SPEC.md 8-82).
        from: Option<Origin>,
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
        verdict: Ruling,
        idem: IdemKey,
    },
    /// Give one waiting question to a resident to answer.
    ///
    /// The inbox holds design questions, and a person who does not
    /// want to answer one has exactly two ways out: answer it anyway,
    /// or name somebody who will. This is the second. It moves one
    /// item and leaves [`Command::SetAutonomy`] to say who answers
    /// everything, because handing over a single question and
    /// appointing a standing delegate are different decisions with
    /// different reach.
    ///
    /// Writes `question_handed`, after which the named resident may
    /// answer this item even though it is not the standing delegate.
    HandOff {
        item: ApprovalId,
        to: ResidentId,
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
    /// Writes one of a building's own spine documents.
    ///
    /// The body replaces the file whole. Unlike [`Command::PutDocument`]
    /// these have a second writer - a resident reaches `Roadmap.md`
    /// through `plan` and the others through `edit` - so `base` carries
    /// the text the sender started from and a file that moved underneath
    /// it is refused rather than overwritten. One guard for two writers,
    /// and the same one `edit` holds resident-side.
    PutSpine {
        building: Address,
        which: SpineDocument,
        base: String,
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
    /// Change one fact about how this person reads their own city.
    ///
    /// One named fact per frame rather than a whole record: two
    /// screens settling different things must not be able to overwrite
    /// each other's field on the way past.
    PutPreferences {
        patch: PreferencePatch,
        idem: IdemKey,
    },
    /// Write one skill or script onto a shelf.
    ///
    /// The shelf and the name are separate because the city owns where
    /// a shelf lives: the two shelves sit inside the reserved subtree,
    /// which no write domain reaches, so this is the door a person
    /// writes there through and there is no path to spell. `name` may
    /// carry sub-directories, which is how a script keeps its folder.
    ///
    /// The body replaces the file whole, for the reason
    /// [`Command::PutDocument`] gives. Writes `shelved_document_written`.
    PutShelved {
        shelf: Shelf,
        name: String,
        text: String,
        idem: IdemKey,
    },
    Auth {
        token: String,
    },
}

/// The Command surface, in declaration order — the order the handshake
/// hash mixes these names in.
pub const COMMAND_NAMES;
}
