// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::error::AxError;
use crate::event::{EventDraft, EventRef};
use crate::locator::B3Hash;

pub const GENESIS_PREV: B3Hash = B3Hash::from_bytes([0; 32]);

pub fn chain_hash(raw_line: &[u8]) -> B3Hash {
    B3Hash::from_bytes(*blake3::hash(raw_line).as_bytes())
}

pub trait Ledger {
    fn append(&mut self, draft: EventDraft) -> Result<EventRef, AxError>;

    fn append_all(&mut self, drafts: Vec<EventDraft>) -> Result<Vec<EventRef>, AxError> {
        let mut refs = Vec::with_capacity(drafts.len());
        for draft in drafts {
            refs.push(self.append(draft)?);
        }
        Ok(refs)
    }
}

#[cfg(feature = "conformance")]
pub mod conformance {

    use super::{GENESIS_PREV, Ledger, chain_hash};
    use crate::address::Address;
    use crate::error::AxError;
    use crate::event::{EventDraft, EventKind, EventRecord, Payload, RunId, Seq, TimeMs};

    pub trait LedgerInspect {
        fn raw_lines(&self) -> Result<Vec<Vec<u8>>, AxError>;
    }

    fn script() -> Vec<EventDraft> {
        let run = RunId::from_bytes([1; 16]);
        let parsed = Address::parse("lab");
        assert!(parsed.is_ok(), "fixture address must parse: {parsed:?}");
        let addr = parsed.ok();
        vec![
            EventDraft {
                run: RunId::CITY,
                t: TimeMs::new(1),
                who: "city".to_owned(),
                addr: None,
                kind: EventKind::CityInitialized,
                data: Payload::empty(),
                ig: false,
            },
            EventDraft {
                run: RunId::CITY,
                t: TimeMs::new(2),
                who: "city".to_owned(),
                addr,
                kind: EventKind::BuildingCreated,
                data: Payload::empty(),
                ig: false,
            },
            EventDraft {
                run,
                t: TimeMs::new(3),
                who: "planner@lab.1".to_owned(),
                addr: None,
                kind: EventKind::RunStarted,
                data: Payload::empty(),
                ig: false,
            },
            EventDraft {
                run,
                t: TimeMs::new(4),
                who: "planner@lab.1".to_owned(),
                addr: None,
                kind: EventKind::ToolCalled,
                data: Payload::empty(),
                ig: false,
            },
            EventDraft {
                run,
                t: TimeMs::new(5),
                who: "planner@lab.1".to_owned(),
                addr: None,
                kind: EventKind::RunFrozen,
                data: Payload::empty(),
                ig: true,
            },
        ]
    }

    fn collect_lines<L: Ledger + LedgerInspect>(ledger: &mut L) -> Vec<Vec<u8>> {
        for (index, draft) in script().into_iter().enumerate() {
            let kind = draft.kind;
            let appended = ledger.append(draft);
            assert!(
                appended.is_ok(),
                "append {index} ({kind:?}) must succeed: {appended:?}"
            );
            let Ok(echo) = appended else {
                return Vec::new();
            };

            assert_eq!(echo.kind(), kind, "ref kind must echo the draft");
            assert_eq!(
                echo.seq(),
                Seq::new(u64::try_from(index).unwrap_or(u64::MAX))
            );
        }
        let lines = ledger.raw_lines();
        assert!(lines.is_ok(), "raw_lines must succeed: {lines:?}");
        let Ok(lines) = lines else { return Vec::new() };
        lines
    }

    pub fn assert_ledger_conformance<L, F>(mut fresh: F)
    where
        L: Ledger + LedgerInspect,
        F: FnMut() -> L,
    {
        let mut first = fresh();
        let lines = collect_lines(&mut first);
        assert_eq!(lines.len(), script().len(), "one line per draft");

        let mut prev = GENESIS_PREV;
        for (index, line) in lines.iter().enumerate() {
            let parsed = EventRecord::parse_line(line);
            assert!(parsed.is_ok(), "line {index} must parse: {parsed:?}");
            let Ok(record) = parsed else { return };

            assert_eq!(
                record.prev(),
                prev,
                "line {index}: prev must hash the previous line"
            );

            assert_eq!(
                record.seq(),
                Seq::new(u64::try_from(index).unwrap_or(u64::MAX)),
                "line {index}: seq must be contiguous"
            );

            assert_eq!(
                record.v(),
                crate::consts_external::EVENT_LOG_V,
                "line {index}: v must be EVENT_LOG_V"
            );

            let echo = record.canonical_line();
            assert!(echo.is_ok(), "line {index} must reserialize");
            let Ok(echo) = echo else { return };
            assert_eq!(
                &echo, line,
                "line {index}: adapter bytes must be the canonical bytes"
            );
            prev = chain_hash(line);
        }

        let mut second = fresh();
        let again = collect_lines(&mut second);
        assert_eq!(
            lines, again,
            "two fresh instances fed the same drafts must produce identical bytes"
        );
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

    #[test]
    fn genesis_prev_is_sixty_four_zeros() {
        assert_eq!(GENESIS_PREV.to_string(), "0".repeat(64));
    }

    #[test]
    fn chain_hash_is_plain_blake3_of_the_line_bytes() {
        let line = b"{\"v\":1}";
        assert_eq!(
            chain_hash(line).to_string(),
            blake3::hash(line).to_hex().to_string()
        );
    }
}

#[cfg(all(test, feature = "conformance"))]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod conformance_self_test {
    use super::conformance::{LedgerInspect, assert_ledger_conformance};
    use super::*;
    use crate::error::{AxCode, AxError};
    use crate::event::{EventDraft, EventRecord, EventRef, Seq};

    struct VecLedger {
        lines: Vec<Vec<u8>>,
        next_seq: Seq,
        prev: B3Hash,
    }

    impl VecLedger {
        fn new() -> Self {
            VecLedger {
                lines: Vec::new(),
                next_seq: Seq::FIRST,
                prev: GENESIS_PREV,
            }
        }
    }

    impl Ledger for VecLedger {
        fn append(&mut self, draft: EventDraft) -> Result<EventRef, AxError> {
            let record = EventRecord::from_draft(draft, self.next_seq, self.prev);
            let line = record.canonical_line()?;
            self.prev = chain_hash(&line);
            self.next_seq = self.next_seq.next()?;
            self.lines.push(line);
            Ok(record.to_ref())
        }
    }

    impl LedgerInspect for VecLedger {
        fn raw_lines(&self) -> Result<Vec<Vec<u8>>, AxError> {
            Ok(self.lines.clone())
        }
    }

    #[test]
    fn the_reference_implementation_passes_the_suite() {
        assert_ledger_conformance(VecLedger::new);
    }

    #[test]
    fn a_chain_breaking_implementation_fails_the_suite() {
        struct BrokenPrev(VecLedger);
        impl Ledger for BrokenPrev {
            fn append(&mut self, draft: EventDraft) -> Result<EventRef, AxError> {
                let echo = self.0.append(draft)?;
                self.0.prev = GENESIS_PREV;
                Ok(echo)
            }
        }
        impl LedgerInspect for BrokenPrev {
            fn raw_lines(&self) -> Result<Vec<Vec<u8>>, AxError> {
                self.0.raw_lines()
            }
        }
        let outcome = std::panic::catch_unwind(|| {
            assert_ledger_conformance(|| BrokenPrev(VecLedger::new()));
        });
        assert!(outcome.is_err(), "the suite must bite a broken chain");
    }

    #[test]
    fn a_wave_appended_at_once_is_the_log_of_its_drafts_one_by_one() {
        let drafts: Vec<EventDraft> = [
            crate::event::EventKind::CityInitialized,
            crate::event::EventKind::RunStarted,
            crate::event::EventKind::RunFrozen,
        ]
        .into_iter()
        .enumerate()
        .map(|(at, kind)| EventDraft {
            run: crate::event::RunId::CITY,
            t: crate::event::TimeMs::new(u64::try_from(at).unwrap()),
            who: "city".to_owned(),
            addr: None,
            kind,
            data: crate::event::Payload::empty(),
            ig: false,
        })
        .collect();
        let mut one_by_one = VecLedger::new();
        let singly: Vec<EventRef> = drafts
            .iter()
            .map(|draft| one_by_one.append(draft.clone()).unwrap())
            .collect();
        let mut at_once = VecLedger::new();
        let waved = at_once.append_all(drafts).unwrap();
        assert_eq!(waved, singly);
        assert_eq!(
            at_once.raw_lines().unwrap(),
            one_by_one.raw_lines().unwrap()
        );
    }

    #[test]
    fn suite_error_paths_use_invalid_args() {
        let err = AxError::failure(AxCode::InvalidArgs, "x", "y").with_recovery("z");
        assert_eq!(err.code(), &AxCode::InvalidArgs);
    }
}
