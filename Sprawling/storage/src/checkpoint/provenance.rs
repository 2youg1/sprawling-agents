// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::event::record::CommitAttribution;
use kernel::{Address, B3Hash, Effort, RunId};

use crate::error::StorageError;
use crate::jsonl::{first_line, ledger_segments_at};
use crate::real_fs::RealFs;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ModelChoice {

    pub id: String,

    pub effort: Option<Effort>,
}

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

    #[must_use]
    pub fn succeeding(mut self, predecessor: RunId) -> Provenance {
        self.predecessor = Some(predecessor);
        self
    }

    pub fn predecessor(&self) -> Option<RunId> {
        self.predecessor
    }

    pub fn city_of(ledger_dir: &Path) -> Result<B3Hash, StorageError> {
        let vfs = RealFs::new();
        for segment in ledger_segments_at(ledger_dir)? {
            let first = first_line(&vfs, &segment).map_err(|source| StorageError::Io {
                op: "read the genesis line",
                path: segment.clone(),
                source,
            })?;
            match first {
                Some(line) if !line.is_empty() => return Ok(kernel::ledger::chain_hash(&line)),
                _ => continue,
            }
        }
        Err(StorageError::Checkpoint {
            op: "read the city's genesis line",
            detail: format!("{} holds no ledger line", ledger_dir.display()),
        })
    }

    pub fn actor(&self) -> &Address {
        &self.actor
    }

    pub fn run(&self) -> RunId {
        self.run
    }

    pub(crate) fn email(&self) -> String {
        let city = self.city.to_string();
        let short = city.get(..CITY_PREFIX_HEX).unwrap_or(city.as_str());
        format!("{}@{short}.sprawling", self.actor.as_str())
    }

    pub fn attribution(&self) -> CommitAttribution {
        CommitAttribution {
            model: self.model.clone(),
            effort: Some(recorded_effort(self.effort)),
            predecessor: self.predecessor,
        }
    }

    pub fn trailers(&self) -> String {
        let mut out = format!(
            "Sprawling-Run: {}\nSprawling-Actor: {}\nSprawling-Model: {}\nSprawling-Effort: {}\nSprawling-City: {}\n",
            self.run,
            self.actor.as_str(),
            self.model,
            recorded_effort(self.effort).as_str(),
            self.city,
        );
        if let Some(predecessor) = self.predecessor {
            out.push_str(&format!("Sprawling-Predecessor: {predecessor}\n"));
        }
        out
    }
}

const CITY_PREFIX_HEX: usize = 12;

#[must_use]
pub fn recorded_effort(effort: Option<Effort>) -> Effort {
    effort.unwrap_or(Effort::None)
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

        assert_eq!(recorded_effort(None), Effort::None);
        assert_eq!(recorded_effort(Some(Effort::High)), Effort::High);
        assert_eq!(recorded_effort(None).as_str(), "none");
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
