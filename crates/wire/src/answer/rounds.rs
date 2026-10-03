// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One session read as the rounds a person reads.
//!
//! These are the shapes only; the fold that produces them is
//! `accounting::views::rounds`, and reading one payload into them is
//! [`crate::reading`]. Three files because they are three shapes
//! (ARCHITECTURE.md section 9): a value, a projection, and a decision.
//!
//! They live on the wire because a second client must be able to draw a
//! session without folding the ledger itself.

use kernel::{AxError, GitOid, Locator, RunId, Seq, TimeMs, Tokens, UsdMicros};
use serde::{Deserialize, Serialize};

/// What a tool call has come to so far.
///
/// Three states rather than a `bool` and an `Option`: a call still
/// running and a call that failed are different things to a person
/// deciding whether to step in, and the pair could spell a fourth state
/// that cannot happen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Outcome {
    /// Called, and no result has arrived in this window.
    Waiting,
    /// Answered.
    Answered,
    /// Answered with an error. **Not an alert**: one failed call is a
    /// fact, not a request for a person. If it actually stopped the
    /// session, the freeze raises its own card.
    Failed,
}

/// Whether the span between two times on a row is a measurement.
///
/// Two values because a page does one of two things with a row: draw
/// how long it took, or draw no duration at all. The times themselves
/// travel either way; they still give the row its order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Timing {
    /// Every time on the row is the moment its own record measured
    /// (`kernel::event::moment`).
    Measured,
    /// At least one is not: a line written before each line recorded
    /// its own moment carries its turn's stamp, which every line of that
    /// turn shares, and an answer the city wrote itself after a restart
    /// (`E_TOOL_OUTCOME_UNKNOWN`) carries when the city wrote it.
    Unmeasured,
}

/// What a tool said, bounded so a wave of output cannot become the page.
///
/// The cut is counted rather than hinted at: a reader who cannot see how
/// much was withheld is being dumped on slowly. Bytes already too large
/// were replaced by `runtime::offload` before they reached the Ledger,
/// and that substitute carries its own line naming the `Locator`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Output {
    /// The first [`crate::OUTPUT_LINES`] lines of it.
    pub head: String,
    /// How many lines this view cut. The rest is in the Ledger at the
    /// call's own `at`.
    pub cut: usize,
    /// Where the whole of a call's output was stored when it left the
    /// window: the bytes the command itself wrote, even when the sieve
    /// cut them first and the cut was stored again. `None` when nothing
    /// left the window, on a line written before results carried their
    /// accounts, and always on a call's arguments, which never leave.
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub pinned: Option<Locator>,
}

/// What a turn came to besides the calls it made.
///
/// **The criterion is closed on purpose**: an event earns a `Note` when
/// it changed what this turn did, or what it is waiting on. Everything
/// else stays in the event stream, which is the Ledger's shape rather
/// than a reader's. Without that line this enum would grow to one arm
/// per event kind and stop meaning anything.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Note {
    /// A door refused something. The error travels whole because the
    /// interface has one place where a refusal becomes the three parts a
    /// person needs; taking it apart here would be the second.
    Refused { error: AxError, at: Seq },
    /// A checkpoint went up, and this is the commit it made. It is
    /// what a change list is addressed by.
    Checkpointed { oid: GitOid, at: Seq },
    /// This turn stopped for a person. What waits and who answers is the
    /// approval queue's; copying it here would be a third authority.
    /// `t` is when the Ledger recorded the request and `answered` when
    /// it recorded the answer, paired back by approval id from the
    /// city's own run; `None` when the answer is outside that window or
    /// has not come, and the page then draws no guessed end.
    Waiting {
        at: Seq,
        t: TimeMs,
        answered: Option<TimeMs>,
    },
    /// A word arrived - from the person watching, or from another
    /// address that reached this one.
    Arrived { from: String, said: String, at: Seq },
    /// Files went away. Every one carries its way back, which is the
    /// Recycle Bin's to state.
    Discarded { count: usize, at: Seq },
    /// A record of a kind that earns a note, whose payload did not read
    /// back as that kind. The failure stays visible here instead of the
    /// turn reading as if nothing happened; `cause` names the kind and
    /// what the reading stopped at.
    Unreadable { cause: String, at: Seq },
}

impl Note {
    /// Where in the Ledger this note is, so every row can be read
    /// further.
    #[must_use]
    pub fn at(&self) -> Seq {
        match *self {
            Self::Refused { at, .. }
            | Self::Checkpointed { at, .. }
            | Self::Waiting { at, .. }
            | Self::Arrived { at, .. }
            | Self::Discarded { at, .. }
            | Self::Unreadable { at, .. } => at,
        }
    }
}

/// What one turn cost in tokens.
///
/// Absolute counts and no ratio: the numerator is on the wire and the
/// denominator - this model's context window - is not. A percentage with
/// no denominator is the thing `UnplannedProgress` already refuses to
/// spell, and it would be no more honest here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Used {
    pub input: Tokens,
    pub output: Tokens,
    /// The part of `input` read from the provider's cache
    /// (`kernel::ModelUsage::cache_read_tokens`).
    pub cached: Tokens,
    /// The part of `input` written into the provider's cache
    /// (`kernel::ModelUsage::cache_write_tokens`), priced apart from a
    /// read. Always present beside the other three in this build; absent
    /// only in a frame from a city written before it was answered
    /// (`crates/wire/Spec.lean` D13).
    #[serde(default)]
    pub cache_write: Option<Tokens>,
    /// Whole microseconds from the attempt going out to the reply's first
    /// content, as `model_returned.first_us` recorded it (kernel D20).
    /// `None` on a line written before the key existed, and wherever
    /// `Turn::first_at` is `None`.
    #[serde(default)]
    pub first_us: Option<u64>,
    /// Whole microseconds from the attempt going out to the reply being
    /// whole, as `model_returned.took_us` recorded it. `None` on a line
    /// written before the key existed: a page then shows the difference
    /// of the moments, in milliseconds (`crates/wire/spec/Reading.lean` D28).
    #[serde(default)]
    pub took_us: Option<u64>,
}

/// One tool call inside a turn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Call {
    /// The tool's own name, as the Ledger records it.
    pub tool: String,
    /// What it acted on, when the arguments name one thing. A display
    /// reading rather than a field: the arguments are free JSON and this
    /// picks the one a person recognises the call by.
    pub subject: Option<String>,
    /// What it was called with, bounded exactly the way [`Output`] is and
    /// cut off at the same limit.
    ///
    /// A person judging a call needs what went in, not only what came
    /// back: "the tool failed" and "the tool was asked for the wrong
    /// path" are read from different halves of the same row. `subject`
    /// stays one display reading of these rather than a replacement for
    /// them. `None` when the recorded call carried no arguments this
    /// build can read as text; the whole of them is in the Ledger at
    /// `at`, which is where the cut lines went too.
    pub arguments: Option<Output>,
    pub outcome: Outcome,
    /// Where in the Ledger the bytes are. The row shows a shape; this is
    /// how somebody reads the rest.
    pub at: Seq,
    /// What it said, bounded. `None` when the call has not answered, or
    /// when the result carried nothing this build can read as text.
    pub output: Option<Output>,
    /// When the Ledger wrote the call, read from that record the way
    /// [`Turn::t`] is, so a replayed session keeps its original times.
    pub called: TimeMs,
    /// When the Ledger wrote the result paired with this call. `None`
    /// exactly when `outcome` is [`Outcome::Waiting`]: the two are set by
    /// one pairing, and an answer outside the window is not guessed.
    pub answered: Option<TimeMs>,
    /// Whether `answered - called` is how long the call took. One value
    /// for both times: two records a build writes are both measured or
    /// both not, and the one exception - an answer the city supplied
    /// after a restart - makes the span unmeasured either way.
    pub timing: Timing,
    /// What boundary the tool was registered as crossing when it was
    /// called, as its `tool_called` line recorded it. `None` for a tool
    /// the bench did not know, and for a line written before the key
    /// existed.
    pub effect: Option<kernel::Effect>,
    /// How the tool was registered to be drawn, from the same line: a
    /// terminal, a diff, or the generic row. `None` in the same cases;
    /// a page draws such a call as generic. A diff's `locations` are the
    /// registration's and empty today - the file is [`Call::subject`].
    pub render: Option<kernel::RenderIntent>,
    /// The code the command of an `exec` call ended with, as its paired
    /// result recorded it. `None` for every other tool, for a command a
    /// signal stopped or the city never waited on - which have no code -
    /// and for a call not yet answered: no code is drawn rather than an
    /// invented one.
    #[serde(default)]
    pub exit_code: Option<i64>,
    /// Whole microseconds the call took, as its paired `tool_result.took_us`
    /// recorded it (kernel D20). `None` for a call not yet answered and for
    /// a line written before the key existed: a page then shows
    /// `answered - called` in milliseconds (`crates/wire/spec/Reading.lean` D28).
    #[serde(default)]
    pub took_us: Option<u64>,
}

/// One turn: the model was asked, and this is what came of it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Turn {
    /// Counted from one, in the order the turns opened.
    pub number: u32,
    /// The event that opened it.
    pub opened: Seq,
    /// When the Ledger wrote the event `opened` names.
    ///
    /// Read from that record rather than from a clock consulted here:
    /// the Ledger is the authority on when a thing happened and this
    /// field is a projection of it, so a session replayed tomorrow
    /// reports the times it originally had rather than the times it was
    /// replayed.
    ///
    /// Whether it is the moment the model was asked, or the stamp a
    /// ledger written before per-line moments gave every line of the
    /// turn, is [`Turn::timing`].
    ///
    /// Not optional, unlike [`crate::LogLine::t`]. A log line is written
    /// beside the Ledger and can find the clock unreadable; a turn is
    /// folded from an `EventRecord`, which carries a reading in every
    /// case. An `Option` here would be a state no fold can produce and
    /// every reader would still have to answer.
    pub t: TimeMs,
    /// Whether [`Turn::t`] is the moment the model was asked.
    pub timing: Timing,
    /// When the reply's first content reached the city, as the turn's
    /// `model_returned` recorded it; the time to first content is this
    /// minus [`Turn::t`]. `None` when the reply had no stream, streamed
    /// only tool calls, or was written before the key existed - the page
    /// then draws no figure rather than a guessed one.
    pub first_at: Option<TimeMs>,
    /// When the reply that answered this turn was whole, as that
    /// `model_returned` measured its own moment
    /// (`kernel::event::moment`). The output rate is
    /// `used.output` over `returned - first_at`. `None` when no reply
    /// answered in this window and when the reply's line measured no
    /// moment of its own - the envelope stamp of such a line is the
    /// turn's, and a rate drawn from it would be invented.
    #[serde(default)]
    pub returned: Option<TimeMs>,
    /// The endpoint's model id the `model_called` that opened this turn
    /// recorded. Per turn because a session can change model midway,
    /// and a name for the whole session would be wrong for half of it.
    /// `None` when that record carries no model name as text; the page
    /// then draws no name rather than a guessed one.
    pub model: Option<String>,
    /// What the model said in this turn: its prose, without the
    /// reasoning that produced it.
    pub said: Option<String>,
    /// What the model reasoned in this turn, when the provider sent it.
    /// A field of its own rather than part of `said`, because a page
    /// folds the two differently and a person asked for an answer.
    pub thought: Option<String>,
    /// What this one turn was billed.
    pub spent: Option<UsdMicros>,
    pub used: Option<Used>,
    /// Why the model stopped, in the provider's own word.
    pub stopped: Option<String>,
    pub calls: Vec<Call>,
    /// What else happened inside this turn, oldest first.
    pub notes: Vec<Note>,
}

/// How a session began: what the person asked for, in their words.
///
/// The rounds start at the first `model_called`, so without this the
/// first thing said in a conversation - the task - is the one thing the
/// answer never carried.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Opening {
    pub task: String,
    pub goal: String,
    pub at: TimeMs,
    /// Who dispatched the run, as its `run_started` records it; `None`
    /// for a ledger written before the key existed.
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub dispatched_by: Option<kernel::event::Who>,
    /// The run policy it was dispatched under - mode, write limit,
    /// admission requirement and landing policy - as its `run_started`
    /// records it; `None` for a line written before the key existed.
    #[serde(default)]
    pub policy: Option<kernel::RunPolicy>,
    /// The effort the run's requests froze, as its `run_started`
    /// records it (`crates/kernel/Spec.lean` §8-85); `None` when the
    /// dispatch stated none, and for a line written before the key
    /// existed.
    #[serde(default)]
    pub effort: Option<kernel::Effort>,
    /// The names the run's session froze, read back from the version its
    /// `run_started` names (`crates/wire/Spec.lean` D17). `None` for a
    /// line written before the city recorded names, and for a version
    /// the store no longer holds or cannot read: the page then falls
    /// back to the address and the role's name, never to today's names.
    #[serde(default)]
    pub names: Option<FrozenNames>,
}

/// The names a session froze, as a page draws them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct FrozenNames {
    /// What the Mayor was called; `None` when nobody had named it, and
    /// the page then draws the name its language table ships.
    pub mayor: Option<String>,
}

/// How a session ended, in the word the run froze with: `done`,
/// `limit` or `cancelled`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Closing {
    pub completion: String,
    pub at: TimeMs,
}

/// One session's rounds, oldest first.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RoundsAnswer {
    pub run: RunId,
    pub turns: Vec<Turn>,
    /// The checkpoint this session's changes are measured from: its
    /// first checkpoint, because that is the tree as the work found it.
    /// Measuring from the latest one would answer "what moved in the
    /// last wave", which is a different question.
    pub opened_at: Option<GitOid>,
    /// Absent when the window this answer reads did not hold the
    /// session's `run_started`; what the window does not hold is not
    /// guessed.
    pub opening: Option<Opening>,
    /// Absent while the session is still going, or when the window did
    /// not reach its `run_frozen`.
    pub closing: Option<Closing>,
    /// The name of the worktree this run was lent, from its own
    /// `worktree_opened`: a name, never a path, which is a fact about one
    /// machine. Absent for a run that worked in its building's own tree,
    /// and when the window did not reach that line.
    #[serde(default)]
    pub worktree: Option<String>,
}
