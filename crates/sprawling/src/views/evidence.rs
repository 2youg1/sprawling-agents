// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a run wrote down that somebody can check it by.
//!
//! Two records write a locator a reader can go and look at: the browser
//! tool storing a screenshot (`bin::browser_tool::stored`) and a plan
//! node being closed with a completion (`collab::ClaimEffect::PutDown`).
//! This folds those two and nothing else, so the answer is exactly the
//! set of things the Ledger says can be looked at again.
//!
//! **No bytes.** A row carries the locator and, for a picture, its two
//! sides; fetching the content is the asset endpoint's. A query that
//! inlined the pixels would charge "what did this run do" the price of
//! every screenshot it took.

use channels::{EventKind, EventRecord, RunId};
use kernel::Locator;

use super::holding::Views;

impl Views {
    /// Every locator this run left behind, oldest first.
    pub(super) fn evidence_answer(&mut self, run: RunId) -> channels::EvidenceAnswer {
        let items = self
            .records_of(run)
            .iter()
            .filter_map(evidence_in)
            .collect();
        channels::EvidenceAnswer { run, items }
    }
}

/// The one evidence row this record carries, if it carries one.
///
/// A record whose locator will not parse produces no row: inventing a
/// row that points at nothing is worse than one row fewer, and the
/// record itself is still in the history where the reader can see it.
fn evidence_in(record: &EventRecord) -> Option<channels::EvidenceItem> {
    let map = record.data().as_map();
    let at = record.seq();
    match record.kind() {
        EventKind::ToolResult => {
            let result = map.get("result")?.as_object()?;
            let locator = Locator::parse(result.get("image")?.as_str()?).ok()?;
            Some(channels::EvidenceItem {
                at,
                kind: channels::EvidenceKind::Screenshot,
                locator,
                picture: picture_in(result),
            })
        }
        EventKind::RoadmapFinished => {
            let locator = Locator::parse(map.get("evidence")?.as_str()?).ok()?;
            Some(channels::EvidenceItem {
                at,
                kind: channels::EvidenceKind::Finished,
                locator,
                picture: None,
            })
        }
        _ => None,
    }
}

/// The two sides and the media type, when the record wrote all three.
///
/// All three or none: a picture with one side is not a size a reader can
/// lay out with, and half a shape is what invites somebody to default
/// the other half.
fn picture_in(result: &serde_json::Map<String, serde_json::Value>) -> Option<channels::Picture> {
    let side = |name: &str| {
        result
            .get(name)
            .and_then(serde_json::Value::as_u64)
            .and_then(|held| u32::try_from(held).ok())
    };
    Some(channels::Picture {
        media_type: result.get("media_type")?.as_str()?.to_owned(),
        width: side("width")?,
        height: side("height")?,
    })
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
    use super::evidence_in;
    use channels::{B3Hash, EventDraft, EventKind, EventRecord, Payload, RunId, Seq, TimeMs};

    fn record(kind: EventKind, data: serde_json::Value) -> EventRecord {
        EventRecord::from_draft(
            EventDraft {
                run: RunId::from_bytes([7u8; 16]),
                t: TimeMs::new(1),
                who: "lab/parser".to_owned(),
                addr: None,
                kind,
                data: Payload::new(data.as_object().unwrap().clone()).unwrap(),
                ig: false,
            },
            Seq::new(4),
            B3Hash::digest(b"prev"),
        )
    }

    fn cas() -> String {
        format!("cas:b3-{}", "0".repeat(64))
    }

    #[test]
    fn a_stored_screenshot_is_a_row_with_its_two_sides() {
        let held = record(
            EventKind::ToolResult,
            serde_json::json!({
                "name": "browser",
                "result": { "image": cas(), "width": 1280, "height": 720,
                            "media_type": "image/png" },
            }),
        );
        let item = evidence_in(&held).expect("a screenshot is evidence");
        assert_eq!(item.kind, channels::EvidenceKind::Screenshot);
        assert_eq!(item.at, Seq::new(4));
        let picture = item.picture.expect("the record wrote all three");
        assert_eq!((picture.width, picture.height), (1280, 720));
    }

    #[test]
    fn a_finished_node_is_a_row_and_carries_no_picture() {
        let held = record(
            EventKind::RoadmapFinished,
            serde_json::json!({ "verb": "finished", "node": "2.3", "evidence": cas() }),
        );
        let item = evidence_in(&held).expect("a completion is evidence");
        assert_eq!(item.kind, channels::EvidenceKind::Finished);
        assert!(item.picture.is_none());
    }

    #[test]
    fn a_locator_this_build_cannot_read_makes_no_row_rather_than_an_empty_one() {
        let held = record(
            EventKind::ToolResult,
            serde_json::json!({ "result": { "image": "not a locator" } }),
        );
        assert!(evidence_in(&held).is_none());
    }

    #[test]
    fn an_ordinary_tool_result_is_not_evidence() {
        let held = record(
            EventKind::ToolResult,
            serde_json::json!({ "tool_use_id": "a", "result": { "lines": 412 } }),
        );
        assert!(evidence_in(&held).is_none());
    }
}
