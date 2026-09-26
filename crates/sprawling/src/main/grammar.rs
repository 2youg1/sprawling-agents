// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A command line becomes an `Invocation`, or a `LineError` naming the
//! word that could not be read (sprawling-SPEC.md 8-89). Pure: it reads
//! the command table and the words, and touches nothing else.
//!
//! `--help`/`-h` and `--version`/`-V` win wherever they stand, so no
//! verb can act on a request for its help.

use super::verbs::{Need, Row, Takes, VERBS, Verb};

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
    if words.iter().any(|word| word == "--version" || word == "-V") {
        return Ok(Invocation::Version);
    }
    let asks_help = words.iter().any(|word| word == "--help" || word == "-h");
    let Some((first, rest)) = words.split_first() else {
        return Ok(Invocation::FirstScreen);
    };
    let (named, rest) = match first.as_str() {
        "help" => match rest.first() {
            Some(verb) => return Ok(Invocation::Help(find(verb)?.verb)),
            None => return Ok(Invocation::Overview),
        },
        "--help" | "-h" => return Ok(Invocation::Overview),
        verb => (find(verb)?, rest),
    };
    if asks_help {
        return Ok(Invocation::Help(named.verb));
    }
    arguments(named, rest).map(|arguments| Invocation::Run(named.verb, arguments))
}

fn find(word: &str) -> Result<&'static Row, LineError> {
    VERBS
        .iter()
        .find(|row| row.name == word || row.aliases.contains(&word))
        .ok_or_else(|| LineError::UnknownVerb {
            given: word.to_owned(),
            nearest: nearest(word, VERBS.iter().map(|row| row.name)),
        })
}

fn arguments(row: &Row, words: &[String]) -> Result<Arguments, LineError> {
    let mut read = Arguments::default();
    let mut words = words.iter();
    while let Some(word) = words.next() {
        if !word.starts_with("--") || word == "-" {
            read.positionals.push(word.clone());
            continue;
        }
        let flag = row
            .flags
            .iter()
            .find(|flag| flag.name == word)
            .ok_or_else(|| LineError::UnknownFlag {
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
