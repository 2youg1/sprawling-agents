// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Reading the payload: what was said, what it cost, what each call came to.

use channels::{AxError, EventKind, EventRecord, GitOid, Seq, Tokens, UsdMicros};

/// What a tool call has come to so far.
///
/// Three states rather than a `bool` and an `Option`: a call still
/// running and a call that failed are different things to a person
/// deciding whether to step in, and the pair could spell a fourth state
/// that cannot happen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

impl Outcome {
    /// The word for this outcome, as a message rather than a string, so a
    /// state cannot be the one English word left on a Chinese page.
    #[must_use]
    pub fn word(self) -> crate::lang::Msg {
        match self {
            Self::Waiting => crate::lang::Msg::TurnWaiting,
            Self::Answered => crate::lang::Msg::TurnAnswered,
            Self::Failed => crate::lang::Msg::TurnFailed,
        }
    }

    /// The class a row takes, so lightness and a word carry the state
    /// together - colour is a redundant layer here as everywhere.
    #[must_use]
    pub fn class(self) -> &'static str {
        match self {
            Self::Waiting => "out waiting",
            Self::Answered => "out answered",
            Self::Failed => "out failed",
        }
    }
}

/// What a tool said, bounded so a wave of output cannot become the page.
///
/// The cut is counted rather than hinted at: §8-47 admits disclosure and
/// refuses dumping, and a reader who cannot see how much was withheld is
/// being dumped on slowly. Bytes already too large were replaced by
/// `runtime::offload` before they reached the Ledger, and that substitute
/// carries its own line naming the `Locator` - so this shows what it was
/// given and never parses that line, which would be a second authority
/// for the substitute's format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    /// The first [`OUTPUT_LINES`] lines of it.
    pub head: String,
    /// How many lines this view cut. The rest is in the Ledger at the
    /// call's own `at`.
    pub cut: usize,
}

/// The most lines of one tool's output a row carries.
pub const OUTPUT_LINES: usize = 12;

/// What a turn came to besides the calls it made.
///
/// **The criterion is closed on purpose**: an event earns a `Note` when
/// it changed what this turn did, or what it is waiting on. Everything
/// else stays in the event stream, which is the Ledger's shape rather
/// than a reader's (§8-6). Without that line this enum would grow to
/// fifty-eight arms and stop meaning anything.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Note {
    /// A door refused something. The error travels whole because
    /// `web::alert::refused` is the one place a refusal becomes the three
    /// parts a person needs; taking it apart here would be the second.
    Refused { error: AxError, at: Seq },
    /// A checkpoint fence went up, and this is the commit it made.
    ///
    /// A `GitOid` and not the string it arrived as: the parse is
    /// fail-closed on exactly forty lowercase hex digits, so a payload
    /// this build cannot read produces no row rather than a row nothing
    /// can be asked about. It is what the change list is addressed by.
    Fenced { oid: GitOid, at: Seq },
    /// This turn stopped for a person. What waits and who answers is
    /// `web::approval`'s; copying it here would be a third authority.
    Waiting { at: Seq },
    /// A word arrived - from the person watching, or from another
    /// address that reached this one.
    Arrived { from: String, said: String, at: Seq },
    /// Files went away. Every one carries its way back, which is the
    /// Recycle Bin's to state.
    Discarded { count: usize, at: Seq },
}

impl Note {
    /// Where in the Ledger this note is, so every row can be read further.
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Used {
    pub input: Tokens,
    pub output: Tokens,
    pub cached: Tokens,
}

/// One tool call inside a turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Call {
    /// The tool's own name, as the Ledger records it.
    pub tool: String,
    /// What it acted on, when the arguments name one thing.
    ///
    /// A display reading rather than a field: the arguments are free JSON
    /// and this picks the one a person recognises the call by. `None`
    /// prints the tool alone, which is what the old line did for every
    /// call.
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Turn {
    /// Counted from one, in the order the turns opened.
    pub number: u32,
    /// The event that opened it.
    pub opened: Seq,
    /// What the model said in this turn, thinking blocks left out: they
    /// are carried end to end for the provider's signature check and are
    /// not this page's to render.
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

/// Argument names that say what a call acted on, in the order they are
/// preferred.
///
/// Taken from the tool definitions rather than guessed: `path` is what
/// twelve of them take, and the rest name their one subject.
const SUBJECT_KEYS: [&str; 4] = ["path", "addr", "program", "arm"];
pub(crate) fn text(value: Option<&serde_json::Value>) -> Option<String> {
    value.and_then(|held| held.as_str()).map(str::to_owned)
}

/// What the model said, out of the message `runtime::turn` recorded.
///
/// Text blocks only. `Thinking` and `RedactedThinking` are carried end to
/// end so the provider can verify the signature it issued; relaying them
/// is this city's job and publishing them is not.
///
/// Visible to the crate because the sessions list needs the same
/// sentence the session page shows: a row saying one thing and the page
/// under it saying another would be two readings of one record.
pub(crate) fn said_in(message: &serde_json::Value) -> Option<String> {
    let blocks = message.as_object()?.get("content")?.as_array()?;
    let said: Vec<&str> = blocks
        .iter()
        .filter_map(|block| {
            let map = block.as_object()?;
            (map.get("kind")?.as_str()? == "text").then(|| map.get("text")?.as_str())?
        })
        .collect();
    (!said.is_empty()).then(|| said.join("\n"))
}

/// The four counters `ModelUsage` carries. Absent when the provider sent
/// no usage, which is a different fact from having spent nothing.
pub(crate) fn used_in(usage: &serde_json::Value) -> Option<Used> {
    let map = usage.as_object()?;
    let counter = |name: &str| {
        map.get(name)
            .and_then(serde_json::Value::as_u64)
            .map(Tokens::new)
    };
    let input = counter("input_tokens")?;
    let output = counter("output_tokens")?;
    Some(Used {
        input,
        output,
        cached: counter("cache_read_tokens").unwrap_or_default(),
    })
}

/// What a tool said, cut to [`OUTPUT_LINES`] with the remainder counted.
///
/// A result too large to carry was replaced upstream by
/// `runtime::offload`, whose substitute already states its own size and
/// names the `Locator` holding the rest. This never reads that line: the
/// substitute's format has one authority and it is not this module.
pub(crate) fn output_in(said: &serde_json::Value) -> Option<Output> {
    let whole = match said {
        serde_json::Value::String(text) => text.clone(),
        other => other.to_string(),
    };
    if whole.trim().is_empty() {
        return None;
    }
    let head: Vec<&str> = whole.lines().take(OUTPUT_LINES).collect();
    let cut = whole.lines().count().saturating_sub(head.len());
    Some(Output {
        head: head.join("\n"),
        cut,
    })
}

/// Whether this kind changed what the turn did or what it waits on, and
/// what to say about it if so.
///
/// Exhaustive over the kinds that earn a note and closed against the
/// rest: the criterion is in web-SPEC.md section 8-48, and an enum that
/// grew an arm per event would be the event stream with extra steps.
pub(crate) fn note_of(kind: EventKind, record: &EventRecord) -> Option<Note> {
    let at = record.seq();
    let map = record.data().as_map();
    match kind {
        // The carrier table in `kernel::error` decides which codes land
        // under which kind, and `runtime::run` writes the error flat into
        // the payload. A payload that will not read back as one is left
        // to the event stream rather than rendered as a refusal this
        // build invented.
        EventKind::GateDenied
        | EventKind::BudgetLimit
        | EventKind::WatchdogFired
        | EventKind::ProviderDegraded => {
            let value = serde_json::Value::Object(map.clone());
            serde_json::from_value(value)
                .ok()
                .map(|error| Note::Refused { error, at })
        }
        EventKind::ApprovalRequested => Some(Note::Waiting { at }),
        EventKind::CheckpointCommitted => text(map.get("oid"))
            .as_deref()
            .and_then(GitOid::parse)
            .map(|oid| Note::Fenced { oid, at }),
        EventKind::SteerReceived | EventKind::SignalConsumed => Some(Note::Arrived {
            from: text(map.get("source"))
                .or_else(|| text(map.get("from")))
                .unwrap_or_else(|| record.who().to_owned()),
            said: text(map.get("text")).unwrap_or_default(),
            at,
        }),
        EventKind::FileDiscarded => Some(Note::Discarded {
            count: map
                .get("paths")
                .and_then(serde_json::Value::as_array)
                .map_or(1, Vec::len),
            at,
        }),
        _ => None,
    }
}

/// The one argument a person recognises a call by.
pub(crate) fn subject_of(args: Option<&serde_json::Value>) -> Option<String> {
    let map = args?.as_object()?;
    for key in SUBJECT_KEYS {
        if let Some(named) = text(map.get(key)) {
            return Some(named);
        }
    }
    // A tool this build has no preferred key for still says something,
    // rather than falling back to the bare tool name.
    map.values().find_map(|value| text(Some(value)))
}
