// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Command kinds: names, steps, the wire enum.

use kernel::model::{RunPolicy, Window};
use kernel::{
    Address, ApprovalId, Autonomy, Ceiling, DialectKind, Effort, GitOid, IdemKey, McpServer,
    ModelTag, Origin, ResidentId, Restoration, Ruling, RunId, SandboxLimits, Sealed, SessionName,
};
use serde::{Deserialize, Serialize};

use crate::carried_name::{ProviderName, TemplateName, ToolkitSlug};
use crate::command::shelf::Shelf;
use crate::command::step::{
    Carry, CitySettings, GovernedDocument, HaltScope, IdentityCard, PursuitStep, RulesWrite,
    SpineDocument,
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
        /// The four values the run works under: mode, write limit,
        /// admission requirement and landing policy, all four stated
        /// (wire-SPEC.md section 8-57). Written as sent into the run's
        /// `run_started` line.
        policy: RunPolicy,
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
        /// (wire-SPEC.md section 8-48c).
        model: Option<String>,
    },
    /// Asks a base URL what it serves, and attaches nothing.
    ///
    /// A person cannot choose from a list they have not seen, so the
    /// list is asked for apart from attaching; the answer lands as
    /// `endpoint_probed`, which the page folds like any other fact.
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
    /// Each field is optional and an absent one leaves that section
    /// alone; an empty `mcp` list is a building that reaches no server,
    /// which is a different statement from not saying.
    ///
    /// `desktop` is the allowlist's text and not a parsed value: the
    /// connector that reads that file at start-up is the authority on
    /// its syntax and fails closed, and a second parser here would be a
    /// second authority (city-SPEC.md 8-26).
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
        /// every request to it carries, kept beside the registration so a
        /// call made a week later is made the way the person set it up.
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
    /// Starts a new session at an address: same room, a fresh stretch of
    /// conversation in it, so a person who changed the model starts it on
    /// purpose rather than having the room's frozen shape move behind
    /// their back (sprawling-SPEC.md 8-82). [`Carry`] names what crosses;
    /// the default keeps nothing. Writes `session_opened`.
    OpenSession {
        addr: Address,
        carry: Carry,
        /// What this session continues, when it continues something: a
        /// branch is a session whose first run begins from a line of
        /// another conversation, and everything else about it - model,
        /// effort, handoff - is what [`Carry`] and the room already say
        /// (sprawling-SPEC.md 8-82).
        from: Option<Origin>,
        idem: IdemKey,
    },
    CreateBuilding {
        addr: Address,
        template: TemplateName,
        idem: IdemKey,
    },
    /// Take a building out of the city. Its files move under the
    /// reserved subtree and its history stays in the Ledger, so nothing
    /// the person made is lost; a building with a run going is refused.
    ///
    /// Writes `building_removed`.
    RemoveBuilding {
        addr: Address,
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
    /// Put one recycle-bin row back. The frame carries the row's own
    /// restoration rather than a path, because a path can be discarded
    /// twice and the row already knows which commit holds the bytes.
    RestoreDiscard {
        restoration: Restoration,
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
    /// which a page that has just installed something must not be given.
    /// Probing starts programs for seconds, so it runs here, where the
    /// city already serialises work, rather than inside a read.
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
    /// A person who does not want to answer a question may name
    /// somebody who will. This moves one item and leaves
    /// [`Command::SetAutonomy`] to say who answers everything, because
    /// handing over one question and appointing a standing delegate are
    /// different decisions with different reach.
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
    /// Writes one of the three documents that govern this city, whole,
    /// only if it still holds `base`: the raw editor and the identity
    /// cards are two writers of one file (wire-SPEC.md 8-59).
    PutDocument {
        which: GovernedDocument,
        base: String,
        body: String,
        idem: IdemKey,
    },
    /// One identity card: the city rewrites the card's keys in `base`'s
    /// identity area and leaves every other byte (wire-SPEC.md 8-59).
    PutIdentity {
        card: IdentityCard,
        base: String,
        idem: IdemKey,
    },
    /// Writes a building's `RULES.toml` whole (wire-SPEC.md 8-60).
    PutRules(RulesWrite),
    /// Takes one file of the city's own tree back to what a checkpoint
    /// holds, or away when it holds none (wire-SPEC.md 8-62).
    RestoreFile {
        at: Address,
        point: GitOid,
        idem: IdemKey,
    },
    /// Writes the city's own layer (wire-SPEC.md 8-61).
    ConfigureCity(CitySettings),
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
    /// Connects one outside application through the broker that holds
    /// its OAuth, answered with the whole shelf: the broker was asked, so
    /// every row is fresh, and the pressed one carries its consent page.
    ///
    /// **The client opens the consent page, never the city**, which may
    /// run on a machine nobody is looking at. The `IdemKey` stops a
    /// second press from opening a second account on one application.
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
    /// Presenting a pairing token. Read-only, hence no `IdemKey`; the token
    /// is plain here because a token that must cross a wire has, by
    /// definition, no secrecy left to protect in transit - it is sealed the
    /// moment it lands (see `server::decide_handshake`).
    Auth {
        token: String,
    },
}

/// The Command surface, in declaration order — the order the handshake
/// hash mixes these names in.
pub const COMMAND_NAMES;
}
