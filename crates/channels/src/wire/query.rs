// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Everything a page may ask, and the name table the schema hash is
//! built from.
//!
//! Queries read state. They are cacheable and free of side effects, so
//! none carries an `IdemKey` — a Query that needed one would have
//! stopped being a Query. The name table is in declaration order and a
//! test holds it there, because the schema hash is built from it and a
//! table that drifted from the enum would let two builds agree on a hash
//! while disagreeing on what a frame means.

use kernel::{Address, GitOid, Locator, NodeId, RunId, Seq};
use serde::{Deserialize, Serialize};

/// The Query surface, in declaration order.
pub const QUERY_NAMES: [&str; 30] = [
    "History",
    "RunHistory",
    "Changes",
    "Hunks",
    "Commit",
    "RunView",
    "CityView",
    "ApprovalQueue",
    "InboxView",
    "Metrics",
    "CostView",
    "ArchiveSearch",
    "RegistryView",
    "DiscardView",
    "EndpointView",
    "BuildingView",
    "Governance",
    "Rounds",
    "Evidence",
    "CostOf",
    "Listing",
    "Document",
    "Commits",
    "Doctor",
    "Prefix",
    "Content",
    "Skills",
    "GitStatus",
    "McpHealth",
    "Toolkits",
];

/// Queries read state. They are cacheable and free of side effects, so none
/// carries an `IdemKey` - a Query that needed one would have stopped being a
/// Query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Query {
    /// A bounded slice of the one history, ending just before `before`
    /// or at the tail when that is absent.
    ///
    /// The server broadcasts what happens next and never what happened,
    /// so a page opened today saw a city that had been running for a
    /// month as an empty one. Bounded because the whole ledger is not a
    /// thing to put on a socket, and paged backwards because what a
    /// reader wants first is the end.
    History {
        before: Option<Seq>,
        limit: u32,
    },
    /// The same slice, narrowed to one session.
    ///
    /// [`Query::History`] carries no run, so a client watching four
    /// sessions divides one bounded slice between them and a session
    /// that started before the tab did is not in it at all - which is
    /// the whole of why opening yesterday's session showed a blank
    /// page. [`Query::RunView`] does not close the gap: five fields say
    /// whether a run exists and where it got to, not what happened in
    /// it.
    ///
    /// Answered with [`HistoryAnswer`], because "a page of history"
    /// already has a shape and a second one would be a second answer to
    /// the same question.
    RunHistory {
        run: RunId,
        before: Option<Seq>,
        limit: u32,
    },
    /// What moved between two checkpoints: paths and counts, never patch
    /// text.
    ///
    /// The caller names both ends because it already knows them - a
    /// checkpoint's oid is in the `checkpoint_committed` payload the
    /// client folded - and computing the pair a second time on the
    /// server would be a second answer to "which fences belong to this
    /// session". Both oids are immutable, so the answer is cacheable
    /// forever by anybody who wants to.
    ///
    /// `head` absent means the working tree: a wave still running has
    /// written files no checkpoint holds yet, and a list that ignored
    /// them would describe the session as it was one fence ago.
    Changes {
        base: GitOid,
        head: Option<GitOid>,
    },
    /// The patch text of one file between two checkpoints.
    ///
    /// **A separate frame from [`Query::Changes`], and that is the
    /// point.** `Changes` costs what the number of changed files costs
    /// and answers which files moved; this costs what one file costs
    /// and answers what moved inside it. One frame answering both would
    /// charge every "what changed here" with a whole batch of patches,
    /// which is why `memory::changes` says a hunk has to be its own
    /// request. This is that request.
    ///
    /// One path per frame; there is no spelling that asks for all of
    /// them. A line the credential scan matched is not echoed: the
    /// answer reports its line number and the reason, because printing
    /// the bytes to prove a leak is the leak.
    Hunks {
        oid_a: GitOid,
        oid_b: GitOid,
        path: String,
    },
    /// Which run wrote one commit the city made.
    ///
    /// Every commit this city makes carries five git trailers naming the
    /// run, the resident, the model, the effort and the city itself.
    /// Those trailers are a projection for readers outside the city, so
    /// this query is answered from the Ledger and never from git: a city
    /// exported and restored elsewhere, with no `.git` beside it, still
    /// answers.
    ///
    /// An oid this city never wrote is [`Answer::Unavailable`], for the
    /// reason [`Query::Changes`] gives: "I did not write it" and "it
    /// changed nothing" are different answers.
    Commit {
        oid: GitOid,
    },
    RunView {
        run: RunId,
    },
    CityView,
    ApprovalQueue,
    InboxView {
        addr: Address,
    },
    Metrics,
    CostView,
    ArchiveSearch {
        needle: String,
    },
    RegistryView,
    DiscardView,
    /// What is attached and what is chosen: the settings page's read.
    EndpointView,
    /// One building's own files and its archive - the pages an agent
    /// writes for the next agent, which are also the pages a person
    /// reads to know what happened in there.
    BuildingView {
        addr: Address,
    },
    /// Who answers for this city, and what has been answered on the
    /// person's behalf.
    ///
    /// Both halves in one answer because a reader needs both to make
    /// sense of either: a list of decisions with nobody named beside it
    /// does not say whether the person delegated them, and a delegation
    /// with nothing decided under it does not say whether it has ever
    /// been used.
    Governance,
    /// One session, folded into the rounds a person reads.
    ///
    /// Answered server-side, because the wire is the whole API
    /// (ARCHITECTURE.md section 8): a second client must be able to
    /// draw a session without reimplementing the fold.
    ///
    /// Bounded by [`HISTORY_MAX`](crate::HISTORY_MAX) records, which is
    /// the same slice the client used to ask for with
    /// [`Query::RunHistory`]: moving the fold must not quietly change
    /// how much of a session it can see.
    Rounds {
        run: RunId,
    },
    /// What one run left that somebody can check it by: the screenshots
    /// it stored and the completions it closed plan nodes with.
    ///
    /// Locators, never bytes. Fetching a picture is the asset
    /// endpoint's, for the reason [`Query::Hunks`] is separate from
    /// [`Query::Changes`]: one question must not carry the cost of
    /// every answer somebody might go on to want.
    Evidence {
        run: RunId,
    },
    /// What one plan node has cost, and which runs spent it.
    ///
    /// A node nobody claimed answers zero, not `Unavailable`: "no run
    /// has held this node" is a true answer. The money comes from
    /// `memory::attribution` and is priced nowhere but `gateway::cost`.
    CostOf {
        node: NodeId,
    },
    /// One directory of the city, one level deep; `None` is the root.
    ///
    /// The tree is the product - a building is a directory and a room
    /// is one inside it - and until this existed nothing on the wire
    /// could open a room. One level per question, so looking at a room
    /// never pays for the ledger segments beside it.
    Listing {
        at: Option<Address>,
    },
    /// One file of the city, bounded, with the cut stated.
    ///
    /// The path is an `Address`, so it cannot leave the city root; the
    /// reserved subtree is readable here on purpose, because what
    /// governs a building is part of what this view exists to show, and
    /// this door answers the person and not a resident.
    Document {
        at: Address,
    },
    /// The commits this city made, newest first, a page at a time.
    ///
    /// `building` keeps the commits whose actor worked at that address
    /// or under it - the actor is the authority and a session is its
    /// projection, so filtering by session would lose the runs nobody
    /// opened a session for. `before` is exclusive, as in
    /// [`Query::History`]; the next page asks with the last `seq` it was
    /// handed. Only commits this city wrote are listed: an oid a person
    /// rewrote into trunk by hand is not this city's, and it says so by
    /// leaving it out rather than reading a trailer back.
    Commits {
        building: Option<Address>,
        before: Option<Seq>,
        limit: u32,
    },
    /// What this machine has, and what this city still needs of it.
    ///
    /// Answered from what the city found when it started, not from a
    /// fresh look: every item is a program asked its version, and a
    /// query that started a dozen processes would hold the one thread
    /// that answers every other read. A city that has not looked
    /// answers [`Answer::Unavailable`], which is what a worker driven
    /// one command at a time is.
    Doctor,
    /// The system prompt one run was frozen with: four segments, their
    /// text, and what each was assembled from.
    ///
    /// Answered from the run's own `prompt_assembled` record joined to
    /// the store, never by assembling a prefix again: what a person
    /// needs to see is the bytes that were sent, and a second assembly
    /// taken now would read files that have moved since. A segment the
    /// store no longer holds says so by name rather than by an empty
    /// string, because "this segment was empty" and "these bytes are
    /// gone" are different answers.
    Prefix {
        run: RunId,
    },
    /// One object of the content store, bounded, with the cut stated.
    ///
    /// The general read behind [`Query::Prefix`], and the door every
    /// other `cas:` reference a page is shown can be opened through -
    /// an approval's artifact, a discarded file's way back, a norm on a
    /// handoff's must-read list. Only the `cas:` scheme is answered: a
    /// `file:` locator names a path in the tree, which is
    /// [`Query::Document`]'s question and must not have a second answer
    /// here.
    Content {
        locator: Locator,
    },
    /// What one building can do, and where each of those came from.
    ///
    /// Two shelves in one answer because a reader needs both to make
    /// sense of either: the city's library is what any building may
    /// admit, and the building's own shelf is what only it keeps. Which
    /// runs pinned a skill is folded from `run_started`, so a shelf
    /// nothing has ever used says so instead of looking unused because
    /// nobody wrote it down.
    Skills {
        building: Address,
    },
    /// What is uncommitted in one building right now: the branch, how
    /// far it has drifted from its upstream, the files that moved, and
    /// the last checkpoint the city fenced there.
    ///
    /// Read from git at the moment of asking, which is the one answer
    /// here that is about the disk rather than about the history: a
    /// working tree is what a person is looking at, and the Ledger
    /// records fences rather than edits. The checkpoint beside it comes
    /// from the history, for the reason [`Query::Commit`] gives.
    GitStatus {
        building: Address,
    },
    /// Whether each tool server one address reaches is answering, and
    /// what it offers.
    ///
    /// One handshake per configured server, run at the moment of
    /// asking: a server's state is a fact about now - a program that
    /// starts, a host that answers, an account that is still valid -
    /// and a remembered one would tell a person their server is up an
    /// hour after it stopped. It is the same handshake a run opens with
    /// (`protocol::handshake`, then `tools/list`), so what this answers
    /// and what a model is given cannot disagree.
    ///
    /// **This is the one query that costs seconds.** A page asks it
    /// when a person opens the MCP page or adds a server, never on a
    /// timer.
    McpHealth {
        addr: Address,
    },
    /// Which outside applications the broker offers, and where each one
    /// stands for this city.
    ///
    /// **The second query that costs a round trip to somebody else**,
    /// and it is asked on the same terms as [`Query::McpHealth`]: when
    /// a person opens the page, and when they come back to it from the
    /// consent page they were sent to. Never on a timer - a city that
    /// asked the broker "anything new?" on a schedule would be
    /// generating traffic nobody reads, which `docs/third-party.md`
    /// rules out.
    ///
    /// Carries no key: the project key is enrolled in the vault and
    /// redeemed on the host machine, so a frame from a socket names
    /// nothing secret and an unenrolled city answers
    /// `ToolkitsAnswer::Unenrolled` rather than failing.
    Toolkits,
}

impl Query {
    /// Exhaustive, for the same reason as [`Command::name`].
    #[must_use]
    pub fn name(&self) -> &'static str {
        match *self {
            Self::History { .. } => "History",
            Self::RunHistory { .. } => "RunHistory",
            Self::Changes { .. } => "Changes",
            Self::Commit { .. } => "Commit",
            Self::RunView { .. } => "RunView",
            Self::CityView => "CityView",
            Self::ApprovalQueue => "ApprovalQueue",
            Self::InboxView { .. } => "InboxView",
            Self::Metrics => "Metrics",
            Self::CostView => "CostView",
            Self::ArchiveSearch { .. } => "ArchiveSearch",
            Self::RegistryView => "RegistryView",
            Self::DiscardView => "DiscardView",
            Self::EndpointView => "EndpointView",
            Self::BuildingView { .. } => "BuildingView",
            Self::Governance => "Governance",
            Self::Hunks { .. } => "Hunks",
            Self::Rounds { .. } => "Rounds",
            Self::Evidence { .. } => "Evidence",
            Self::CostOf { .. } => "CostOf",
            Self::Listing { .. } => "Listing",
            Self::Document { .. } => "Document",
            Self::Commits { .. } => "Commits",
            Self::Doctor => "Doctor",
            Self::Prefix { .. } => "Prefix",
            Self::Content { .. } => "Content",
            Self::Skills { .. } => "Skills",
            Self::GitStatus { .. } => "GitStatus",
            Self::McpHealth { .. } => "McpHealth",
            Self::Toolkits => "Toolkits",
        }
    }
}
