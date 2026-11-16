// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::PathBuf;

use kernel::ledger::chain_hash;
use kernel::{
    AxCode, AxError, EventDraft, EventKind, EventRecord, EventRef, Payload, RunId, Seq, TimeMs,
};

use crate::error::{StorageError, io_err};

use super::barrier::Barrier;
use super::ledger::{JsonlLedger, WriteObserver, is_segment, segment_file_name, u64_count};

impl JsonlLedger {
    pub(crate) fn append_log_truncated(
        &mut self,
        now: TimeMs,
        dropped: u64,
    ) -> Result<(), StorageError> {
        let data = Payload::of(&kernel::event::record::LogTruncated {
            dropped_bytes: dropped,
        })
        .map_err(|source| StorageError::Draft { source })?;
        let draft = EventDraft {
            run: RunId::CITY,
            t: now,
            who: kernel::event::Who::City.to_string(),
            addr: None,
            kind: EventKind::LogTruncated,
            data,
            ig: false,
        };
        self.append_all(vec![draft]).map(|_| ())
    }

    pub fn append_all(&mut self, drafts: Vec<EventDraft>) -> Result<Vec<EventRef>, StorageError> {
        self.finish_unwind()?;
        self.barrier.admit(&self.dir, self.next_seq)?;
        self.halt.admit()?;
        if drafts.is_empty() {
            return Ok(Vec::new());
        }
        let mut seq = self.next_seq;
        let mut prev = self.prev;
        let mut cur_path = self.seg_path.clone();
        let mut cur_len = self.seg_len;
        let mut writes: Vec<(PathBuf, u64, Vec<u8>)> = Vec::new();
        let mut created: Vec<PathBuf> = Vec::new();
        let mut records = Vec::with_capacity(drafts.len());

        if cur_len == 0 && !self.vfs.exists(&cur_path) {
            created.push(cur_path.clone());
        }

        for draft in drafts {
            let record = EventRecord::from_draft(draft, seq, prev);
            let line = record
                .canonical_line()
                .map_err(|source| StorageError::Draft { source })?;
            let line_len = u64_count(line.len())
                .map_err(io_err("measure an event line", &cur_path))?
                .saturating_add(1);
            if cur_len > 0 && cur_len.saturating_add(line_len) > self.roll_bytes {
                cur_path = self.dir.join(segment_file_name(seq));
                cur_len = 0;
                created.push(cur_path.clone());
            }
            prev = chain_hash(&line);
            seq = seq
                .next()
                .map_err(|source| StorageError::Draft { source })?;
            records.push(record);
            let mut terminated = line;
            terminated.push(b'\n');
            match writes.last_mut() {
                Some((path, _, buffer)) if *path == cur_path => {
                    buffer.extend_from_slice(&terminated);
                }
                _ => writes.push((cur_path.clone(), cur_len, terminated)),
            }
            cur_len = cur_len.saturating_add(line_len);
        }

        self.barrier = Barrier::Broken;
        self.write_wave(&writes, created)?;

        self.barrier = Barrier::Whole;
        self.seg_path = cur_path;
        self.seg_len = cur_len;
        self.next_seq = seq;
        self.prev = prev;

        if let Some(sessions) = self.sessions.as_mut() {
            for record in &records {
                if let Err(error) = sessions.absorb(record) {
                    eprintln!("a session slice was refused and skipped: {error}");
                }
            }
        }

        if let Some(observer) = self.observer.as_mut() {
            for record in &records {
                observer(record);
            }
        }
        Ok(records.iter().map(EventRecord::to_ref).collect())
    }

    pub fn hand_off_session_slices(&mut self) -> Option<crate::sessions::Sessions> {
        self.sessions.take()
    }

    #[must_use]
    pub fn position(&self) -> Seq {
        self.next_seq
    }

    pub fn observe(&mut self, sink: WriteObserver) {
        self.observer = Some(sink);
    }

    pub fn read_raw_lines(&self) -> Result<Vec<Vec<u8>>, StorageError> {
        let mut out = Vec::new();
        let segments: Vec<PathBuf> = self
            .vfs
            .list(&self.dir)
            .map_err(io_err("list ledger dir", &self.dir))?
            .into_iter()
            .filter(|p| is_segment(p))
            .collect();
        for seg in segments {
            out.extend(super::reading::read_lines(self.vfs.as_ref(), &seg)?);
        }
        Ok(out)
    }

    #[cfg(test)]
    pub(crate) fn set_roll_bytes_for_test(&mut self, roll_bytes: u64) {
        self.roll_bytes = roll_bytes;
    }
}

impl kernel::Ledger for JsonlLedger {
    fn append(&mut self, draft: EventDraft) -> Result<EventRef, AxError> {
        let refs = self
            .append_all(vec![draft])
            .map_err(StorageError::into_ax)?;
        refs.into_iter().next().ok_or_else(|| {
            AxError::failure(AxCode::InvalidArgs, "append event", "empty wave echo").with_recovery(
                "report this against storage::jsonl::append with the city path: \
                         append_all owes one echo per draft and returned none",
            )
        })
    }

    fn append_all(&mut self, drafts: Vec<EventDraft>) -> Result<Vec<EventRef>, AxError> {
        JsonlLedger::append_all(self, drafts).map_err(StorageError::into_ax)
    }
}

#[cfg(any(test, feature = "conformance"))]
impl kernel::ledger::conformance::LedgerInspect for JsonlLedger {
    fn raw_lines(&self) -> Result<Vec<Vec<u8>>, AxError> {
        self.read_raw_lines().map_err(StorageError::into_ax)
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
    use kernel::GENESIS_PREV;
    use std::fs;
    use std::path::Path;
    fn draft(kind: EventKind, t: u64) -> EventDraft {
        EventDraft {
            run: RunId::CITY,
            t: TimeMs::new(t),
            who: "city".to_string(),
            addr: None,
            kind,
            data: Payload::empty(),
            ig: false,
        }
    }
    fn verify_chain(lines: &[Vec<u8>]) {
        let mut prev = GENESIS_PREV;
        for (i, line) in lines.iter().enumerate() {
            let record = EventRecord::parse_line(line).unwrap();
            assert_eq!(record.prev(), prev, "prev broken at line {i}");
            assert_eq!(record.seq(), Seq::new(u64::try_from(i).unwrap()));
            prev = chain_hash(line);
        }
    }

    #[test]
    fn passes_the_kernel_conformance_suite() {
        use kernel::ledger::conformance::assert_ledger_conformance;
        let keep: std::cell::RefCell<Vec<tempfile::TempDir>> = std::cell::RefCell::new(Vec::new());
        assert_ledger_conformance(|| {
            let dir = tempfile::tempdir().unwrap();
            let (ledger, _) = JsonlLedger::open(dir.path(), TimeMs::new(0)).unwrap();
            keep.borrow_mut().push(dir);
            ledger
        });
    }

    #[test]
    fn rolls_segments_without_breaking_seq_or_chain() {
        let dir = tempfile::tempdir().unwrap();
        let (mut ledger, _) = JsonlLedger::open(dir.path(), TimeMs::new(0)).unwrap();
        ledger.set_roll_bytes_for_test(1);
        for t in 0..4 {
            ledger
                .append_all(vec![draft(EventKind::GateChecked, t)])
                .unwrap();
        }
        let segments = fs::read_dir(dir.path()).unwrap().count();
        assert!(segments >= 4, "tiny roll budget must produce many segments");
        let lines = ledger.read_raw_lines().unwrap();
        assert_eq!(lines.len(), 4);
        verify_chain(&lines);

        drop(ledger);
        let (mut reopened, report) = JsonlLedger::open(dir.path(), TimeMs::new(9)).unwrap();
        assert!(report.recovered.is_none());
        reopened
            .append_all(vec![draft(EventKind::RunFrozen, 9)])
            .unwrap();
        verify_chain(&reopened.read_raw_lines().unwrap());
    }

    #[test]
    fn one_append_costs_a_fixed_number_of_disk_operations() {
        let dir = tempfile::tempdir().unwrap();
        let fs = crate::fault_fs::FaultFs::new(crate::fault_fs::FaultPlan {
            cut_at_op: None,
            cut_on_write: None,
            torn_tail: crate::fault_fs::TornTail::None,
        });
        let (mut ledger, _) =
            JsonlLedger::open_faulty(fs.clone(), dir.path(), TimeMs::new(0)).unwrap();
        let opened = fs.op_count();
        kernel::Ledger::append(&mut ledger, draft(EventKind::GateChecked, 1)).unwrap();
        let first = fs.op_count().saturating_sub(opened);
        kernel::Ledger::append(&mut ledger, draft(EventKind::GateChecked, 2)).unwrap();
        let second = fs.op_count().saturating_sub(opened).saturating_sub(first);
        let bytes: u64 = ledger
            .read_raw_lines()
            .unwrap()
            .iter()
            .map(|line| u64::try_from(line.len().saturating_add(1)).unwrap())
            .sum();
        eprintln!(
            "ledger_append_ops: {first} disk ops for the first record, {second} for \n             every one after it, {bytes} B on disk for two records"
        );
        assert!(
            first > second,
            "the first record also creates the segment and syncs its directory entry"
        );
    }

    fn only_segment(dir: &Path) -> std::path::PathBuf {
        let mut files: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        files.sort();
        assert_eq!(files.len(), 1);
        files.remove(0)
    }

    fn faulty(dir: &Path) -> (JsonlLedger, crate::fault_fs::FaultFs) {
        let fs = crate::fault_fs::FaultFs::new(crate::fault_fs::FaultPlan {
            cut_at_op: None,
            cut_on_write: None,
            torn_tail: crate::fault_fs::TornTail::None,
        });
        let (ledger, _) = JsonlLedger::open_faulty(fs.clone(), dir, TimeMs::new(0)).unwrap();
        (ledger, fs)
    }

    fn drafts(count: u64) -> Vec<EventDraft> {
        (0..count)
            .map(|t| draft(EventKind::GateChecked, t))
            .collect()
    }

    #[test]
    fn a_wave_costs_one_write_and_one_barrier_whatever_it_carries() {
        let dir = tempfile::tempdir().unwrap();
        let (mut ledger, fs) = faulty(dir.path());
        ledger.append_all(drafts(1)).unwrap();
        let before_one = fs.op_count();
        ledger.append_all(drafts(1)).unwrap();
        let one = fs.op_count() - before_one;
        let before_three = fs.op_count();
        ledger.append_all(drafts(3)).unwrap();
        let three = fs.op_count() - before_three;
        assert_eq!(three, one);
    }

    #[test]
    fn a_wave_across_the_roll_puts_each_line_in_the_segment_named_for_it() {
        let dir = tempfile::tempdir().unwrap();
        let (mut ledger, _) = JsonlLedger::open(dir.path(), TimeMs::new(0)).unwrap();
        ledger.set_roll_bytes_for_test(1);
        ledger.append_all(drafts(3)).unwrap();
        let mut held: Vec<(String, Vec<u64>)> = fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| is_segment(path))
            .map(|path| {
                let seqs = fs::read(&path)
                    .unwrap()
                    .split(|byte| *byte == b'\n')
                    .filter(|line| !line.is_empty())
                    .map(|line| EventRecord::parse_line(line).unwrap().seq().value())
                    .collect();
                (
                    path.file_name().unwrap().to_string_lossy().into_owned(),
                    seqs,
                )
            })
            .collect();
        held.sort();
        let expected: Vec<(String, Vec<u64>)> = (0..3)
            .map(|seq| (segment_file_name(Seq::new(seq)), vec![seq]))
            .collect();
        assert_eq!(held, expected);
    }

    #[test]
    fn a_line_that_fills_the_segment_exactly_does_not_roll() {
        let dir = tempfile::tempdir().unwrap();
        let (mut ledger, _) = JsonlLedger::open(dir.path(), TimeMs::new(0)).unwrap();
        ledger.append_all(drafts(2)).unwrap();
        let exact = u64::try_from(fs::read(only_segment(dir.path())).unwrap().len()).unwrap();
        let fresh = tempfile::tempdir().unwrap();
        let (mut ledger, _) = JsonlLedger::open(fresh.path(), TimeMs::new(0)).unwrap();
        ledger.set_roll_bytes_for_test(exact);
        ledger.append_all(drafts(1)).unwrap();
        ledger.append_all(drafts(1)).unwrap();
        assert_eq!(
            fs::read(only_segment(fresh.path())).unwrap().len(),
            usize::try_from(exact).unwrap()
        );
    }

    #[test]
    fn the_observer_hears_each_record_of_a_wave_in_order() {
        let dir = tempfile::tempdir().unwrap();
        let (mut ledger, _) = JsonlLedger::open(dir.path(), TimeMs::new(0)).unwrap();
        let heard = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let sink = std::sync::Arc::clone(&heard);
        ledger.observe(Box::new(move |record| {
            sink.lock().unwrap().push(record.seq().value());
        }));
        ledger.append_all(drafts(3)).unwrap();
        assert_eq!(*heard.lock().unwrap(), vec![0, 1, 2]);
    }

    #[test]
    fn the_port_s_batch_append_echoes_what_it_wrote() {
        let dir = tempfile::tempdir().unwrap();
        let (mut ledger, _) = JsonlLedger::open(dir.path(), TimeMs::new(0)).unwrap();
        let echoed = kernel::Ledger::append_all(&mut ledger, drafts(2)).unwrap();
        let written: Vec<_> = ledger
            .read_raw_lines()
            .unwrap()
            .iter()
            .map(|line| EventRecord::parse_line(line).unwrap().to_ref())
            .collect();
        assert_eq!(echoed, written);
    }

    #[test]
    fn an_empty_segment_a_crash_left_is_appended_to_without_a_directory_barrier() {
        let dir = tempfile::tempdir().unwrap();
        let fs = crate::fault_fs::FaultFs::new(crate::fault_fs::FaultPlan {
            cut_at_op: None,
            cut_on_write: None,
            torn_tail: crate::fault_fs::TornTail::None,
        });

        let mut seeded = fs.clone();
        crate::vfs::Vfs::create_dir_all(&mut seeded, dir.path()).unwrap();
        crate::vfs::Vfs::append(
            &mut seeded,
            &dir.path().join(segment_file_name(Seq::FIRST)),
            b"",
        )
        .unwrap();
        let (mut ledger, _) =
            JsonlLedger::open_faulty(fs.clone(), dir.path(), TimeMs::new(0)).unwrap();
        let before_first = fs.op_count();
        ledger.append_all(drafts(1)).unwrap();
        let first = fs.op_count() - before_first;
        let before_second = fs.op_count();
        ledger.append_all(drafts(1)).unwrap();
        let second = fs.op_count() - before_second;
        assert_eq!(first, second);
    }

    proptest::proptest! {
        #[test]
        fn any_tail_damage_recovers_to_a_valid_chain(
            n in 1usize..5,
            cut_back in 1usize..40,
            garbage in proptest::collection::vec(1u8..=255, 0..24),
        ) {
            let dir = tempfile::tempdir().unwrap();
            let (mut ledger, _) = JsonlLedger::open(dir.path(), TimeMs::new(0)).unwrap();
            let drafts: Vec<_> = (0..n)
                .map(|i| draft(EventKind::GateChecked, u64::try_from(i).unwrap()))
                .collect();
            ledger.append_all(drafts).unwrap();
            let baseline = ledger.read_raw_lines().unwrap();
            let seg = only_segment(dir.path());
            drop(ledger);

            let mut bytes = fs::read(&seg).unwrap();
            let keep = bytes.len().saturating_sub(cut_back);
            bytes.truncate(keep);
            bytes.extend_from_slice(&garbage);
            fs::write(&seg, &bytes).unwrap();

            let (reopened, _) = JsonlLedger::open(dir.path(), TimeMs::new(999)).unwrap();
            let lines = reopened.read_raw_lines().unwrap();
            verify_chain(&lines);

            let survivors = lines
                .iter()
                .filter(|l| {
                    EventRecord::parse_line(l).unwrap().kind() != EventKind::LogTruncated
                })
                .count();
            proptest::prop_assert!(survivors <= baseline.len());
            for (mine, original) in lines.iter().take(survivors).zip(baseline.iter()) {
                proptest::prop_assert_eq!(mine, original);
            }
        }
    }
}
