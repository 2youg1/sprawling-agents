// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::{Scanner, Syntax, at, expression};

impl Scanner<'_> {
    pub(super) fn template(&mut self, index: usize) -> usize {
        let Some(&byte) = self.bytes.get(index) else {
            return self.bytes.len();
        };
        if byte == b'\n' {
            return self.advance(index);
        }
        if byte == b'\\' {
            let next = self.advance(index);
            return self.advance(next);
        }
        if byte == b'`' {
            self.modes.pop();
            return self.advance(index);
        }
        if at(self.bytes, index, b"${") {
            self.modes.push(expression());
            let next = self.advance(index);
            return self.advance(next);
        }
        self.advance(index)
    }

    pub(super) fn single_quote_starts_literal(&self, index: usize, syntax: Syntax) -> bool {
        if syntax != Syntax::Rust {
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

    pub(super) fn quoted(&mut self, mut index: usize, quote: u8) -> usize {
        index = self.advance(index);
        while index < self.bytes.len() {
            let Some(&byte) = self.bytes.get(index) else {
                break;
            };
            index = self.advance(index);
            if byte == b'\\' && index < self.bytes.len() {
                index = self.advance(index);
            } else if byte == quote {
                break;
            }
        }
        index
    }

    pub(super) fn python_quoted(&mut self, mut index: usize, quote: u8) -> usize {
        let delimiter = [quote, quote, quote];
        let triple = at(self.bytes, index, &delimiter);
        let width = if triple { 3 } else { 1 };
        for _ in 0..width {
            index = self.advance(index);
        }
        while index < self.bytes.len() {
            if triple && at(self.bytes, index, &delimiter) {
                for _ in 0..3 {
                    index = self.advance(index);
                }
                break;
            }
            let Some(&byte) = self.bytes.get(index) else {
                break;
            };
            index = self.advance(index);
            if byte == b'\\' && index < self.bytes.len() {
                index = self.advance(index);
            } else if !triple && (byte == quote || byte == b'\n') {
                break;
            }
        }
        index
    }

    pub(super) fn raw_string_start(&self, index: usize) -> Option<(usize, usize)> {
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

    pub(super) fn raw_string(&mut self, mut index: usize, prefix: usize, hashes: usize) -> usize {
        for _ in 0..prefix {
            index = self.advance(index);
        }
        while index < self.bytes.len() {
            if self.bytes.get(index) == Some(&b'"') && self.raw_close(index, hashes) {
                for _ in 0..hashes.saturating_add(1) {
                    index = self.advance(index);
                }
                break;
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

    pub(super) fn regex_allowed(&self, index: usize) -> bool {
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
        if byte == b'+' || byte == b'-' {
            return self.bytes.get(position.saturating_sub(1)) != Some(&byte);
        }
        if b"=([{,!?:;>&|*%^~".contains(&byte) {
            return true;
        }
        if byte == b')' {
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

    pub(super) fn regex(&mut self, mut index: usize) -> usize {
        let mut in_class = false;
        index = self.advance(index);
        while index < self.bytes.len() {
            let Some(&byte) = self.bytes.get(index) else {
                break;
            };
            if byte == b'\n' {
                break;
            }
            if !in_class && byte == b'/' {
                if matches!(
                    self.bytes.get(index.saturating_add(1)),
                    Some(b'/') | Some(b'*')
                ) {
                    return index;
                }
                return self.advance(index);
            }
            if byte == b'\\' {
                index = self.advance(index);
                if index < self.bytes.len() {
                    index = self.advance(index);
                }
                continue;
            }
            if byte == b'[' {
                in_class = true;
            } else if byte == b']' {
                in_class = false;
            }
            index = self.advance(index);
        }
        index
    }
}
