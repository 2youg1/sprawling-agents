// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};

use kernel::layout::CityLayout;
use kernel::{Address, AxCode, AxError, B3Hash, EventKind, EventRecord, RunId, Seq};

const EPHEMERAL_SEGMENT: &str = "You have no standing identity and you take no work below you. Finish \
     the task in JOB.md, and say what you did and what you found by writing it into the documents in \
     your workspace. Nothing about you outlives this run.\n";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Identity {
    Resident(Resident),
    Ephemeral { addr: Address },
}

impl Identity {
    pub fn load(city_root: &Path, addr: &Address) -> Result<Identity, AxError> {
        let path = urbanite_path(city_root, addr);
        match std::fs::read(&path) {
            Ok(bytes) => Ok(Identity::Resident(Resident::new(
                addr.clone(),
                described(addr, bytes),
            ))),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                Ok(Identity::Ephemeral { addr: addr.clone() })
            }
            Err(err) => Err(AxError::failure(
                AxCode::StorageFatal,
                "read a resident description",
                format!("{}: {err}", path.display()),
            )
            .with_recovery("fix the file's permissions, or remove it to run as ephemeral")),
        }
    }

    #[must_use]
    pub fn segment_bytes(&self) -> Vec<u8> {
        match self {
            Identity::Resident(resident) => resident.urbanite.clone(),
            Identity::Ephemeral { .. } => EPHEMERAL_SEGMENT.as_bytes().to_vec(),
        }
    }

    #[must_use]
    pub fn addr(&self) -> &Address {
        match self {
            Identity::Resident(resident) => &resident.addr,
            Identity::Ephemeral { addr } => addr,
        }
    }

    #[must_use]
    pub fn who(&self) -> String {
        self.addr().as_str().to_owned()
    }
}

fn described(addr: &Address, written: Vec<u8>) -> Vec<u8> {
    let Some(discipline) = crate::spine_files::hall_discipline(addr) else {
        return written;
    };
    let mut out = Vec::new();
    paragraph(&mut out, discipline.opening.as_bytes());

    paragraph(&mut out, &crate::identity::persona(written));
    paragraph(&mut out, discipline.closing.as_bytes());
    out
}

fn paragraph(out: &mut Vec<u8>, part: &[u8]) {
    const NEWLINE: u8 = 10;
    if part.is_empty() {
        return;
    }
    if !out.is_empty() {
        out.push(NEWLINE);
    }
    out.extend_from_slice(part);
    if !part.ends_with(&[NEWLINE]) {
        out.push(NEWLINE);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resident {
    addr: Address,
    urbanite: Vec<u8>,
    digest: B3Hash,
}

impl Resident {
    fn new(addr: Address, urbanite: Vec<u8>) -> Resident {
        let digest = B3Hash::digest(&urbanite);
        Resident {
            addr,
            urbanite,
            digest,
        }
    }

    #[must_use]
    pub fn addr(&self) -> &Address {
        &self.addr
    }

    #[must_use]
    pub fn digest(&self) -> B3Hash {
        self.digest
    }
}

#[must_use]
pub fn urbanite_path(city_root: &Path, addr: &Address) -> PathBuf {
    if let Some(hall) = crate::spine_files::hall_identity_path(city_root, addr) {
        return hall;
    }
    CityLayout::new(city_root).urbanite(addr)
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Dossier {
    runs_started: u32,
    runs_frozen: u32,
    last_seq: Option<Seq>,
    last_run: Option<RunId>,
}

impl Dossier {
    #[must_use]
    pub fn new() -> Dossier {
        Dossier::default()
    }

    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "two kinds count a resident's runs; the rest of the vocabulary is not about them"
    )]
    pub fn apply(&mut self, who: &str, record: &EventRecord) {
        if record.who() != who {
            return;
        }
        match record.kind() {
            EventKind::RunStarted => self.runs_started = self.runs_started.saturating_add(1),
            EventKind::RunFrozen => self.runs_frozen = self.runs_frozen.saturating_add(1),
            _ => {}
        }
        self.last_seq = Some(record.seq());
        self.last_run = Some(record.run());
    }

    #[must_use]
    pub fn runs_started(&self) -> u32 {
        self.runs_started
    }

    #[must_use]
    pub fn runs_frozen(&self) -> u32 {
        self.runs_frozen
    }

    #[must_use]
    pub fn last_seq(&self) -> Option<Seq> {
        self.last_seq
    }

    #[must_use]
    pub fn last_run(&self) -> Option<RunId> {
        self.last_run
    }

    #[must_use]
    pub fn is_live(&self) -> bool {
        self.runs_started > self.runs_frozen
    }
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
    use kernel::{EventDraft, Payload, TimeMs};

    fn record(who: &str, kind: EventKind, seq: u64) -> EventRecord {
        EventRecord::from_draft(
            EventDraft {
                run: RunId::from_bytes([5; 16]),
                t: TimeMs::new(seq),
                who: who.to_owned(),
                addr: None,
                kind,
                data: Payload::empty(),
                ig: false,
            },
            Seq::new(seq),
            B3Hash::digest(b"prev"),
        )
    }

    #[test]
    fn an_address_with_a_description_is_a_resident_and_one_without_is_ephemeral() {
        let dir = tempfile::tempdir().unwrap();
        let addr = Address::parse("lab/room1").unwrap();
        std::fs::create_dir_all(dir.path().join("lab").join("room1")).unwrap();
        std::fs::write(
            urbanite_path(dir.path(), &addr),
            "# URBANITE.md\n\nWrites the failing test first.\n",
        )
        .unwrap();

        let identity = Identity::load(dir.path(), &addr).unwrap();
        let Identity::Resident(resident) = &identity else {
            panic!("an address with a description holds a resident");
        };
        assert_eq!(resident.addr(), &addr);
        assert!(identity.segment_bytes().starts_with(b"# URBANITE.md"));

        let empty = Address::parse("lab/room2").unwrap();
        let ephemeral = Identity::load(dir.path(), &empty).unwrap();
        assert!(matches!(ephemeral, Identity::Ephemeral { .. }));
        assert!(
            ephemeral
                .segment_bytes()
                .starts_with(b"You have no standing identity"),
            "an ephemeral worker is told so, rather than given a character"
        );
    }

    #[test]
    fn a_resident_segment_is_byte_stable_across_runs() {
        let dir = tempfile::tempdir().unwrap();
        let addr = Address::parse("lab/room1").unwrap();
        std::fs::create_dir_all(dir.path().join("lab").join("room1")).unwrap();
        std::fs::write(urbanite_path(dir.path(), &addr), "same every time\n").unwrap();

        let first = Identity::load(dir.path(), &addr).unwrap();
        let second = Identity::load(dir.path(), &addr).unwrap();
        assert_eq!(first.segment_bytes(), second.segment_bytes());
        assert_eq!(first, second);

        std::fs::write(urbanite_path(dir.path(), &addr), "now different\n").unwrap();
        let third = Identity::load(dir.path(), &addr).unwrap();
        assert_ne!(first.segment_bytes(), third.segment_bytes());
    }

    #[test]
    fn a_dossier_counts_only_this_residents_runs_and_survives_the_run_that_made_it() {
        let mut dossier = Dossier::new();
        dossier.apply("lab/room1", &record("lab/room1", EventKind::RunStarted, 1));
        dossier.apply("lab/room1", &record("lab/room2", EventKind::RunStarted, 2));
        assert_eq!(dossier.runs_started(), 1, "another room's run is not mine");
        assert!(dossier.is_live());

        dossier.apply("lab/room1", &record("lab/room1", EventKind::RunFrozen, 3));
        assert!(!dossier.is_live(), "the run ended; the identity did not");
        assert_eq!(dossier.runs_frozen(), 1);

        dossier.apply("lab/room1", &record("lab/room1", EventKind::RunStarted, 4));
        assert_eq!(
            dossier.runs_started(),
            2,
            "a resident crosses runs; that is what makes it standing"
        );
        assert_eq!(dossier.last_seq(), Some(Seq::new(4)));
    }
}
