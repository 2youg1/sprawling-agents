// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use globset::{GlobBuilder, GlobMatcher};
use kernel::{AxCode, AxError, ErrorDraft, ToolCall};
use regex::{Regex, RegexBuilder};
use regex_syntax::ParserBuilder;
use regex_syntax::hir::{Hir, HirKind, Literal};
use serde_json::{Map, Value};

const CONTEXT_CAP: u8 = 4;

const ONE_LINE: &str =
    "this search matches one line at a time; write a `pattern` that matches within one line";

pub(super) enum Predicate {

    Lines(Lines, Option<Names>),

    Names(Names),
}

pub(super) struct Lines {
    pub(super) pattern: Regex,
    pub(super) mode: Mode,
    pub(super) context: Context,
}

#[derive(Clone, Copy)]
pub(super) struct Context {
    pub(super) before: usize,
    pub(super) after: usize,
}

#[derive(Clone, Copy)]
pub(super) enum Mode {

    Files,

    Content,

    Count,
}

pub(super) struct Names {
    matcher: GlobMatcher,

    by_path: bool,
}

impl Predicate {

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
            (None, None, Mode::Files | Mode::Content | Mode::Count) => Err(refused(
                "neither `pattern` nor `glob` was given",
            )
            .with_recovery(
                "pass `pattern`, a regular expression in ripgrep's syntax, to search the \
                     lines of files; or `glob` alone, such as `*.rs`, to find files by name",
            )),
        }
    }

    pub(super) fn admits(&self, root: &str, rel: &str) -> bool {
        match self {
            Predicate::Lines(_, None) => true,
            Predicate::Lines(_, Some(names)) | Predicate::Names(names) => names.admit(root, rel),
        }
    }

    pub(super) fn key(&self) -> &'static str {
        match self {
            Predicate::Lines(lines, _) => lines.mode.key(),
            Predicate::Names(_) => Mode::Files.key(),
        }
    }
}

impl Mode {

    const SPELLED: [(&'static str, Mode); 3] = [
        ("files_with_matches", Mode::Files),
        ("content", Mode::Content),
        ("count", Mode::Count),
    ];

    fn key(self) -> &'static str {
        match self {
            Mode::Files => "files",
            Mode::Content => "matches",
            Mode::Count => "counts",
        }
    }

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
            (true, None) | (false, Some(_) | None) => self.matcher.is_match(name),
        }
    }
}

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
    let does_not_compile = |err: &dyn std::fmt::Display| {
        refused(format!("`pattern` {pattern:?} does not compile: {err}")).with_recovery(
            "write it in ripgrep's regex syntax, escaping `( ) [ ] { } . * + ? | ^ $ \\` with \
             `\\` to match them literally",
        )
    };
    let regex = RegexBuilder::new(pattern)
        .case_insensitive(caseless)
        .build()
        .map_err(|err| does_not_compile(&err))?;
    let parsed = ParserBuilder::new()
        .case_insensitive(caseless)
        .build()
        .parse(pattern)
        .map_err(|err| does_not_compile(&err))?;
    if spells_line_break(&parsed) {
        return Err(
            refused(format!("`pattern` {pattern:?} spells a line break")).with_recovery(ONE_LINE),
        );
    }
    Ok(regex)
}

fn spells_line_break(hir: &Hir) -> bool {
    match hir.kind() {
        HirKind::Empty | HirKind::Look(_) => false,
        HirKind::Literal(Literal(bytes)) => bytes.contains(&b'\n'),
        HirKind::Class(class) => class.literal().as_deref() == Some(b"\n".as_slice()),
        HirKind::Repetition(repetition) => spells_line_break(&repetition.sub),
        HirKind::Capture(capture) => spells_line_break(&capture.sub),
        HirKind::Concat(parts) | HirKind::Alternation(parts) => parts.iter().any(spells_line_break),
    }
}

fn side(args: &Map<String, Value>, key: &str) -> Result<Option<usize>, AxError> {
    let Some(value) = args.get(key) else {
        return Ok(None);
    };
    let asked = value.as_u64().ok_or_else(|| {
        refused(format!("`{key}` is not a whole number of lines")).with_recovery(format!(
            "pass `{key}` as an integer from 0 to 4, or leave it out"
        ))
    })?;

    Ok(Some(usize::from(
        u8::try_from(asked).map_or(CONTEXT_CAP, |fits| fits.min(CONTEXT_CAP)),
    )))
}

fn refuse_misleading(args: &Map<String, Value>) -> Result<(), AxError> {
    let misleading = [
        (
            "multiline",
            args.get("multiline") == Some(&Value::Bool(true)),
            ONE_LINE,
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

fn refused(subject: impl Into<String>) -> ErrorDraft {
    AxError::failure(AxCode::InvalidArgs, "search", subject)
}
