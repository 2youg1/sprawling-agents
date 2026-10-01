// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One window of a stored Markdown version, laid out as blocks
//! (accounting-SPEC.md 8-23, wire-SPEC.md 8-74).
//!
//! Reads the content store and nothing else, the way a range does: the
//! version is the object's address, so a page previewing a file a
//! resident is rewriting keeps reading the version it opened. Every
//! judgement - which encoding, where the window is cut and where it ends,
//! what the blocks are - is `documents::preview`'s.

use std::path::Path;

use documents::{Encoding, Lifted, Preview, Span};
use kernel::{AxError, B3Hash};
use storage::{Cas, StorageError};

use super::super::holding::Views;
use super::super::prepared::{Prepared, unavailable};

impl Views {
    /// A preview is read from the store after the views are released.
    pub(in crate::views) fn preview_ask(&self, version: B3Hash, viewport: Span) -> Prepared {
        Prepared::Preview {
            city_root: self.city_root.clone(),
            version,
            viewport,
        }
    }
}

/// The preview of the window of `version` that `viewport` asks for, or
/// `Unavailable` when the store does not hold that version or its bytes
/// are not text.
pub(in crate::views) fn preview_answer(
    city_root: &Path,
    version: B3Hash,
    viewport: Span,
) -> wire::Answer {
    match preview_of(city_root, &version, viewport) {
        Ok(preview) => wire::Answer::Preview(Box::new(wire::PreviewAnswer { version, preview })),
        // "I could not look" is the answer for every way this fails, as it
        // is for a range: a version the store never kept, a store that
        // will not open, bytes that are not text. The query handed back
        // names which version.
        Err(_) => unavailable(format!("Preview({version})")),
    }
}

fn preview_of(city_root: &Path, version: &B3Hash, viewport: Span) -> Result<Preview, AxError> {
    let store = Cas::open(&kernel::layout::CityLayout::new(city_root).cas())
        .map_err(StorageError::into_ax)?;
    let size = store.size(version).map_err(StorageError::into_ax)?;
    let mark = read(&store, version, Span::new(0, size.min(3))?)?;
    let lift = documents::lift(viewport, size);
    let bytes = read(&store, version, lift)?;
    documents::preview(
        Encoding::of_mark(&mark),
        Lifted {
            at: lift.start(),
            bytes: &bytes,
            size,
        },
        viewport,
    )
}

/// The bytes of one half-open span of a stored object.
fn read(store: &Cas, version: &B3Hash, span: Span) -> Result<Vec<u8>, AxError> {
    if span.is_empty() {
        return Ok(Vec::new());
    }
    let closed = kernel::Range::bytes(span.start(), span.end().saturating_sub(1))?;
    store
        .get_range(version, &closed)
        .map_err(StorageError::into_ax)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use documents::{Block, Encoding, Inline, Laid, Preview};
    use kernel::Address;
    use storage::Cas;
    use wire::{Coverage, DocumentBody, DocumentState};

    use super::*;
    use crate::views::Views;

    fn preview(views: &mut Views, version: B3Hash, start: u64, end: u64) -> wire::Answer {
        views.answer(&wire::Query::Preview {
            version,
            viewport: Span::new(start, end).unwrap(),
        })
    }

    fn kept(dir: &Path, bytes: &[u8]) -> B3Hash {
        Cas::open(&kernel::layout::CityLayout::new(dir).cas())
            .unwrap()
            .put(bytes)
            .unwrap()
    }

    #[test]
    fn a_version_the_store_never_kept_is_unavailable() {
        let dir = tempfile::tempdir().unwrap();
        let mut views = Views::new(dir.path());
        let never = B3Hash::digest(b"never kept");
        assert_eq!(
            preview(&mut views, never, 0, 10),
            wire::Answer::Unavailable {
                query: format!("Preview({never})"),
            }
        );
    }

    /// A Markdown file three windows long, opened through `Document` and
    /// previewed from 0 on from each answer's end: every window but the
    /// last ends where a paragraph ends, and every paragraph is laid out
    /// exactly once.
    #[test]
    fn a_long_document_previews_window_by_window_to_its_end() {
        let dir = tempfile::tempdir().unwrap();
        let paragraphs: Vec<String> = (0..2_500)
            .map(|index| format!("Paragraph {index} {}", "x".repeat(60)))
            .collect();
        let source: String = paragraphs
            .iter()
            .map(|line| format!("{line}\n\n"))
            .collect();
        std::fs::create_dir_all(dir.path().join("lab")).unwrap();
        std::fs::write(dir.path().join("lab/notes.md"), &source).unwrap();
        let mut views = Views::new(dir.path());
        let at = Address::parse("lab/notes.md").unwrap();
        let wire::Answer::Document(answer) = views.answer(&wire::Query::Document { at }) else {
            panic!("Document answers with a document");
        };
        let DocumentState::Held(held) = answer.state else {
            panic!("{answer:?}");
        };
        let DocumentBody::Text { coverage, .. } = held.body else {
            panic!("the fixture is text");
        };
        assert_eq!(coverage, Coverage::Head, "the version is kept in the store");
        let size = u64::try_from(source.len()).unwrap();
        let mut read = Vec::new();
        let mut from = 0;
        while from < size {
            let wire::Answer::Preview(answer) = preview(&mut views, held.version, from, size)
            else {
                panic!("the version was kept");
            };
            assert_eq!(answer.version, held.version);
            let Preview::Laid(Laid { span, blocks }) = answer.preview else {
                panic!("UTF-8 is laid out");
            };
            assert_eq!(span.start(), from);
            if span.end() < size {
                let last = blocks.last().map(|block| {
                    let Block::Paragraph { span, .. } = block else {
                        panic!("{block:?}");
                    };
                    span.end()
                });
                assert_eq!(
                    last,
                    Some(span.end()),
                    "a window ends where a paragraph ends"
                );
            }
            for block in blocks {
                let Block::Paragraph { inline, .. } = block else {
                    panic!("{block:?}");
                };
                let [Inline::Text(words)] = inline.as_slice() else {
                    panic!("{inline:?}");
                };
                read.push(words.clone());
            }
            from = span.end();
        }
        assert_eq!(read, paragraphs);
    }

    /// A Markdown file one window holds whole, opened through `Document`,
    /// previews by the version that answer named (accounting-SPEC 33).
    #[test]
    fn a_short_document_previews_by_its_version() {
        let dir = tempfile::tempdir().unwrap();
        let source = "# Title\n\nbody\n";
        std::fs::create_dir_all(dir.path().join("lab")).unwrap();
        std::fs::write(dir.path().join("lab/short.md"), source).unwrap();
        let mut views = Views::new(dir.path());
        let at = Address::parse("lab/short.md").unwrap();
        let wire::Answer::Document(answer) = views.answer(&wire::Query::Document { at }) else {
            panic!("Document answers with a document");
        };
        let DocumentState::Held(held) = answer.state else {
            panic!("{answer:?}");
        };
        let DocumentBody::Text { coverage, .. } = held.body else {
            panic!("the fixture is text");
        };
        assert_eq!(coverage, Coverage::Whole, "one window holds it");
        assert_eq!(
            preview(&mut views, held.version, 0, 14),
            wire::Answer::Preview(Box::new(wire::PreviewAnswer {
                version: held.version,
                preview: Preview::Laid(Laid {
                    span: Span::new(0, 14).unwrap(),
                    blocks: vec![
                        Block::Heading {
                            span: Span::new(0, 7).unwrap(),
                            level: 1,
                            inline: vec![Inline::Text("Title".to_owned())],
                        },
                        Block::Paragraph {
                            span: Span::new(9, 13).unwrap(),
                            inline: vec![Inline::Text("body".to_owned())],
                        },
                    ],
                }),
            }))
        );
    }

    #[test]
    fn bytes_that_are_not_text_are_unavailable() {
        let dir = tempfile::tempdir().unwrap();
        let version = kept(dir.path(), b"\x80\x81 is not UTF-8");
        let mut views = Views::new(dir.path());
        assert_eq!(
            preview(&mut views, version, 0, 16),
            wire::Answer::Unavailable {
                query: format!("Preview({version})"),
            }
        );
    }

    #[test]
    fn a_utf16_version_is_answered_as_not_laid_out() {
        let dir = tempfile::tempdir().unwrap();
        let wide: Vec<u8> = "\u{feff}# Title\n\nbody\n"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        let version = kept(dir.path(), &wide);
        let size = u64::try_from(wide.len()).unwrap();
        let mut views = Views::new(dir.path());
        assert_eq!(
            preview(&mut views, version, 0, size),
            wire::Answer::Preview(Box::new(wire::PreviewAnswer {
                version,
                preview: Preview::Unsupported {
                    encoding: Encoding::Utf16Le,
                },
            }))
        );
    }
}
