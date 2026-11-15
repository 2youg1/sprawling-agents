// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

mod license;
mod tokens;

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
    Markup,
}

pub(crate) fn measure(text: &str, language: Language) -> Reading {
    let body = license::without_license(text);
    let lines = body.lines().count();
    let start = match language {
        Language::Svelte => Mode::Markup,
        Language::Rust | Language::TypeScript => Mode::Code {
            template_expression: false,
            braces: 0,
        },
    };
    let mut scanner = Scanner {
        bytes: body.as_bytes(),
        language,
        comment: vec![false; lines],
        code: vec![false; lines],
        line: 0,
        modes: vec![start],
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
                Some(Mode::Markup) => self.markup(index),
                Some(Mode::Template) => self.template(index),
                Some(Mode::Code {
                    template_expression,
                    braces,
                }) => self.code(index, template_expression, braces),
                None => self.bytes.len(),
            };
        }
    }

    fn markup(&mut self, index: usize) -> usize {
        let Some(&byte) = self.bytes.get(index) else {
            return self.bytes.len();
        };
        if byte == b'\n' {
            return self.advance(index);
        }
        if at(self.bytes, index, b"<!--") {
            return self.html_comment(index);
        }
        if byte == b'{' {
            self.mark_code();
            self.modes.push(Mode::Code {
                template_expression: true,
                braces: 0,
            });
            return self.advance(index);
        }
        if byte == b'<'
            && let Some(end) = self.script_or_style_open(index)
        {
            self.mark_code_range(index, end);
            self.modes.push(Mode::Code {
                template_expression: false,
                braces: 0,
            });
            return end;
        }
        if !byte.is_ascii_whitespace() {
            self.mark_code();
        }
        self.advance(index)
    }

    fn script_or_style_open(&self, index: usize) -> Option<usize> {
        for name in [b"script".as_slice(), b"style".as_slice()] {
            let start = index.saturating_add(1);
            if !tag_name_at(self.bytes, start, name) {
                continue;
            }
            let mut cursor = start.saturating_add(name.len());
            while cursor < self.bytes.len() {
                match self.bytes.get(cursor) {
                    Some(b'>') => return Some(cursor.saturating_add(1)),
                    Some(b'\n') | None => return None,
                    _ => cursor = cursor.saturating_add(1),
                }
            }
        }
        None
    }

    fn script_or_style_close(&self, index: usize) -> Option<usize> {
        if self.bytes.get(index) != Some(&b'<')
            || self.bytes.get(index.saturating_add(1)) != Some(&b'/')
        {
            return None;
        }
        for name in [b"script".as_slice(), b"style".as_slice()] {
            let start = index.saturating_add(2);
            if !self.bytes.get(start..).is_some_and(|rest| {
                rest.len() >= name.len()
                    && rest
                        .get(..name.len())
                        .is_some_and(|found| found.eq_ignore_ascii_case(name))
                    && rest.get(name.len()) == Some(&b'>')
            }) {
                continue;
            }
            return Some(start.saturating_add(name.len()).saturating_add(1));
        }
        None
    }

    fn mark_code_range(&mut self, mut index: usize, end: usize) {
        while index < end && index < self.bytes.len() {
            if self
                .bytes
                .get(index)
                .is_some_and(|byte| !byte.is_ascii_whitespace())
            {
                self.mark_code();
            }
            index = self.advance(index);
        }
    }

    fn code(&mut self, index: usize, template_expression: bool, braces: usize) -> usize {
        let Some(&byte) = self.bytes.get(index) else {
            return self.bytes.len();
        };
        if byte == b'\n' {
            return self.advance(index);
        }
        if self.language == Language::Svelte
            && !template_expression
            && let Some(end) = self.script_or_style_close(index)
        {
            self.mark_code_range(index, end);
            self.modes.pop();
            return end;
        }
        if at(self.bytes, index, b"//") {
            if index > 0 && self.bytes.get(index.saturating_sub(1)) == Some(&b':') {
                self.mark_code();
                return self.advance(index);
            }
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

fn tag_name_at(bytes: &[u8], index: usize, name: &[u8]) -> bool {
    bytes.get(index..).is_some_and(|rest| {
        rest.len() >= name.len()
            && rest
                .get(..name.len())
                .is_some_and(|found| found.eq_ignore_ascii_case(name))
            && rest
                .get(name.len())
                .is_some_and(|next| next.is_ascii_whitespace() || *next == b'>')
    })
}
