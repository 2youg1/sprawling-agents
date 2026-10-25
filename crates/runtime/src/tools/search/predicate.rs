// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What one search call asks, read from the arguments of Claude Code's
//! `Grep` (`crates/runtime/spec/Tools/Search.lean` §8-30-2).
//!
//! **A regular expression that never backtracks** (D120). The pattern
//! is compiled by the `regex` crate, the engine ripgrep uses by default,
//! so a model writes it in the syntax it writes for `Grep`; the crate
//! matches with finite automata in time linear in the line, so a pattern
//! a model wrote wrong cannot stall the turn behind it.
//!
//! **Refused where ignoring would mislead** (D121). `multiline`, `type`
//! and a nonzero `offset` are refused, because a search that left them
//! unread would answer wrongly rather than at more length; `-n`, `-o`
//! and `head_limit` are left unread.

use globset::{GlobBuilder, GlobMatcher};
use kernel::{AxCode, AxError, ErrorDraft, ToolCall};
use regex::{Regex, RegexBuilder};
use serde_json::{Map, Value};

/// How many lines of context each side may be asked for.
const CONTEXT_CAP: u8 = 4;

/// What one call is looking for, settled once from the arguments so the
/// walk carries one value.
pub(super) enum Predicate {
    /// The lines a pattern matches, in the files the glob, if any, names.
    Lines(Lines, Option<Names>),
    /// Only the files a glob names, opened by nobody (D117).
    Names(Names),
}

/// The question a pattern asks of each line, and how it is answered.
pub(super) struct Lines {
    pub(super) pattern: Regex,
    pub(super) mode: Mode,
    pub(super) context: Context,
}

/// How many lines a hit carries before and after its own.
#[derive(Clone, Copy)]
pub(super) struct Context {
    pub(super) before: usize,
    pub(super) after: usize,
}

/// What a call hands back for the files it matched
/// (`crates/runtime/spec/Tools/Search.lean` D109).
#[derive(Clone, Copy)]
pub(super) enum Mode {
    /// Only the files that hold a match.
    Files,
    /// Every match, with its line number and context.
    Content,
    /// Each file that holds a match, with how many lines matched.
    Count,
}

/// The files a glob admits by name (D117).
pub(super) struct Names {
    matcher: GlobMatcher,
    /// A glob with a `/` in it is matched against the path below the
    /// search's `path`; one without, against the file's own name.
    by_path: bool,
}

impl Predicate {
    /// What `call` asks, or why it is not a question this search answers.
    ///
    /// # Errors
    /// `E_INVALID_ARGS` when neither `pattern` nor `glob` is given, when
    /// either does not compile, when an argument has the wrong type, when
    /// `output_mode` is not one of the three, when a glob alone asks for
    /// lines, and for the arguments D121 refuses.
    pub(super) fn asked_by(call: &ToolCall) -> Result<Predicate, AxError> {
        let args = call.args.as_map();
        refuse_misleading(args)?;
        let names = args.get("glob").map(Names::of).transpose()?;
        let mode = Mode::asked(args)?;
        match (args.get("pattern"), names, mode) {
            (Some(pattern), within, mode) => Ok(Predicate::Lines(
                Lines {
                    pattern: compiled(pattern, args)?,
                    mode,
                    context: Context::asked(args)?,
                },
                within,
            )),
            (None, Some(names), Mode::Files) => Ok(Predicate::Names(names)),
            (None, Some(_), Mode::Content | Mode::Count) => Err(refused(
                "a `glob` alone finds files by name, and they have no matched lines to show or \
                 count",
            )
            .with_recovery("add a `pattern` to search their lines, or leave `output_mode` out")),
            (None, None, _) => Err(refused("neither `pattern` nor `glob` was given")
                .with_recovery(
                    "pass `pattern`, a regular expression in ripgrep's syntax, to search the \
                     lines of files; or `glob` alone, such as `*.rs`, to find files by name",
                )),
        }
    }

    /// Whether the file at `rel` is one this call looks at, for a walk
    /// started at `root`.
    pub(super) fn admits(&self, root: &str, rel: &str) -> bool {
        match self {
            Predicate::Lines(_, None) => true,
            Predicate::Lines(_, Some(names)) | Predicate::Names(names) => names.admit(root, rel),
        }
    }

    /// The payload key the answer's entries travel under.
    pub(super) fn key(&self) -> &'static str {
        match self {
            Predicate::Lines(lines, _) => lines.mode.key(),
            Predicate::Names(_) => Mode::Files.key(),
        }
    }
}

impl Mode {
    /// The spelling a call passes, and the one a refusal offers back.
    const SPELLED: [(&'static str, Mode); 3] = [
        ("files_with_matches", Mode::Files),
        ("content", Mode::Content),
        ("count", Mode::Count),
    ];

    /// The payload key the entries travel under; the ledger keeps these
    /// names whatever the call spelled (`crates/runtime/spec/Rendering.lean`
    /// §8-63 draws them).
    fn key(self) -> &'static str {
        match self {
            Mode::Files => "files",
            Mode::Content => "matches",
            Mode::Count => "counts",
        }
    }

    /// The answer `args` asked for; `files_with_matches` when it did not say.
    fn asked(args: &Map<String, Value>) -> Result<Mode, AxError> {
        let Some(asked) = args.get("output_mode") else {
            return Ok(Mode::Files);
        };
        Mode::SPELLED
            .iter()
            .find(|(spelled, _)| asked.as_str() == Some(*spelled))
            .map(|(_, mode)| *mode)
            .ok_or_else(|| {
                refused(format!(
                    "`output_mode` {asked} is not one this tool answers"
                ))
                .with_nearby(
                    Mode::SPELLED
                        .iter()
                        .map(|(spelled, _)| (*spelled).to_owned())
                        .collect(),
                )
                .with_recovery(
                    "pass one of the nearby modes, or leave `output_mode` out for the files \
                         that hold a match",
                )
            })
    }
}

impl Context {
    /// `-B` and `-A` for one side each, over `context` or its alias `-C`
    /// for both; each side defaults to none and is capped at
    /// [`CONTEXT_CAP`].
    fn asked(args: &Map<String, Value>) -> Result<Context, AxError> {
        let both = match side(args, "context")? {
            Some(lines) => lines,
            None => side(args, "-C")?.unwrap_or(0),
        };
        Ok(Context {
            before: side(args, "-B")?.unwrap_or(both),
            after: side(args, "-A")?.unwrap_or(both),
        })
    }
}

impl Names {
    /// The glob in `asked`, compiled with ripgrep's syntax, where `*`
    /// does not cross a `/`.
    fn of(asked: &Value) -> Result<Names, AxError> {
        let glob = asked
            .as_str()
            .filter(|glob| !glob.is_empty())
            .ok_or_else(|| {
                refused("`glob` is not a non-empty string")
                    .with_recovery("pass a glob such as `*.rs` or `src/**/*.ts`, or leave it out")
            })?;
        let matcher = GlobBuilder::new(glob)
            .literal_separator(true)
            .build()
            .map_err(|err| {
                refused(format!("`glob` {glob:?} does not compile: {err}")).with_recovery(
                    "write it in ripgrep's glob syntax: `*`, `?`, `[ab]`, `{a,b}` and `**/`",
                )
            })?
            .compile_matcher();
        Ok(Names {
            matcher,
            by_path: glob.contains('/'),
        })
    }

    /// Whether the file at `rel`, below a walk started at `root`, is
    /// named by this glob.
    fn admit(&self, root: &str, rel: &str) -> bool {
        let name = rel.rsplit('/').next().unwrap_or(rel);
        let below = match root {
            "" => Some(rel),
            root => rel
                .strip_prefix(root)
                .and_then(|rest| rest.strip_prefix('/')),
        };
        match (self.by_path, below) {
            (true, Some(below)) => self.matcher.is_match(below),
            (true, None) | (false, _) => self.matcher.is_match(name),
        }
    }
}

/// The pattern in `asked`, compiled; `-i` in `args` drops the case.
fn compiled(asked: &Value, args: &Map<String, Value>) -> Result<Regex, AxError> {
    let pattern = asked
        .as_str()
        .filter(|pattern| !pattern.is_empty())
        .ok_or_else(|| {
            refused("`pattern` is not a non-empty string").with_recovery(
                "pass the regular expression to look for; an empty one would match every line \
                 in the city",
            )
        })?;
    let caseless = match args.get("-i") {
        None => false,
        Some(asked) => asked.as_bool().ok_or_else(|| {
            refused("`-i` is not true or false")
                .with_recovery("pass `-i: true` to match without regard to case, or leave it out")
        })?,
    };
    RegexBuilder::new(pattern)
        .case_insensitive(caseless)
        .build()
        .map_err(|err| {
            refused(format!("`pattern` {pattern:?} does not compile: {err}")).with_recovery(
                "write it in ripgrep's regex syntax, escaping `( ) [ ] { } . * + ? | ^ $ \\` \
                 with `\\` to match them literally",
            )
        })
}

/// The lines asked for on one side under `key`, capped at
/// [`CONTEXT_CAP`]; `None` when the call did not say.
fn side(args: &Map<String, Value>, key: &str) -> Result<Option<usize>, AxError> {
    let Some(value) = args.get(key) else {
        return Ok(None);
    };
    let asked = value.as_u64().ok_or_else(|| {
        refused(format!("`{key}` is not a whole number of lines")).with_recovery(format!(
            "pass `{key}` as an integer from 0 to 4, or leave it out"
        ))
    })?;
    // Above the cap is served at the cap, and so is a number no `u8`
    // holds.
    Ok(Some(usize::from(
        u8::try_from(asked).map_or(CONTEXT_CAP, |fits| fits.min(CONTEXT_CAP)),
    )))
}

/// Refuses the arguments whose answer would mislead if they were left
/// unread (D121), naming the way this search answers the same question.
fn refuse_misleading(args: &Map<String, Value>) -> Result<(), AxError> {
    let misleading = [
        (
            "multiline",
            args.get("multiline") == Some(&Value::Bool(true)),
            "this search matches one line at a time; write a `pattern` that matches within one \
             line",
        ),
        (
            "type",
            args.contains_key("type"),
            "pass `glob` instead, such as `*.rs` for Rust files",
        ),
        (
            "offset",
            args.get("offset")
                .is_some_and(|offset| offset.as_u64() != Some(0)),
            "this search does not page; narrow `path`, `glob` or `pattern` instead",
        ),
    ];
    match misleading.into_iter().find(|(_, asked, _)| *asked) {
        Some((key, _, recovery)) => {
            Err(refused(format!("`{key}` is not one this search answers")).with_recovery(recovery))
        }
        None => Ok(()),
    }
}

/// An invalid-argument refusal from this tool.
fn refused(subject: impl Into<String>) -> ErrorDraft {
    AxError::failure(AxCode::InvalidArgs, "search", subject)
}
