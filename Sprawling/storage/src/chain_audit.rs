// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};

use kernel::{AxError, B3Hash};

use crate::error::StorageError;
use crate::jsonl::{JsonlLedger, LineCheck, segment_names};
use crate::real_fs::RealFs;
use crate::verified_prefix::{ProofRecords, SegmentRecord, line_check_version};
use wave::{Hashed, PROOF_WAVE, Read, read_wave};

mod wave;

#[derive(Debug, Clone, PartialEq)]
pub enum ChainAudit {

    Whole { lines: u64 },

    Broken(AxError),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProofCount {

    pub lines_checked: u64,

    pub segments_by_digest: u64,
    pub bytes_read: u64,
    pub bytes_hashed: u64,

    pub waves: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Proven {
    pub audit: ChainAudit,
    pub counted: ProofCount,
    pub unkept: Option<AxError>,
}

#[derive(Debug, Clone, Default)]
pub struct ChainHalt {
    verdict: Arc<(Mutex<Option<Verdict>>, Condvar)>,
    before: Before,
}

#[derive(Debug, Clone)]
enum Verdict {
    Whole,
    Broken(AxError),
}

#[derive(Debug, Clone, Copy, Default)]
enum Before {

    #[default]
    Admit,

    Refuse,
}

impl ChainHalt {

    pub fn awaiting_proof() -> ChainHalt {
        ChainHalt {
            verdict: Arc::default(),
            before: Before::Refuse,
        }
    }

    pub fn prove(&self) {
        self.settle(Verdict::Whole);
    }

    pub fn trip(&self, reason: AxError) {
        self.settle(Verdict::Broken(reason));
    }

    pub fn reason(&self) -> Option<AxError> {
        match &*self.held() {
            Some(Verdict::Broken(reason)) => Some(reason.clone()),
            Some(Verdict::Whole) | None => None,
        }
    }

    pub fn proved(&self) -> bool {
        matches!(&*self.held(), Some(Verdict::Whole))
    }

    pub fn await_verdict(&self) {
        match self.before {
            Before::Admit => {}
            Before::Refuse => {
                let (lock, settled) = &*self.verdict;
                let guard = lock.lock().unwrap_or_else(PoisonError::into_inner);
                drop(
                    settled
                        .wait_while(guard, |verdict| verdict.is_none())
                        .unwrap_or_else(PoisonError::into_inner),
                );
            }
        }
    }

    pub(crate) fn admit(&self) -> Result<(), StorageError> {
        match (&*self.held(), self.before) {
            (Some(Verdict::Whole), _) | (None, Before::Admit) => Ok(()),
            (Some(Verdict::Broken(reason)), _) => Err(StorageError::ChainHalted {
                source: reason.clone(),
            }),
            (None, Before::Refuse) => Err(StorageError::Unproven),
        }
    }

    fn settle(&self, verdict: Verdict) {
        let (lock, settled) = &*self.verdict;
        let mut held = lock.lock().unwrap_or_else(PoisonError::into_inner);
        if held.is_none() {
            *held = Some(verdict);
            settled.notify_all();
        }
    }

    fn held(&self) -> MutexGuard<'_, Option<Verdict>> {
        self.verdict
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

impl JsonlLedger {

    pub fn halt_on(&mut self, halt: ChainHalt) {
        self.halt = halt;
    }

    pub fn await_verdict(&self) {
        self.halt.await_verdict();
    }

    pub fn admits(&self) -> Result<(), StorageError> {
        self.halt.admit()
    }
}

pub fn audit_chain(dir: &Path) -> Result<ChainAudit, StorageError> {
    walk(dir, None).map(|proven| proven.audit)
}

pub fn prove_chain(dir: &Path, records: &ProofRecords) -> Result<Proven, StorageError> {
    walk(dir, Some(records))
}

fn walk(dir: &Path, records: Option<&ProofRecords>) -> Result<Proven, StorageError> {
    let version = line_check_version();
    let mut walked = Walked {
        check: LineCheck::at_genesis(),
        lines: 0,
        counted: ProofCount::default(),
        unkept: None,
    };
    for wave in segment_names(&RealFs::new(), dir)?.chunks(PROOF_WAVE) {
        walked.counted.waves = walked.counted.waves.saturating_add(1);
        for (name, read) in wave.iter().zip(read_wave(dir, wave, records, version)?) {
            let proved = match walked.segment(read, version) {
                Ok(proved) => proved,
                Err(broken) => {
                    return Ok(Proven {
                        audit: ChainAudit::Broken(broken),
                        counted: walked.counted,
                        unkept: walked.unkept,
                    });
                }
            };
            if let Some(records) = records {
                let kept = records.keep(name, &proved).map_err(StorageError::into_ax);
                walked.unkept = walked.unkept.take().or(kept.err());
            }
        }
    }
    Ok(Proven {
        audit: ChainAudit::Whole {
            lines: walked.lines,
        },
        counted: walked.counted,
        unkept: walked.unkept,
    })
}

struct Walked {
    check: LineCheck,
    lines: u64,
    counted: ProofCount,
    unkept: Option<AxError>,
}

impl Walked {

    fn segment(&mut self, read: Read, version: u32) -> Result<SegmentRecord, AxError> {
        let Read { bytes, hashed } = read;
        self.counted.bytes_read = self.counted.bytes_read.saturating_add(len_of(&bytes));
        let entry = self.check;
        let complete = bytes
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map_or(0, |at| at.saturating_add(1));
        let (from, mut in_segment, mut hasher) = hashed
            .and_then(|hashed| self.reuse(hashed))
            .unwrap_or_else(|| (0, 0, blake3::Hasher::new()));
        let rest = bytes.get(from..complete).unwrap_or_default();
        hasher.update(rest);
        self.counted.bytes_hashed = self.counted.bytes_hashed.saturating_add(len_of(rest));
        for line in rest.split(|byte| *byte == b'\n') {
            if line.is_empty() {
                continue;
            }
            self.lines = self.lines.saturating_add(1);
            in_segment = in_segment.saturating_add(1);
            self.counted.lines_checked = self.counted.lines_checked.saturating_add(1);
            self.check
                .advance(line)
                .map_err(|fault| fault.into_ax(self.lines))?;
        }
        Ok(SegmentRecord {
            version,
            len: len_of(bytes.get(..complete).unwrap_or_default()),
            lines: in_segment,
            entry,
            exit: self.check,
            digest: B3Hash::from_bytes(*hasher.finalize().as_bytes()),
        })
    }

    fn reuse(&mut self, hashed: Hashed) -> Option<(usize, u64, blake3::Hasher)> {
        let Hashed {
            record,
            len,
            hasher,
        } = hashed;
        self.counted.bytes_hashed = self
            .counted
            .bytes_hashed
            .saturating_add(u64::try_from(len).unwrap_or(u64::MAX));
        let (lines, exit) = record.stands(self.check, &hasher)?;
        self.lines = self.lines.saturating_add(lines);
        self.check = exit;
        self.counted.segments_by_digest = self.counted.segments_by_digest.saturating_add(1);
        Some((len, lines, hasher))
    }
}

fn len_of(bytes: &[u8]) -> u64 {
    u64::try_from(bytes.len()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests;
