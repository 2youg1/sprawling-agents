// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::header::{self, Leader};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Language {
    Rust,
    TypeScript,
    Svelte,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Reading {
    pub(crate) comment_lines: usize,
    pub(crate) judged_lines: usize,
}

#[derive(Clone, Copy)]
enum Mode {
    Code {
        template_expression: bool,
        braces: usize,
    },
    Template,
}

pub(crate) fn measure(text: &str, language: Language) -> Reading {
    let body = without_license(text);
    let lines = body.lines().count();
    let mut scanner = Scanner {
        bytes: body.as_bytes(),
        language,
        comment: vec![false; lines],
        code: vec![false; lines],
        line: 0,
        modes: vec![Mode::Code {
            template_expression: false,
            braces: 0,
        }],
    };
    scanner.run();
    let comment_lines = scanner.comment.iter().filter(|line| **line).count();
    let judged_lines = scanner
        .comment
        .iter()
        .zip(&scanner.code)
        .filter(|(comment, code)| **comment || **code)
        .count();
    Reading {
        comment_lines,
        judged_lines,
    }
}

fn without_license(text: &str) -> &str {
    let mut rows = text.lines();
    let expected = header::notice(Leader::Rust);
    let licensed = expected
        .iter()
        .all(|want| rows.next() == Some(want.as_str()));
    if !licensed {
        return text;
    }
    let offset = text
        .split_inclusive('\n')
        .take(expected.len())
        .fold(0_usize, |total, row| total.saturating_add(row.len()));
    text.get(offset..).unwrap_or(text)
}

struct Scanner<'a> {
    bytes: &'a [u8],
    language: Language,
    comment: Vec<bool>,
    code: Vec<bool>,
    line: usize,
    modes: Vec<Mode>,
}

impl Scanner<'_> {
    fn run(&mut self) {
        let mut index = 0;
        while index < self.bytes.len() {
            index = match self.modes.last().copied() {
                Some(Mode::Template) => self.template(index),
                Some(Mode::Code {
                    template_expression,
                    braces,
                }) => self.code(index, template_expression, braces),
                None => self.bytes.len(),
            };
        }
    }

    fn code(&mut self, index: usize, template_expression: bool, braces: usize) -> usize {
        let Some(&byte) = self.bytes.get(index) else {
            return self.bytes.len();
        };
        if byte == b'\n' {
            return self.advance(index);
        }
        if at(self.bytes, index, b"//") {
            return self.line_comment(index);
        }
        if at(self.bytes, index, b"/*") {
            return self.block_comment(index);
        }
        if self.language == Language::Svelte && at(self.bytes, index, b"<!--") {
            return self.html_comment(index);
        }
        if byte.is_ascii_whitespace() {
            return self.advance(index);
        }
        if self.language != Language::Rust && byte == b'`' {
            self.mark_code();
            self.modes.push(Mode::Template);
            return self.advance(index);
        }
        if byte == b'"' || (byte == b'\'' && self.single_quote_starts_literal(index)) {
            return self.quoted(index, byte);
        }
        if self.language == Language::Rust
            && let Some((prefix, hashes)) = self.raw_string_start(index)
        {
            return self.raw_string(index, prefix, hashes);
        }
        if self.language != Language::Rust && byte == b'/' && self.regex_allowed(index) {
            return self.regex(index);
        }
        self.mark_code();
        if template_expression && byte == b'{' {
            if let Some(Mode::Code { braces, .. }) = self.modes.last_mut() {
                *braces = braces.saturating_add(1);
            }
        } else if template_expression && byte == b'}' {
            if braces == 0 {
                self.modes.pop();
            } else if let Some(Mode::Code { braces, .. }) = self.modes.last_mut() {
                *braces = braces.saturating_sub(1);
            }
        }
        self.advance(index)
    }

    fn template(&mut self, index: usize) -> usize {
        let Some(&byte) = self.bytes.get(index) else {
            return self.bytes.len();
        };
        if byte == b'\n' {
            return self.advance(index);
        }
        if byte == b'\\' {
            self.mark_code();
            let next = self.advance(index);
            if next < self.bytes.len() {
                self.mark_code();
            }
            return self.advance(next);
        }
        if byte == b'`' {
            self.mark_code();
            self.modes.pop();
            return self.advance(index);
        }
        if at(self.bytes, index, b"${") {
            self.mark_code();
            self.modes.push(Mode::Code {
                template_expression: true,
                braces: 0,
            });
            let next = self.advance(index);
            return self.advance(next);
        }
        if !byte.is_ascii_whitespace() {
            self.mark_code();
        }
        self.advance(index)
    }

    fn single_quote_starts_literal(&self, index: usize) -> bool {
        if self.language != Language::Rust {
            return true;
        }
        let Some(&first) = self.bytes.get(index.saturating_add(1)) else {
            return false;
        };
        if first == b'\\' {
            let mut cursor = index.saturating_add(2);
            while cursor < self.bytes.len() && cursor.saturating_sub(index) <= 16 {
                match self.bytes.get(cursor) {
                    Some(b'\n') | None => return false,
                    Some(b'\'') => return true,
                    _ => cursor = cursor.saturating_add(1),
                }
            }
            return false;
        }
        let width = match first {
            0x00..=0x7f => 1,
            0xc0..=0xdf => 2,
            0xe0..=0xef => 3,
            0xf0..=0xf7 => 4,
            _ => return false,
        };
        self.bytes
            .get(index.saturating_add(1).saturating_add(width))
            == Some(&b'\'')
    }

    fn line_comment(&mut self, mut index: usize) -> usize {
        while index < self.bytes.len() && self.bytes.get(index) != Some(&b'\n') {
            self.mark_comment();
            index = self.advance(index);
        }
        index
    }

    fn block_comment(&mut self, mut index: usize) -> usize {
        let mut depth = 1_usize;
        index = self.comment_bytes(index, 2);
        while index < self.bytes.len() && depth > 0 {
            if self.language == Language::Rust && at(self.bytes, index, b"/*") {
                depth = depth.saturating_add(1);
                index = self.comment_bytes(index, 2);
            } else if at(self.bytes, index, b"*/") {
                depth = depth.saturating_sub(1);
                index = self.comment_bytes(index, 2);
            } else {
                if self
                    .bytes
                    .get(index)
                    .is_some_and(|byte| !byte.is_ascii_whitespace())
                {
                    self.mark_comment();
                }
                index = self.advance(index);
            }
        }
        index
    }

    fn html_comment(&mut self, mut index: usize) -> usize {
        index = self.comment_bytes(index, 4);
        while index < self.bytes.len() {
            if at(self.bytes, index, b"-->") {
                return self.comment_bytes(index, 3);
            }
            if self
                .bytes
                .get(index)
                .is_some_and(|byte| !byte.is_ascii_whitespace())
            {
                self.mark_comment();
            }
            index = self.advance(index);
        }
        index
    }

    fn comment_bytes(&mut self, mut index: usize, count: usize) -> usize {
        for _ in 0..count {
            if index < self.bytes.len() {
                self.mark_comment();
            }
            index = self.advance(index);
        }
        index
    }

    fn quoted(&mut self, mut index: usize, quote: u8) -> usize {
        self.mark_code();
        index = self.advance(index);
        while index < self.bytes.len() {
            let Some(&byte) = self.bytes.get(index) else {
                break;
            };
            if !byte.is_ascii_whitespace() {
                self.mark_code();
            }
            index = self.advance(index);
            if byte == b'\\' && index < self.bytes.len() {
                if self
                    .bytes
                    .get(index)
                    .is_some_and(|next| !next.is_ascii_whitespace())
                {
                    self.mark_code();
                }
                index = self.advance(index);
            } else if byte == quote {
                break;
            }
        }
        index
    }

    fn raw_string_start(&self, index: usize) -> Option<(usize, usize)> {
        let mut cursor = index;
        if matches!(self.bytes.get(cursor), Some(b'b' | b'c'))
            && self.bytes.get(cursor.saturating_add(1)) == Some(&b'r')
        {
            cursor = cursor.saturating_add(1);
        }
        if self.bytes.get(cursor) != Some(&b'r') {
            return None;
        }
        cursor = cursor.saturating_add(1);
        let mut hashes = 0_usize;
        while self.bytes.get(cursor) == Some(&b'#') {
            hashes = hashes.saturating_add(1);
            cursor = cursor.saturating_add(1);
        }
        if self.bytes.get(cursor) != Some(&b'"') {
            return None;
        }
        Some((cursor.saturating_add(1).saturating_sub(index), hashes))
    }

    fn raw_string(&mut self, mut index: usize, prefix: usize, hashes: usize) -> usize {
        for _ in 0..prefix {
            self.mark_code();
            index = self.advance(index);
        }
        while index < self.bytes.len() {
            if self.bytes.get(index) == Some(&b'"') && self.raw_close(index, hashes) {
                for _ in 0..hashes.saturating_add(1) {
                    self.mark_code();
                    index = self.advance(index);
                }
                break;
            }
            if self
                .bytes
                .get(index)
                .is_some_and(|byte| !byte.is_ascii_whitespace())
            {
                self.mark_code();
            }
            index = self.advance(index);
        }
        index
    }

    fn raw_close(&self, index: usize, hashes: usize) -> bool {
        (0..hashes).all(|offset| {
            self.bytes
                .get(index.saturating_add(offset).saturating_add(1))
                == Some(&b'#')
        })
    }

    fn regex_allowed(&self, index: usize) -> bool {
        let mut cursor = index;
        let mut previous = None;
        while cursor > 0 {
            cursor = cursor.saturating_sub(1);
            let Some(&byte) = self.bytes.get(cursor) else {
                continue;
            };
            if !byte.is_ascii_whitespace() {
                previous = Some((cursor, byte));
                break;
            }
        }
        let Some((position, byte)) = previous else {
            return true;
        };
        if b"=([{,!?:;>&|+-*%^~".contains(&byte) {
            return true;
        }
        if byte.is_ascii_alphabetic() {
            let mut start = position;
            while start > 0
                && self
                    .bytes
                    .get(start.saturating_sub(1))
                    .is_some_and(u8::is_ascii_alphabetic)
            {
                start = start.saturating_sub(1);
            }
            const KEYWORDS: [&[u8]; 7] = [
                b"return", b"case", b"typeof", b"delete", b"void", b"yield", b"await",
            ];
            return self
                .bytes
                .get(start..=position)
                .is_some_and(|word| KEYWORDS.contains(&word));
        }
        false
    }

    fn regex(&mut self, mut index: usize) -> usize {
        let mut in_class = false;
        index = self.advance(index);
        while index < self.bytes.len() {
            let Some(&byte) = self.bytes.get(index) else {
                break;
            };
            if byte == b'\n' {
                break;
            }
            self.mark_code();
            index = self.advance(index);
            if byte == b'\\' && index < self.bytes.len() {
                self.mark_code();
                index = self.advance(index);
            } else if byte == b'[' {
                in_class = true;
            } else if byte == b']' {
                in_class = false;
            } else if byte == b'/' && !in_class {
                break;
            }
        }
        index
    }

    fn mark_comment(&mut self) {
        if let Some(line) = self.comment.get_mut(self.line) {
            *line = true;
        }
    }

    fn mark_code(&mut self) {
        if let Some(line) = self.code.get_mut(self.line) {
            *line = true;
        }
    }

    fn advance(&mut self, index: usize) -> usize {
        if self.bytes.get(index) == Some(&b'\n') {
            self.line = self.line.saturating_add(1);
        }
        index.saturating_add(1)
    }
}

fn at(bytes: &[u8], index: usize, wanted: &[u8]) -> bool {
    bytes
        .get(index..index.saturating_add(wanted.len()))
        .is_some_and(|found| found == wanted)
}
