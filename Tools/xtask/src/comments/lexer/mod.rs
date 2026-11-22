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
    JavaScript,
    Svelte,
    Zig,
    Python,
    Css,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Reading {
    pub(crate) comment_lines: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Syntax {
    Rust,
    JavaScript,
    Zig,
    Python,
    Css,
}

#[derive(Clone, Copy)]
pub(super) struct CodeMode {
    syntax: Syntax,
    template_expression: bool,
    braces: usize,
    embedded: bool,
}

#[derive(Clone, Copy)]
pub(super) enum Mode {
    Code(CodeMode),
    Template,
    Markup,
    PythonString { quote: u8, triple: bool },
}

impl Language {
    fn syntax(self) -> Syntax {
        match self {
            Self::Rust => Syntax::Rust,
            Self::TypeScript | Self::JavaScript | Self::Svelte => Syntax::JavaScript,
            Self::Zig => Syntax::Zig,
            Self::Python => Syntax::Python,
            Self::Css => Syntax::Css,
        }
    }
}

pub(crate) fn measure(text: &str, language: Language) -> Reading {
    let body = license::without_license(text, language);
    let start = if language == Language::Svelte {
        Mode::Markup
    } else {
        code(language.syntax())
    };
    let mut scanner = Scanner {
        bytes: body.as_bytes(),
        comment: vec![false; body.lines().count()],
        line: 0,
        modes: vec![start],
    };
    scanner.run();
    let comment_lines = scanner.comment.iter().filter(|line| **line).count();
    Reading { comment_lines }
}

fn code(syntax: Syntax) -> Mode {
    Mode::Code(CodeMode {
        syntax,
        template_expression: false,
        braces: 0,
        embedded: false,
    })
}

fn expression(syntax: Syntax) -> Mode {
    Mode::Code(CodeMode {
        syntax,
        template_expression: true,
        braces: 0,
        embedded: false,
    })
}

fn embedded(syntax: Syntax) -> Mode {
    Mode::Code(CodeMode {
        syntax,
        template_expression: false,
        braces: 0,
        embedded: true,
    })
}

struct Scanner<'a> {
    bytes: &'a [u8],
    comment: Vec<bool>,
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
                Some(Mode::Code(mode)) => self.code(index, mode),
                Some(Mode::PythonString { quote, triple }) => {
                    self.python_string(index, quote, triple)
                }
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
            if self.bytes.get(index.saturating_add(1)) == Some(&b'/') {
                return self.svelte_close(index);
            }
            self.modes.push(expression(Syntax::JavaScript));
            return self.advance(index);
        }
        if byte == b'<'
            && let Some((syntax, end)) = self.embedded_open(index)
        {
            self.modes.push(embedded(syntax));
            return end;
        }
        self.advance(index)
    }

    fn svelte_close(&mut self, mut index: usize) -> usize {
        while index < self.bytes.len() {
            let Some(&byte) = self.bytes.get(index) else {
                break;
            };
            index = self.advance(index);
            if byte == b'}' {
                break;
            }
        }
        index
    }

    fn embedded_open(&self, index: usize) -> Option<(Syntax, usize)> {
        for (name, syntax) in [
            (b"script".as_slice(), Syntax::JavaScript),
            (b"style".as_slice(), Syntax::Css),
        ] {
            let start = index.saturating_add(1);
            if !tag_name_at(self.bytes, start, name) {
                continue;
            }
            let mut cursor = start.saturating_add(name.len());
            while cursor < self.bytes.len() {
                match self.bytes.get(cursor) {
                    Some(b'>') => return Some((syntax, cursor.saturating_add(1))),
                    None => return None,
                    _ => cursor = cursor.saturating_add(1),
                }
            }
        }
        None
    }

    fn embedded_close(&self, index: usize, syntax: Syntax) -> Option<usize> {
        let name = match syntax {
            Syntax::JavaScript => b"script".as_slice(),
            Syntax::Css => b"style".as_slice(),
            _ => return None,
        };
        if self.bytes.get(index) != Some(&b'<')
            || self.bytes.get(index.saturating_add(1)) != Some(&b'/')
        {
            return None;
        }
        let start = index.saturating_add(2);
        self.bytes.get(start..).and_then(|rest| {
            (rest.len() >= name.len()
                && rest
                    .get(..name.len())
                    .is_some_and(|found| found.eq_ignore_ascii_case(name))
                && rest.get(name.len()) == Some(&b'>'))
            .then_some(start.saturating_add(name.len()).saturating_add(1))
        })
    }

    fn code(&mut self, index: usize, mode: CodeMode) -> usize {
        let Some(&byte) = self.bytes.get(index) else {
            return self.bytes.len();
        };
        if byte == b'\n' {
            return self.advance(index);
        }
        if mode.embedded
            && !mode.template_expression
            && let Some(end) = self.embedded_close(index, mode.syntax)
        {
            self.modes.pop();
            return end;
        }
        if matches!(mode.syntax, Syntax::Rust | Syntax::JavaScript | Syntax::Zig)
            && at(self.bytes, index, b"//")
        {
            let markup_url = mode.syntax == Syntax::JavaScript
                && index > 0
                && self.bytes.get(index.saturating_sub(1)) == Some(&b':')
                && self
                    .bytes
                    .get(index.saturating_add(2))
                    .is_some_and(|byte| !byte.is_ascii_whitespace());
            if markup_url {
                return self.advance(index);
            }
            return self.line_comment(index);
        }
        if mode.syntax == Syntax::Python && byte == b'#' {
            return self.line_comment(index);
        }
        if matches!(mode.syntax, Syntax::Rust | Syntax::JavaScript | Syntax::Css)
            && at(self.bytes, index, b"/*")
        {
            return self.block_comment(index, mode.syntax);
        }
        if mode.syntax == Syntax::Zig && at(self.bytes, index, b"\\\\") {
            return self.line_literal(index);
        }
        if byte.is_ascii_whitespace() {
            return self.advance(index);
        }
        if mode.syntax == Syntax::JavaScript && byte == b'`' {
            self.modes.push(Mode::Template);
            return self.advance(index);
        }
        if mode.syntax == Syntax::Python && matches!(byte, b'\'' | b'"') {
            if self.python_formatted(index) {
                let triple = at(self.bytes, index, &[byte, byte, byte]);
                self.modes.push(Mode::PythonString {
                    quote: byte,
                    triple,
                });
                return self.python_string_open(index, triple);
            }
            return self.python_quoted(index, byte);
        }
        if mode.syntax == Syntax::Rust
            && let Some((prefix, hashes)) = self.raw_string_start(index)
        {
            return self.raw_string(index, prefix, hashes);
        }
        if byte == b'"' || (byte == b'\'' && self.single_quote_starts_literal(index, mode.syntax)) {
            return self.quoted(index, byte);
        }
        if mode.syntax == Syntax::JavaScript && byte == b'/' && self.regex_allowed(index) {
            return self.regex(index);
        }
        if mode.template_expression && byte == b'{' {
            if let Some(Mode::Code(code)) = self.modes.last_mut() {
                code.braces = code.braces.saturating_add(1);
            }
        } else if mode.template_expression && byte == b'}' {
            if mode.braces == 0 {
                self.modes.pop();
            } else if let Some(Mode::Code(code)) = self.modes.last_mut() {
                code.braces = code.braces.saturating_sub(1);
            }
        }
        self.advance(index)
    }

    fn line_literal(&mut self, mut index: usize) -> usize {
        while index < self.bytes.len() && self.bytes.get(index) != Some(&b'\n') {
            index = self.advance(index);
        }
        index
    }

    fn line_comment(&mut self, mut index: usize) -> usize {
        while index < self.bytes.len() && self.bytes.get(index) != Some(&b'\n') {
            self.mark_comment();
            index = self.advance(index);
        }
        index
    }

    fn block_comment(&mut self, mut index: usize, syntax: Syntax) -> usize {
        let mut depth = 1_usize;
        index = self.comment_bytes(index, 2);
        while index < self.bytes.len() && depth > 0 {
            if syntax == Syntax::Rust && at(self.bytes, index, b"/*") {
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
