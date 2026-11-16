// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

use crate::NodeId;
use crate::locator::Locator;
use crate::plan::StopCause;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoadmapMoved {
    pub by: String,
    pub node: NodeId,
    #[serde(flatten)]
    pub step: RoadmapStep,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "verb", rename_all = "snake_case")]
pub enum RoadmapStep {
    Claimed {
        item: String,
    },
    Split {
        children: Vec<String>,
    },
    Finished {
        item: String,
        evidence: Locator,
    },
    Released {
        item: String,
        why: StopCause,
        line: String,
    },
    Blocked {
        item: String,
        why: StopCause,
        line: String,
    },
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;
    use crate::event::Payload;

    #[test]
    fn a_roadmap_line_writes_the_keys_the_hand_written_map_wrote() {
        let claimed = RoadmapMoved {
            by: "lab/parser".to_owned(),
            node: NodeId::parse("2.3").unwrap(),
            step: RoadmapStep::Claimed {
                item: "the lexer".to_owned(),
            },
        };
        let blocked = RoadmapMoved {
            by: "lab/parser".to_owned(),
            node: NodeId::parse("2").unwrap(),
            step: RoadmapStep::Blocked {
                item: "the lexer".to_owned(),
                why: StopCause::Stalled { repeats: 3 },
                line: "stalled".to_owned(),
            },
        };
        let bytes =
            |record: &RoadmapMoved| serde_json::to_string(&Payload::of(record).unwrap()).unwrap();
        assert_eq!(
            (bytes(&claimed), bytes(&blocked)),
            (
                r#"{"by":"lab/parser","item":"the lexer","node":"2.3","verb":"claimed"}"#.to_owned(),
                r#"{"by":"lab/parser","item":"the lexer","line":"stalled","node":"2","verb":"blocked","why":{"stalled":{"repeats":3}}}"#.to_owned(),
            )
        );
        assert_eq!(
            Payload::of(&claimed)
                .unwrap()
                .read::<RoadmapMoved>()
                .unwrap(),
            claimed
        );
    }
}
