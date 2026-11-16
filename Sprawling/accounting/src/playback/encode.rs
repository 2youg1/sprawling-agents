// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError, B3Hash};

use super::document::Document;
use super::{BUNDLE_MAX_BYTES, SCHEMA};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bundle {
    bytes: Vec<u8>,
    events: usize,
}

impl Bundle {

    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    #[must_use]
    pub fn digest(&self) -> B3Hash {
        B3Hash::digest(&self.bytes)
    }

    #[must_use]
    pub fn events(&self) -> usize {
        self.events
    }
}

pub(super) fn encode(document: &Document) -> Result<Bundle, AxError> {
    let mut escaping = Escaping::within(BUNDLE_MAX_BYTES);
    match serde_json::to_writer(&mut escaping, document) {
        Ok(()) => {}

        Err(over) if over.is_io() => {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "write a playback bundle",
                format!("over {BUNDLE_MAX_BYTES} bytes"),
            )
            .with_recovery("narrow the selection with --from and --through, --run or --building"));
        }
        Err(err) => {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "write a playback bundle",
                err.to_string(),
            )
            .with_recovery(
                "export again; a bundle this build cannot serialise is a defect to report",
            ));
        }
    }
    Ok(Bundle {
        events: document.events.len(),
        bytes: escaping.finish(),
    })
}

pub(super) fn decode(bytes: &[u8]) -> Result<(Document, Bundle), AxError> {
    let refuse = |subject: String| {
        AxError::failure(AxCode::InvalidArgs, "read a playback bundle", subject).with_recovery(
            "check the file sprawling playback export wrote, byte for byte; a bundle that was \
             re-indented or edited by hand is not one",
        )
    };
    if bytes.len() > BUNDLE_MAX_BYTES {
        return Err(refuse(format!(
            "{} bytes, over {BUNDLE_MAX_BYTES}",
            bytes.len()
        )));
    }
    let named: Schema = serde_json::from_slice(bytes).map_err(|err| refuse(err.to_string()))?;
    if named.schema != SCHEMA {
        return Err(AxError::failure(
            AxCode::InvalidArgs,
            "read a playback bundle",
            format!(
                "schema '{}', where this build reads '{SCHEMA}'",
                named.schema
            ),
        )
        .with_recovery(
            "check it with the build that exported it, or export it again with this one",
        ));
    }
    let document: Document =
        serde_json::from_slice(bytes).map_err(|err| refuse(err.to_string()))?;
    let bundle = encode(&document)?;
    if bundle.bytes != bytes {
        return Err(refuse(
            "the bytes are not the canonical encoding of what they hold".to_owned(),
        ));
    }
    Ok((document, bundle))
}

#[derive(serde::Deserialize)]
struct Schema {
    schema: String,
}

struct Escaping {
    out: Vec<u8>,
    ceiling: usize,
    held: Held,
}

#[derive(Clone, Copy)]
enum Held {
    Nothing,
    E2,
    E280,
}

impl Escaping {
    fn within(ceiling: usize) -> Escaping {
        Escaping {
            out: Vec::new(),
            ceiling,
            held: Held::Nothing,
        }
    }

    fn finish(mut self) -> Vec<u8> {
        let held = self.release();
        self.out.extend_from_slice(held);
        self.out
    }

    fn take(&mut self, byte: u8) -> std::io::Result<()> {
        match (self.held, byte) {
            (Held::Nothing, b'<') => self.emit(b"\\u003c"),
            (Held::Nothing, b'>') => self.emit(b"\\u003e"),
            (Held::Nothing, b'&') => self.emit(b"\\u0026"),
            (Held::Nothing, 0xE2) => {
                self.held = Held::E2;
                Ok(())
            }
            (Held::Nothing, other) => self.emit(&[other]),
            (Held::E2, 0x80) => {
                self.held = Held::E280;
                Ok(())
            }
            (Held::E280, 0xA8) => {
                self.held = Held::Nothing;
                self.emit(b"\\u2028")
            }
            (Held::E280, 0xA9) => {
                self.held = Held::Nothing;
                self.emit(b"\\u2029")
            }

            (Held::E2 | Held::E280, other) => {
                let held = self.release();
                self.emit(held)?;
                self.take(other)
            }
        }
    }

    fn release(&mut self) -> &'static [u8] {
        let held = match self.held {
            Held::Nothing => &[][..],
            Held::E2 => &[0xE2][..],
            Held::E280 => &[0xE2, 0x80][..],
        };
        self.held = Held::Nothing;
        held
    }

    fn emit(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        match self.out.len().checked_add(bytes.len()) {
            Some(length) if length <= self.ceiling => {
                self.out.extend_from_slice(bytes);
                Ok(())
            }
            Some(_) | None => Err(std::io::Error::other("over the bundle's ceiling")),
        }
    }
}

impl std::io::Write for Escaping {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        for byte in bytes {
            self.take(*byte)?;
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use std::io::Write as _;

    use proptest::prelude::*;

    use super::Escaping;

    fn escaped(text: &str) -> String {
        text.chars()
            .map(|character| match character {
                '<' => "\\u003c".to_owned(),
                '>' => "\\u003e".to_owned(),
                '&' => "\\u0026".to_owned(),
                '\u{2028}' => "\\u2028".to_owned(),
                '\u{2029}' => "\\u2029".to_owned(),
                other => other.to_string(),
            })
            .collect()
    }

    proptest! {

        #[test]
        fn escaping_does_not_depend_on_where_the_writes_are_cut(
            text in "[a-z<>&\u{2028}\u{2029}\u{2027}\u{e280}é]{0,40}",
            cuts in proptest::collection::vec(0usize..48, 0..6),
        ) {
            let bytes = text.as_bytes();
            let mut points: Vec<usize> = cuts.into_iter().map(|cut| cut.min(bytes.len())).collect();
            points.push(bytes.len());
            points.sort_unstable();
            let mut escaping = Escaping::within(usize::MAX);
            let mut from = 0;
            for to in points {
                escaping.write_all(bytes.get(from..to).unwrap()).unwrap();
                from = to;
            }
            prop_assert_eq!(String::from_utf8(escaping.finish()).unwrap(), escaped(&text));
        }
    }

    #[test]
    fn the_writer_stops_at_its_ceiling() {
        let mut escaping = Escaping::within(10);
        let refused = escaping.write_all(b"<<<<");
        assert!(refused.is_err());
        assert!(escaping.finish().len() <= 10);
    }
}
