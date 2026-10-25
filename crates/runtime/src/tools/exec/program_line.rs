// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A command line read without a shell: one program and its arguments
//! (`crates/runtime/spec/Tools/Exec.lean` §8-13-4, D118).
//!
//! A building that offers no shell still runs `command`, as the program
//! its first word names. The grammar is the part of a shell's that
//! changes nothing about what a line reaches: words split on blanks,
//! and quotes that hold blanks inside one word. Every character a shell
//! would act on is refused outside quotes, because passed on as an
//! argument it runs a different command from the one the model wrote,
//! and the model would not know.

use kernel::{AxCode, AxError};

/// The characters a shell gives a meaning to. Outside quotes each one is
/// refused rather than passed on as text.
const SHELL_SYNTAX: [char; 18] = [
    '|', '&', ';', '<', '>', '(', ')', '$', '`', '*', '?', '[', ']', '{', '}', '~', '#', '\n',
];

/// The program a line names and the arguments it hands it.
///
/// Blanks outside quotes end a word. A pair of single or double quotes
/// adds what it holds, blanks included, to the word it stands in, and
/// nothing inside it is expanded; the quotes themselves are dropped. A
/// backslash is an ordinary character, since on Windows it separates a
/// path.
///
/// # Errors
/// `E_INVALID_ARGS` for shell syntax outside quotes, naming the
/// character, for a quote left open, and for a line with no word.
pub(super) fn program_line(text: &str) -> Result<(String, Vec<String>), AxError> {
    let mut words: Vec<String> = Vec::new();
    let mut word: Option<String> = None;
    let mut quote: Option<char> = None;
    for c in text.chars() {
        match quote {
            Some(open) if c == open => quote = None,
            Some(_) => word.get_or_insert_with(String::new).push(c),
            None if c == '"' || c == '\'' => {
                quote = Some(c);
                word.get_or_insert_with(String::new);
            }
            None if SHELL_SYNTAX.contains(&c) => return Err(shell_syntax(c)),
            None if c.is_whitespace() => words.extend(word.take()),
            None => word.get_or_insert_with(String::new).push(c),
        }
    }
    if let Some(open) = quote {
        return Err(refused(
            format!("the quote {open} is never closed"),
            "close the quote, or leave it out",
        ));
    }
    words.extend(word);
    let mut words = words.into_iter();
    let program = words.next().ok_or_else(|| {
        refused(
            "`command` names no program".to_owned(),
            "name the program to run",
        )
    })?;
    Ok((program, words.collect()))
}

fn shell_syntax(c: char) -> AxError {
    let shown = match c {
        '\n' => "a line break".to_owned(),
        other => format!("`{other}`"),
    };
    refused(
        format!("{shown} outside quotes"),
        "this building has no shell, so `command` runs one program with its arguments: quote \
         the character if it belongs to an argument, write pipes, redirects and wildcards as \
         `python`, or ask the User to set `[sandbox] shell = true` in this building's \
         CONFIG.toml",
    )
}

fn refused(subject: String, recovery: &str) -> AxError {
    AxError::failure(AxCode::InvalidArgs, "run command", subject).with_recovery(recovery)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]
mod tests {
    use super::*;

    fn read(text: &str) -> Result<(String, Vec<String>), String> {
        program_line(text).map_err(|err| err.subject().to_owned())
    }

    fn owned(program: &str, args: &[&str]) -> Result<(String, Vec<String>), String> {
        Ok((
            program.to_owned(),
            args.iter().map(|arg| (*arg).to_owned()).collect(),
        ))
    }

    #[test]
    fn words_split_on_blanks_and_quotes_hold_blanks() {
        assert_eq!(
            [
                read("cargo test  -p city"),
                read(r#"git commit -m "two words" '' x"y z"w"#),
                read(r"C:\tools\rg.exe --files src\lib"),
                read(r#"python3 -c "print('a|b')""#),
            ],
            [
                owned("cargo", &["test", "-p", "city"]),
                owned("git", &["commit", "-m", "two words", "", "xy zw"]),
                owned(r"C:\tools\rg.exe", &["--files", r"src\lib"]),
                owned("python3", &["-c", "print('a|b')"]),
            ]
        );
    }

    #[test]
    fn shell_syntax_open_quotes_and_empty_lines_are_refused() {
        let refusals = [
            "ls | wc",
            "cargo test > out.txt",
            "rm *.tmp",
            "echo $HOME",
            "a && b",
            "first\nsecond",
            "echo \"open",
            "   ",
        ]
        .map(|line| read(line).unwrap_err());
        assert_eq!(
            refusals,
            [
                "`|` outside quotes",
                "`>` outside quotes",
                "`*` outside quotes",
                "`$` outside quotes",
                "`&` outside quotes",
                "a line break outside quotes",
                "the quote \" is never closed",
                "`command` names no program",
            ]
            .map(str::to_owned)
        );
    }
}
