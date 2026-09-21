// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which session a commit came out of, as five git trailers.
//!
//! A person reading `git log` in a city this harness works in has to be
//! able to answer "who wrote this line, on whose behalf, with which
//! model" without opening the Ledger. That answer is a **projection**:
//! the Ledger stays the authority for what happened, and the two sides
//! reconcile on the commit oid, which is written into
//! `checkpoint_committed` and into every `file_discarded` restoration.
//! Where a trailer and the Ledger disagree, the trailer is the side that
//! is wrong.
//!
//! The block is rendered as git's own trailer syntax rather than as a
//! prefix language of this repository's invention, so
//! `git interpret-trailers --parse` reads it with no help from us.

use std::path::Path;

use kernel::event::record::CommitAttribution;
use kernel::{Address, B3Hash, Effort, RunId};

use crate::error::MemoryError;
use crate::jsonl::ledger_segments_at;
use crate::real_fs::RealFs;
use crate::vfs::Vfs;

/// The model a run was given, and the thinking budget it was asked for.
///
/// The two always travel together — a model id without the effort asked
/// of it does not say what the call was — so they arrive as one value
/// rather than as two parameters a caller can hand over half of.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelChoice {
    /// The endpoint's own model id. Empty when the caller genuinely does
    /// not know it: an invented id would be worse than an absent one.
    pub id: String,
    /// Absent leaves the choice to the provider, which is not the same
    /// fact as `Effort::None` asking it not to think.
    pub effort: Option<Effort>,
}

/// Who made a commit, and on whose behalf.
///
/// Private fields with one construction point, so a commit signed by a
/// run that does not exist cannot be assembled a field at a time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenance {
    run: RunId,
    actor: Address,
    model: String,
    effort: Option<Effort>,
    city: B3Hash,
    predecessor: Option<RunId>,
}

impl Provenance {
    /// The one construction point.
    pub fn new(run: RunId, actor: Address, city: B3Hash, chosen: ModelChoice) -> Provenance {
        Provenance {
            run,
            actor,
            model: chosen.id,
            effort: chosen.effort,
            city,
            predecessor: None,
        }
    }

    /// Names the run this one replaced, so a commit can be asked for
    /// its lineage. A sixth trailer, present only when there is one.
    #[must_use]
    pub fn succeeding(mut self, predecessor: RunId) -> Provenance {
        self.predecessor = Some(predecessor);
        self
    }

    /// The run this one replaced, when it is a successor.
    pub fn predecessor(&self) -> Option<RunId> {
        self.predecessor
    }

    /// The city's identity: the chain hash of its genesis line.
    ///
    /// Only the first line of the first segment is read. A city's whole
    /// history can be tens of megabytes, and reading it to learn one
    /// hash would make every fence pay for the length of the city's
    /// life.
    ///
    /// # Errors
    /// Propagates a ledger directory that cannot be listed or read, and
    /// refuses a directory holding no ledger at all — a city with no
    /// genesis line has no identity to sign with.
    pub fn city_of(ledger_dir: &Path) -> Result<B3Hash, MemoryError> {
        let vfs = RealFs::new();
        for segment in ledger_segments_at(ledger_dir)? {
            let bytes = vfs.read(&segment).map_err(|source| MemoryError::Io {
                op: "read the genesis line",
                path: segment.clone(),
                source,
            })?;
            let first = match bytes.split(|byte| *byte == b'\n').next() {
                Some(line) if !line.is_empty() => line,
                _ => continue,
            };
            return Ok(kernel::ledger::chain_hash(first));
        }
        Err(MemoryError::Checkpoint {
            op: "read the city's genesis line",
            detail: format!("{} holds no ledger line", ledger_dir.display()),
        })
    }

    /// Who this commit is signed as.
    pub fn actor(&self) -> &Address {
        &self.actor
    }

    /// The run this commit belongs to, which names the reference a wave
    /// fence is filed under.
    pub fn run(&self) -> RunId {
        self.run
    }

    /// The git author line: the resident's address, at a mailbox derived
    /// from the city it lives in. The domain is unroutable on purpose —
    /// it identifies a city rather than promising to deliver mail.
    pub(crate) fn email(&self) -> String {
        let city = self.city.to_string();
        let short = city.get(..CITY_PREFIX_HEX).unwrap_or(city.as_str());
        format!("{}@{short}.sprawling", self.actor.as_str())
    }

    /// Which session made this commit, in the shape every record that
    /// names a commit carries it. Both `checkpoint_committed` and
    /// `pr_merged` are stamped from here, so the two cannot describe
    /// one commit differently.
    pub fn attribution(&self) -> CommitAttribution {
        CommitAttribution {
            model: self.model.clone(),
            effort: Some(recorded_effort(self.effort)),
            predecessor: self.predecessor,
        }
    }

    /// The trailers block, in this order and this spelling, one per line
    /// and newline-terminated. The sixth line appears only for a run
    /// that replaced another.
    pub fn trailers(&self) -> String {
        let mut out = format!(
            "Sprawling-Run: {}\nSprawling-Actor: {}\nSprawling-Model: {}\nSprawling-Effort: {}\nSprawling-City: {}\n",
            self.run,
            self.actor.as_str(),
            self.model,
            effort_word(recorded_effort(self.effort)),
            self.city,
        );
        if let Some(predecessor) = self.predecessor {
            out.push_str(&format!("Sprawling-Predecessor: {predecessor}\n"));
        }
        out
    }
}

/// How much of the city hash the author domain carries. Twelve hex
/// digits distinguish every city a person will ever have on one machine
/// while still fitting in a terminal beside the address.
const CITY_PREFIX_HEX: usize = 12;

/// How an effort nobody asked for is recorded: as [`Effort::None`].
///
/// The distinction `Option` draws — the provider chose, against we asked
/// it not to think — has never reached the git trailer or the ledger,
/// both of which have written `none` for either since the first fence.
/// Every output that has to pick a word goes through here, so one place
/// decides it and a record written today still reads back the way a
/// year-old build wrote it.
#[must_use]
pub fn recorded_effort(effort: Option<Effort>) -> Effort {
    effort.unwrap_or(Effort::None)
}

/// How an effort is spelled, taken from `kernel::Effort`'s own serde
/// names so this repository has one authority for the word rather than a
/// second table that drifts. A `match` here would be that second table:
/// it would compile while spelling one level differently from the wire.
///
/// Public because three readers show a person this word — the commit's
/// trailers, the ledger payload beside them, and `sprawling whose` —
/// and three spellings of one effort is how a reader starts doubting
/// all three.
#[must_use]
pub fn effort_word(effort: Effort) -> String {
    serde_json::to_value(effort)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_else(|| "none".to_owned())
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
    use super::*;

    pub(crate) fn sample() -> Provenance {
        Provenance::new(
            RunId::CITY,
            Address::parse("lab/parser").unwrap(),
            B3Hash::digest(b"a city"),
            ModelChoice {
                id: "claude-sonnet-4-6".to_owned(),
                effort: Some(Effort::High),
            },
        )
    }

    #[test]
    fn the_five_trailers_come_in_the_order_a_reader_was_promised() {
        let rendered = sample().trailers();
        let lines: Vec<String> = rendered.lines().map(str::to_owned).collect();
        let city = B3Hash::digest(b"a city");
        assert_eq!(
            lines,
            vec![
                format!("Sprawling-Run: {}", RunId::CITY),
                "Sprawling-Actor: lab/parser".to_owned(),
                "Sprawling-Model: claude-sonnet-4-6".to_owned(),
                "Sprawling-Effort: high".to_owned(),
                format!("Sprawling-City: {city}"),
            ]
        );
    }

    #[test]
    fn a_successor_signs_a_sixth_trailer_and_a_first_run_signs_five() {
        assert_eq!(sample().trailers().lines().count(), 5);
        let predecessor = RunId::from_bytes([3u8; 16]);
        let successor = sample().succeeding(predecessor);
        let rendered = successor.trailers();
        assert_eq!(rendered.lines().count(), 6);
        assert!(
            rendered.ends_with(&format!("Sprawling-Predecessor: {predecessor}\n")),
            "{rendered}"
        );
        assert_eq!(successor.predecessor(), Some(predecessor));
        assert_eq!(sample().predecessor(), None);
    }

    #[test]
    fn an_unknown_model_and_an_unasked_effort_are_written_as_what_they_are() {
        let bare = Provenance::new(
            RunId::CITY,
            Address::parse("lab/parser").unwrap(),
            B3Hash::digest(b"a city"),
            ModelChoice {
                id: String::new(),
                effort: None,
            },
        );
        let rendered = bare.trailers();
        assert!(rendered.contains("Sprawling-Model: \n"), "{rendered}");
        assert!(rendered.contains("Sprawling-Effort: none\n"), "{rendered}");
    }

    #[test]
    fn an_effort_nobody_asked_for_is_recorded_as_the_level_none() {
        // What the ledger record and the git trailer have always said,
        // now settled once instead of at each output.
        assert_eq!(recorded_effort(None), Effort::None);
        assert_eq!(recorded_effort(Some(Effort::High)), Effort::High);
        assert_eq!(effort_word(recorded_effort(None)), "none");
        assert_eq!(effort_word(Effort::XHigh), "xhigh");
    }

    #[test]
    fn the_author_mailbox_names_the_city_rather_than_a_host() {
        let city = B3Hash::digest(b"a city").to_string();
        assert_eq!(
            sample().email(),
            format!("lab/parser@{}.sprawling", city.get(..12).unwrap())
        );
    }

    #[test]
    fn a_directory_with_no_ledger_line_has_no_identity_to_sign_with() {
        let tmp = tempfile::tempdir().unwrap();
        let err = Provenance::city_of(tmp.path()).unwrap_err();
        assert!(err.to_string().contains("holds no ledger line"));
    }
}
