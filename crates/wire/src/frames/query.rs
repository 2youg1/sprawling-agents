// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Everything a page may ask, and the name table the schema hash is
//! built from; `frames` says what every Query shares.

use kernel::{Address, B3Hash, GitOid, Locator, NodeId, RunId, Seq};
use serde::{Deserialize, Serialize};

use crate::named_frames::named_frames;

named_frames! {
/// A read of the city; the module documentation says what all of them share.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Query {
    /// A bounded slice of the one history, ending just before `before`
    /// or at the tail when that is absent.
    ///
    /// The server broadcasts what happens next and never what happened,
    /// so a page that only listened would see a city that had been
    /// running for a month as an empty one. Bounded because the whole ledger is not a
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
    /// that started before the tab did is not in it at all.
    /// [`Query::RunView`] says whether a run exists and where it got to,
    /// not what happened in it. Answered with [`HistoryAnswer`], because
    /// "a page of history" already has a shape.
    RunHistory {
        run: RunId,
        before: Option<Seq>,
        limit: u32,
    },
    /// The records of the one history between two sequence numbers, both
    /// ends included: the question a page asks when the event stream tells
    /// it what it skipped.
    ///
    /// [`Query::History`] walks backwards from a cursor, which answers
    /// "what came before this" and cannot answer "what happened between
    /// these two". A gap has a near end as well as a far one, and a page
    /// that walked to the gap's near end from the tail would pay for every
    /// record in between.
    ///
    /// Answered with [`HistoryRangeAnswer`](crate::HistoryRangeAnswer)
    /// rather than with [`HistoryAnswer`](crate::HistoryAnswer): the
    /// question carries no cursor a reader holds on to, so the answer has
    /// to name the slice it is, and a page filling a gap while its record
    /// view is open is asking both questions at once. A range the Ledger
    /// holds more records for than `limit` allows is answered up to the
    /// limit, and the answer says where to ask next.
    HistoryRange {
        from: Seq,
        to: Seq,
        limit: u32,
    },
    /// The stretches of one room, newest first, which a page loses on a
    /// reload: [`SessionsAnswer`](crate::SessionsAnswer) (`crates/wire/spec/Answer/Sessions.lean` §8-71).
    Sessions {
        room: Address,
    },
    /// What moved between two checkpoints: paths and counts, never patch
    /// text.
    ///
    /// The caller names both ends because it already knows them - a
    /// checkpoint's oid is in the `checkpoint_committed` payload the
    /// client folded - and computing the pair a second time on the
    /// server would be a second answer to "which checkpoints belong to this
    /// session". Both oids are immutable, so the answer is cacheable
    /// forever by anybody who wants to.
    ///
    /// `head` absent means the working tree: a wave still running has
    /// written files no checkpoint holds yet, and a list that ignored
    /// them would describe the session as it was one checkpoint ago.
    Changes {
        base: GitOid,
        head: Option<GitOid>,
    },
    /// The patch text of one file between two checkpoints.
    ///
    /// **A separate frame from [`Query::Changes`], and that is the
    /// point.** `Changes` costs what the number of changed files costs
    /// and answers which files moved; this costs what one file costs
    /// and answers what moved inside it, the request of its own that
    /// `storage::changes` says a hunk needs.
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
    /// The vendors this city knows by host, and the base URL of each
    /// face they document, so a person picks a vendor rather than
    /// pasting its address.
    KnownHosts,
    /// The official harnesses, the command that starts each as an ACP
    /// agent, and whether this machine can run it: the harness page.
    Harnesses,
    /// One building's own files and its archive - the pages an agent
    /// writes for the next agent, which are also the pages a person
    /// reads to know what happened in there.
    BuildingView {
        addr: Address,
    },
    /// What the city calls the person and the Mayor, read from the two
    /// identity areas at the moment of asking (`crates/wire/spec/Answer/Identity.lean` §8-59).
    Identity,
    /// The schedule and the watch table, read at the moment of asking:
    /// shown on the page, written by hand (`crates/wire/spec/Answer/Automation.lean` §8-62).
    Automation,
    /// A candidate user id: gh's login for a host (`crates/wire/spec/Answer/Github.lean` §8-67).
    GithubLogin(Option<String>),
    /// The first-run guide's progress (`crates/wire/spec/Guide.lean` §8-68).
    Guide,
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
    /// the same slice [`Query::RunHistory`] answers: where the fold runs
    /// must not change how much of a session it can see.
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
    /// `storage::attribution` and is priced nowhere but `gateway::cost`.
    CostOf {
        node: NodeId,
    },
    /// One directory of the city, one level deep; `None` is the root.
    ///
    /// The tree is the product - a building is a directory and a room
    /// is one inside it - and this is the frame that opens a room. One level per question, so looking at a room
    /// never pays for the ledger segments beside it.
    Listing {
        at: Option<Address>,
    },
    /// Files under `under` whose name holds `text` (`crates/wire/spec/Answer/Find.lean` §8-82).
    Find { under: Address, text: String },
    /// One file of the city as a version, with its first window
    /// (`crates/wire/spec/Answer/Document.lean` §8-69). The path is an `Address`, so it cannot leave the
    /// city root; the reserved subtree is readable on purpose, because
    /// this door answers the person and not a resident.
    Document {
        at: Address,
    },
    /// The proposal cards still open on one document (`crates/wire/spec/Answer/Proposals.lean` §8-73).
    Proposals(Address),
    /// Every proposal card still open in the city, by document and
    /// offer time (`crates/wire/spec/Answer/Proposals.lean` §8-73).
    OpenProposals,
    /// One window of a stored version, by its version (`crates/wire/spec/Answer/Range.lean` §8-70).
    Range { version: B3Hash, range: documents::Span },
    /// One document's versions, newest first (`crates/wire/spec/Answer/DocumentVersions.lean` §8-83).
    Versions { at: Address },
    /// One window of a stored object's bytes (`crates/wire/spec/Answer/DocumentBytes.lean` §8-80).
    Bytes { version: B3Hash, range: documents::Span },
    /// A stored Markdown version as one HTML file (`crates/wire/spec/Answer/DocumentBytes.lean` §8-81).
    Export { at: Address, version: B3Hash },
    /// One window of a stored Markdown version, laid out (`crates/wire/spec/Answer/Preview.lean` §8-74).
    Preview { version: B3Hash, viewport: documents::Span },
    /// A reply's text, laid out by the preview's grammar (`crates/wire/spec/Answer/Preview.lean` §8-75).
    Reply {
        text: String,
        state: documents::ReplyState,
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
    /// Every shelf in one answer because a reader needs them together
    /// to make sense of any: a row carries [`crate::SkillShelf`], which
    /// names the shelves and where each row sits on one. Which runs
    /// pinned a skill is folded from `run_started`, so a shelf nothing
    /// has ever used says so instead of looking unused because nobody
    /// wrote it down.
    Skills { building: Address },
    /// What is uncommitted in one building right now: the branch, how
    /// far it has drifted from its upstream, the files that moved, and
    /// the last checkpoint the city checkpointed there.
    ///
    /// Read from git at the moment of asking, which is the one answer
    /// here that is about the disk rather than about the history: a
    /// working tree is what a person is looking at, and the Ledger
    /// records checkpoints rather than edits. The checkpoint beside it comes
    /// from the history, for the reason [`Query::Commit`] gives.
    GitStatus { building: Address },
    /// Whether each tool server one address reaches is answering, and
    /// what it offers.
    ///
    /// One handshake per configured server, run at the moment of
    /// asking: a server's state is a fact about now - a program that
    /// starts, a host that answers, an account that is still valid -
    /// and a remembered one would tell a person their server is up an
    /// hour after it stopped. It is the same handshake a run opens with
    /// (`agent_protocols::handshake`, then `tools/list`), so what this answers
    /// and what a model is given cannot disagree.
    ///
    /// **This is the one query that costs seconds.** A page asks it
    /// when a person opens the MCP page or adds a server, never on a
    /// timer.
    McpHealth {
        addr: Address,
    },
    /// Which outside applications the broker offers, and where each one stands for this city.
    ///
    /// **The second query that costs a round trip to somebody else**, and it is asked on the same
    /// terms as [`Query::McpHealth`]: when a person opens the page, and when they come back to it
    /// from the consent page they were sent to. Never on a timer - a city that asked the broker
    /// "anything new?" on a schedule would be generating traffic nobody reads, which
    /// `docs/third-party.md` rules out.
    ///
    /// Carries no key: the project key is enrolled in the vault and redeemed on the host machine,
    /// so a frame from a socket names nothing secret and an unenrolled city answers
    /// `ToolkitsAnswer::Unenrolled` rather than failing.
    Toolkits,
    /// Which release this city is running, and which ones npm and crates.io offer.
    ///
    /// **Asked when a person presses the button, and at no other time**: not on connect, not on a
    /// timer, not folded into another query. `QUICKSTART.md` promises that nothing outside the
    /// folder was written, and a city that reached a registry on its own schedule would spend that
    /// promise on a question nobody asked; `docs/third-party.md` rules out the same traffic.
    ///
    /// The slowest query here, and the only one whose cost a person chose. A registry that cannot
    /// be reached is [`ReleaseAnswer::Refused`](crate::ReleaseAnswer::Refused) rather than
    /// [`Answer::Unavailable`](crate::Answer::Unavailable): the city is available, the registry is
    /// not. Nothing it answers updates anything; this reports and stops.
    NewestRelease,
    /// Everything this person settled about their own reading of the
    /// city: the language, the appearance, the chords they rebound.
    ///
    /// The whole table in one answer, because a browser that cached
    /// these row by row had a dozen names for them and read three of
    /// those names back with its own idea of what a missing one means.
    /// The file is the authority and this is the only door to it.
    Preferences,
    /// What one address is actually governed by, value by value, with
    /// the file each value came from.
    ///
    /// **The layer is half the answer.** The three files form a ladder
    /// - the city's, the building's, the room's - and a page told only
    /// the resolved figure cannot say whether it is looking at
    /// something this address states or something it inherited, so it
    /// would have to read all three and climb the ladder a second
    /// time. Two climbs of one ladder is two answers to one question.
    Config {
        addr: Address,
    },
    /// What each named run has been billed, for the runs a cost view's
    /// `by_run` leaves out. At most
    /// [`RUN_COSTS_MAX`](crate::RUN_COSTS_MAX) runs are answered per
    /// question and the rest go unanswered, so a directory that names
    /// its runs newest first shows the cost of the newest cold runs; the
    /// money is folded from the Ledger for a run the city no longer
    /// holds warm, so the cost is proportional to the runs asked about.
    RunCosts {
        runs: Vec<RunId>,
    },
    /// The newest release of one requirement-table item, asked of its
    /// publisher (`crates/sprawling/Spec.lean` §8-120).
    ///
    /// Asked by the dependency page for each item once the page is open,
    /// which is the person asking whether their tools are behind; the
    /// city asks nothing on its own schedule. One item per question, so
    /// a slow publisher holds up its own row and no other. A source that
    /// cannot be reached is [`DoctorNewest::Refused`](crate::DoctorNewest::Refused)
    /// rather than [`Answer::Unavailable`](crate::Answer::Unavailable).
    UpstreamVersion {
        item: String,
    },
    /// Every skill's contents, audit and uses by day, folded from the whole ledger when the
    /// skill page opens (`crates/wire/spec/Reading.lean` D28, D33); `skill` narrows it to one.
    SkillUsage { skill: Option<String> },
    /// The same table per tool server and tool; `server` narrows it to one.
    McpUsage { server: Option<String> },
    /// Every use as one JSONL or CSV row for the User to download; the city writes no file.
    UsageExport { what: crate::UsageKind, format: crate::ExportFormat },
    /// Every shell interpreter's calls and failures by class, folded from the whole ledger in
    /// the same pass as the usage tables (`crates/wire/spec/Reading.lean` D48).
    Shells,
}

/// The Query surface, in declaration order — the order the handshake
/// hash mixes these names in.
pub const QUERY_NAMES;
}
