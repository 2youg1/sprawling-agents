// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One session read as the rounds a person reads.
//!
//! These are the shapes only; the fold that produces them is
//! `bin::views::rounds`, and reading one payload into them is
//! [`crate::reading`]. Three files because they are three shapes
//! (ARCHITECTURE.md section 9): a value, a projection, and a decision.
//!
//! They live on the wire because a second client must be able to draw a
//! session without folding the ledger itself. Until card-6.5 the fold
//! was `web::turn`, where nothing but the WebAssembly client could reach
//! it.

use kernel::{AxError, GitOid, RunId, Seq, TimeMs, Tokens, UsdMicros};
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
    /// A checkpoint fence went up, and this is the commit it made. It is
    /// what a change list is addressed by.
    Fenced { oid: GitOid, at: Seq },
    /// This turn stopped for a person. What waits and who answers is the
    /// approval queue's; copying it here would be a third authority.
    Waiting { at: Seq },
    /// A word arrived - from the person watching, or from another
    /// address that reached this one.
    Arrived { from: String, said: String, at: Seq },
    /// Files went away. Every one carries its way back, which is the
    /// Recycle Bin's to state.
    Discarded { count: usize, at: Seq },
}

impl Note {
    /// Where in the Ledger this note is, so every row can be read
    /// further.
    #[must_use]
    pub fn at(&self) -> Seq {
        match *self {
            Self::Refused { at, .. }
            | Self::Fenced { at, .. }
            | Self::Waiting { at }
            | Self::Arrived { at, .. }
            | Self::Discarded { at, .. } => at,
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
    pub cached: Tokens,
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
    pub outcome: Outcome,
    /// Where in the Ledger the bytes are. The row shows a shape; this is
    /// how somebody reads the rest.
    pub at: Seq,
    /// What it said, bounded. `None` when the call has not answered, or
    /// when the result carried nothing this build can read as text.
    pub output: Option<Output>,
}

/// One turn: the model was asked, and this is what came of it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Turn {
    /// Counted from one, in the order the turns opened.
    pub number: u32,
    /// The event that opened it.
    pub opened: Seq,
    /// What the model said in this turn, thinking blocks left out: they
    /// are carried end to end for the provider's signature check and are
    /// not a page's to render.
    pub said: Option<String>,
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
    /// first fence, because that is the tree as the work found it.
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
}
