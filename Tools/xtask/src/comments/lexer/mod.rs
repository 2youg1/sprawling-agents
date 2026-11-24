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
    delimiters: usize,
}

#[derive(Clone, Copy)]
pub(super) struct PythonString {
    pub(super) quote: u8,
    pub(super) triple: bool,
    pub(super) raw: bool,
}

#[derive(Clone, Copy)]
pub(super) enum Mode {
    Code(CodeMode),
    Template,
    PythonString(PythonString),
    PythonFormat,
}

impl Language {
    fn syntax(self) -> Syntax {
        match self {
            Self::Rust => Syntax::Rust,
            Self::TypeScript | Self::JavaScript => Syntax::JavaScript,
            Self::Zig => Syntax::Zig,
            Self::Python => Syntax::Python,
            Self::Css => Syntax::Css,
        }
    }
}

pub(crate) fn measure(text: &str, language: Language) -> Reading {
    let body = license::without_license(text, language);
    let mut scanner = Scanner {
        bytes: body.as_bytes(),
        comment: vec![false; body.lines().count()],
        line: 0,
        modes: vec![code(language.syntax())],
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
        delimiters: 0,
    })
}

fn expression(syntax: Syntax) -> Mode {
    Mode::Code(CodeMode {
        syntax,
        template_expression: true,
        braces: 0,
        delimiters: 0,
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
                Some(Mode::Template) => self.template(index),
                Some(Mode::Code(mode)) => self.code(index, mode),
                Some(Mode::PythonString(string)) => self.python_string(index, string),
                Some(Mode::PythonFormat) => self.python_format_spec(index),
                None => self.bytes.len(),
            };
        }
    }

    fn code(&mut self, index: usize, mode: CodeMode) -> usize {
        let Some(&byte) = self.bytes.get(index) else {
            return self.bytes.len();
        };
        if byte == b'\n' {
            return self.advance(index);
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
        if mode.syntax == Syntax::Python
            && mode.template_expression
            && mode.delimiters == 0
            && byte == b':'
        {
            self.modes.push(Mode::PythonFormat);
            return self.advance(index);
        }
        if mode.syntax == Syntax::Python && matches!(byte, b'\'' | b'"') {
            if let Some(raw) = self.python_format(index) {
                let triple = at(self.bytes, index, &[byte, byte, byte]);
                self.modes.push(Mode::PythonString(PythonString {
                    quote: byte,
                    triple,
                    raw,
                }));
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
        if mode.syntax == Syntax::Python && mode.template_expression {
            if matches!(byte, b'(' | b'[')
                && let Some(Mode::Code(code)) = self.modes.last_mut()
            {
                code.delimiters = code.delimiters.saturating_add(1);
            } else if matches!(byte, b')' | b']')
                && let Some(Mode::Code(code)) = self.modes.last_mut()
            {
                code.delimiters = code.delimiters.saturating_sub(1);
            }
        }
        if mode.template_expression && byte == b'{' {
            if let Some(Mode::Code(code)) = self.modes.last_mut() {
                code.braces = code.braces.saturating_add(1);
                code.delimiters = code.delimiters.saturating_add(1);
            }
        } else if mode.template_expression && byte == b'}' {
            if mode.braces == 0 {
                self.modes.pop();
            } else if let Some(Mode::Code(code)) = self.modes.last_mut() {
                code.braces = code.braces.saturating_sub(1);
                code.delimiters = code.delimiters.saturating_sub(1);
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
