// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError, B3Hash, ByteLen, Locator};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructureNode {
    pub level: u8,
    pub title: String,

    pub offset: ByteLen,

    pub span: ByteLen,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Digest {
    source: B3Hash,
    origin: Option<Locator>,
    structure: Vec<StructureNode>,
    prose: Option<String>,
}

impl Digest {
    #[must_use]
    pub fn structural(source: B3Hash, origin: Option<Locator>, text: &str) -> Digest {
        Digest {
            source,
            origin,
            structure: structure_of(text),
            prose: None,
        }
    }

    #[must_use]
    pub fn with_prose(mut self, prose: String) -> Digest {
        self.prose = Some(prose);
        self
    }

    #[must_use]
    pub fn source(&self) -> B3Hash {
        self.source
    }

    #[must_use]
    pub fn origin(&self) -> Option<&Locator> {
        self.origin.as_ref()
    }

    #[must_use]
    pub fn structure(&self) -> &[StructureNode] {
        &self.structure
    }

    #[must_use]
    pub fn prose(&self) -> Option<&str> {
        self.prose.as_deref()
    }

    #[must_use]
    pub fn is_suspect(&self) -> bool {
        self.prose.is_some()
    }

    #[must_use]
    pub fn window_header(&self) -> String {
        let origin = self
            .origin
            .as_ref()
            .map_or_else(|| self.source.to_string(), Locator::to_string);
        if self.is_suspect() {
            format!("digest of {origin} (suspect: written, not read; the source decides)")
        } else {
            format!("structure of {origin} (read from the source)")
        }
    }
}

#[must_use]
pub fn structure_of(text: &str) -> Vec<StructureNode> {
    let mut nodes: Vec<StructureNode> = Vec::new();
    let mut starts: Vec<(usize, usize)> = Vec::new();
    let mut offset: usize = 0;
    let mut fenced = false;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            fenced = !fenced;
        } else if !fenced && trimmed.starts_with('#') {
            let level = trimmed.chars().take_while(|c| *c == '#').count();
            let title = trimmed
                .trim_start_matches('#')
                .trim_matches(|c: char| c == ' ' || c == '\n' || c == '\r')
                .to_owned();
            if (1..=6).contains(&level) && !title.is_empty() {
                let level = u8::try_from(level).unwrap_or(6);
                close_deeper(&mut nodes, &mut starts, level, offset);
                starts.push((nodes.len(), offset));
                nodes.push(StructureNode {
                    level,
                    title,
                    offset: ByteLen::new(u64::try_from(offset).unwrap_or(u64::MAX)),
                    span: ByteLen::new(0),
                });
            }
        }
        offset = offset.saturating_add(line.len());
    }
    close_deeper(&mut nodes, &mut starts, 0, offset);
    nodes
}

fn close_deeper(
    nodes: &mut [StructureNode],
    starts: &mut Vec<(usize, usize)>,
    level: u8,
    end: usize,
) {
    while let Some((index, start)) = starts.last().copied() {
        let Some(node) = nodes.get_mut(index) else {
            starts.pop();
            continue;
        };
        if level != 0 && node.level < level {
            break;
        }
        node.span = ByteLen::new(u64::try_from(end.saturating_sub(start)).unwrap_or(u64::MAX));
        starts.pop();
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Breaker {
    limit: u32,
    consecutive: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BreakerVerdict {
    Attempt,

    Open { after: u32 },
}

impl Breaker {
    #[must_use]
    pub fn new(limit: u32) -> Breaker {
        Breaker {
            limit,
            consecutive: 0,
        }
    }

    #[must_use]
    pub fn verdict(&self) -> BreakerVerdict {
        if self.limit > 0 && self.consecutive >= self.limit {
            BreakerVerdict::Open {
                after: self.consecutive,
            }
        } else {
            BreakerVerdict::Attempt
        }
    }

    pub fn succeeded(&mut self) {
        self.consecutive = 0;
    }

    pub fn failed(&mut self) {
        self.consecutive = self.consecutive.saturating_add(1);
    }

    #[must_use]
    pub fn consecutive_failures(&self) -> u32 {
        self.consecutive
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DigestOutcome {
    Cached(Digest),

    Fresh(Digest),

    Structural { digest: Digest, reason: AxError },
}

impl DigestOutcome {
    #[must_use]
    pub fn digest(&self) -> &Digest {
        match self {
            DigestOutcome::Cached(digest)
            | DigestOutcome::Fresh(digest)
            | DigestOutcome::Structural { digest, .. } => digest,
        }
    }
}

pub fn digest_once(
    text: &str,
    origin: Option<Locator>,
    breaker: &mut Breaker,
    cached: &mut dyn FnMut(&B3Hash) -> Result<Option<Digest>, AxError>,
    write_prose: &mut dyn FnMut(&str) -> Result<String, AxError>,
) -> Result<DigestOutcome, AxError> {
    let source = B3Hash::digest(text.as_bytes());
    if let Some(hit) = cached(&source)? {
        return Ok(DigestOutcome::Cached(hit));
    }
    let structural = Digest::structural(source, origin, text);
    if let BreakerVerdict::Open { after } = breaker.verdict() {
        return Ok(DigestOutcome::Structural {
            digest: structural,
            reason: AxError::failure(
                AxCode::DigestSuspect,
                "summarise a document",
                format!("the digester failed {after} times in a row"),
            )
            .with_recovery("read the structure and the source; prose resumes after one success"),
        });
    }
    match write_prose(text) {
        Ok(prose) => {
            breaker.succeeded();
            Ok(DigestOutcome::Fresh(structural.with_prose(prose)))
        }
        Err(reason) => {
            breaker.failed();
            Ok(DigestOutcome::Structural {
                digest: structural,
                reason,
            })
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests;
