// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A command line becomes an `Invocation`, or a `LineError` naming the
//! word that could not be read (sprawling-SPEC.md 8-89). Pure: it reads
//! the command table and the words, and touches nothing else.
//!
//! `--help`/`-h` and `--version`/`-V` win wherever they stand among
//! sprawling's own words, so no verb can act on a request for its help.
//! Those words end at the first `--`: what follows belongs to the
//! command a verb such as `gauge` runs (sprawling-SPEC.md 8-129-4).

use super::verbs::{AfterDashes, Need, Row, Takes, VERBS, Verb};

/// What a command line asks for.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Invocation {
    /// No words at all: the first screen.
    FirstScreen,
    /// `help`, or `--help` with no verb.
    Overview,
    /// `<verb> --help` or `help <verb>`.
    Help(Verb),
    /// `--version` anywhere; `version` is an alias of `status`.
    Version,
    Run(Verb, Arguments),
}

/// The words a verb was given, sorted by the table.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Arguments {
    positionals: Vec<String>,
    flags: Vec<(&'static str, Option<String>)>,
    after_dashes: Option<Vec<String>>,
}

impl Arguments {
    /// The nth positional argument, counted from one.
    pub(super) fn positional(&self, nth: usize) -> Option<&String> {
        self.positionals.get(nth.checked_sub(1)?)
    }

    /// Whether the flag was given.
    pub(super) fn has(&self, flag: &str) -> bool {
        self.flags.iter().any(|(name, _)| *name == flag)
    }

    /// The value a value-taking flag was given; the last one wins.
    pub(super) fn value(&self, flag: &str) -> Option<&str> {
        self.flags
            .iter()
            .rev()
            .find(|(name, _)| *name == flag)
            .and_then(|(_, value)| value.as_deref())
    }

    /// The words after the first `--`, exactly as given: `None` when the
    /// line has no `--`, empty when nothing follows it.
    pub(super) fn after_dashes(&self) -> Option<&[String]> {
        self.after_dashes.as_deref()
    }
}

/// A command line this binary cannot read. Every variant exits 2.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum LineError {
    UnknownVerb {
        given: String,
        nearest: Vec<&'static str>,
    },
    UnknownFlag {
        verb: &'static str,
        given: String,
        nearest: Vec<&'static str>,
    },
    MissingValue {
        verb: &'static str,
        flag: &'static str,
    },
    MissingPositional {
        verb: &'static str,
        name: &'static str,
    },
    ExtraPositional {
        verb: &'static str,
        given: String,
    },
}

impl std::fmt::Display for LineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownVerb { given, nearest } => {
                write!(f, "unknown command '{given}'.{}", guess(nearest))
            }
            Self::UnknownFlag {
                verb,
                given,
                nearest,
            } => write!(f, "{verb}: unknown flag '{given}'.{}", guess(nearest)),
            Self::MissingValue { verb, flag } => write!(f, "{verb}: {flag} needs a value"),
            Self::MissingPositional { verb, name } => write!(f, "{verb}: <{name}> is missing"),
            Self::ExtraPositional { verb, given } => {
                write!(f, "{verb}: '{given}' is one argument too many")
            }
        }
    }
}

fn guess(nearest: &[&str]) -> String {
    match nearest {
        [] => String::new(),
        names => format!(" Did you mean '{}'?", names.join("', '")),
    }
}

/// Reads the words after the binary's name.
///
/// # Errors
/// `LineError` names the word that could not be read and what was near it.
pub(super) fn parse(words: &[String]) -> Result<Invocation, LineError> {
    let own = words.split(|word| word == "--").next().unwrap_or(&[]);
    if own.iter().any(|word| word == "--version" || word == "-V") {
        return Ok(Invocation::Version);
    }
    let asks_help = own.iter().any(|word| word == "--help" || word == "-h");
    let Some((first, rest)) = words.split_first() else {
        return Ok(Invocation::FirstScreen);
    };
    let (named, rest) = match first.as_str() {
        "help" => match rest.split_first() {
            Some((verb, after)) => return Ok(Invocation::Help(find(verb, after)?.0.verb)),
            None => return Ok(Invocation::Overview),
        },
        "--help" | "-h" => return Ok(Invocation::Overview),
        verb => find(verb, rest)?,
    };
    if asks_help {
        return Ok(Invocation::Help(named.verb));
    }
    arguments(named, rest).map(|arguments| Invocation::Run(named.verb, arguments))
}

/// The row `first` names, or `first` and the word after it name, and the
/// words left after the name. A row whose name is two words
/// (`playback export`) is named by two; every other row by one
/// (sprawling-SPEC.md 8-126).
fn find<'words>(
    first: &str,
    rest: &'words [String],
) -> Result<(&'static Row, &'words [String]), LineError> {
    if let Some(row) = VERBS
        .iter()
        .find(|row| row.name == first || row.aliases.contains(&first))
    {
        return Ok((row, rest));
    }
    let under: Vec<&'static Row> = VERBS
        .iter()
        .filter(|row| {
            row.name
                .split_once(' ')
                .is_some_and(|(head, _)| head == first)
        })
        .collect();
    if under.is_empty() {
        return Err(LineError::UnknownVerb {
            given: first.to_owned(),
            nearest: nearest(first, VERBS.iter().map(|row| row.name)),
        });
    }
    let second = rest.split_first();
    second
        .and_then(|(second, after)| {
            under
                .iter()
                .find(|row| {
                    row.name
                        .split_once(' ')
                        .is_some_and(|(_, tail)| tail == second)
                })
                .map(|row| (*row, after))
        })
        .ok_or_else(|| LineError::UnknownVerb {
            given: second.map_or_else(
                || first.to_owned(),
                |(second, _)| format!("{first} {second}"),
            ),
            nearest: under.iter().map(|row| row.name).collect(),
        })
}

fn arguments(row: &Row, words: &[String]) -> Result<Arguments, LineError> {
    let mut read = Arguments::default();
    let mut words = words.iter();
    while let Some(word) = words.next() {
        if word == "--" && row.after_dashes == AfterDashes::Command {
            read.after_dashes = Some(words.cloned().collect());
            break;
        }
        let named = row
            .flags
            .iter()
            .find(|flag| flag.name == word || flag.short == Some(word.as_str()));
        if named.is_none() && (!word.starts_with("--") || word == "-") {
            read.positionals.push(word.clone());
            continue;
        }
        let flag = named.ok_or_else(|| LineError::UnknownFlag {
            verb: row.name,
            given: word.clone(),
            nearest: nearest(word, row.flags.iter().map(|flag| flag.name)),
        })?;
        let value = match flag.takes {
            Takes::Nothing => None,
            Takes::Value(_) => Some(words.next().cloned().ok_or(LineError::MissingValue {
                verb: row.name,
                flag: flag.name,
            })?),
        };
        read.flags.push((flag.name, value));
    }
    if let Some(extra) = read.positionals.get(row.positionals.len()) {
        return Err(LineError::ExtraPositional {
            verb: row.name,
            given: extra.clone(),
        });
    }
    match row
        .positionals
        .iter()
        .skip(read.positionals.len())
        .find(|(_, need)| *need == Need::Required)
    {
        Some((name, _)) => Err(LineError::MissingPositional {
            verb: row.name,
            name,
        }),
        None => Ok(read),
    }
}

/// The known names sharing the longest prefix (up to four characters)
/// with what was typed: a short list a person can read, not one winner
/// they then have to doubt.
pub(super) fn nearest<'name>(
    typed: &str,
    known: impl Iterator<Item = &'name str> + Clone,
) -> Vec<&'name str> {
    let typed = typed.trim_start_matches('-');
    (1..=typed.chars().count().min(4))
        .rev()
        .map(|length| typed.chars().take(length).collect::<String>())
        .map(|head| {
            known
                .clone()
                .filter(|name| name.trim_start_matches('-').starts_with(&head))
                .collect::<Vec<_>>()
        })
        .find(|close| !close.is_empty())
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "grammar_tests.rs"]
mod tests;
