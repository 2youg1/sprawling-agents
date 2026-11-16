// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The CPU set records `GetSystemCpuSetInformation` writes, read in safe
//! Rust on every platform (`crates/desktop/ffi/Spec.lean` D4 and the
//! record walk proved in its section 10). The leaf copies the records out
//! unread; this module walks them by each record's own `Size`, so a record
//! that claims less than a CPU set record or runs past the buffer stops
//! the walk instead of being read past.

/// The bytes of one `SYSTEM_CPU_SET_INFORMATION` record of the CPU set
/// type, and the least a record of any type may claim.
const RECORD: usize = 32;

/// `CpuSetInformation`, the one record type the SDK defines.
const CPU_SET_TYPE: u32 = 0;

/// One logical processor as Windows describes it. `core` and `cache` are
/// numbered inside the processor's group; `class` grows with speed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuSet {
    pub group: u16,
    pub logical: u8,
    pub core: u8,
    pub cache: u8,
    pub numa: u8,
    pub class: u8,
    pub flags: Flags,
}

/// The record's flag byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Flags(pub u8);

impl Flags {
    /// The processor is parked to save power at this moment.
    #[must_use]
    pub fn parked(self) -> bool {
        self.0 & 0x1 != 0
    }

    /// The processor is allocated to some process for its exclusive use.
    #[must_use]
    pub fn allocated(self) -> bool {
        self.0 & 0x2 != 0
    }

    /// The process the records were read for holds that allocation.
    #[must_use]
    pub fn allocated_to_this_process(self) -> bool {
        self.0 & 0x4 != 0
    }
}

/// The record at `offset` claims less than a record, or runs past the end
/// of the buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Malformed {
    pub offset: usize,
}

/// Every CPU set record in `bytes`, in the order written.
///
/// # Errors
/// [`Malformed`] at the first record whose `Size` is below a record's or
/// reaches past `bytes`; nothing after it is read.
pub fn parse(bytes: &[u8]) -> Result<Vec<CpuSet>, Malformed> {
    let mut sets = Vec::new();
    let mut offset = 0usize;
    while offset < bytes.len() {
        let malformed = Malformed { offset };
        let record = offset
            .checked_add(RECORD)
            .and_then(|end| bytes.get(offset..end))
            .ok_or(malformed)?;
        let size = usize::try_from(u32_at(record, 0).ok_or(malformed)?).map_err(|_| malformed)?;
        let next = offset.checked_add(size).ok_or(malformed)?;
        if size < RECORD || next > bytes.len() {
            return Err(malformed);
        }
        if u32_at(record, 4) == Some(CPU_SET_TYPE) {
            sets.push(read(record).ok_or(malformed)?);
        }
        offset = next;
    }
    Ok(sets)
}

fn read(record: &[u8]) -> Option<CpuSet> {
    let group = u16::from_le_bytes(field::<2>(record, 12)?);
    let byte = |at: usize| record.get(at).copied();
    Some(CpuSet {
        group,
        logical: byte(14)?,
        core: byte(15)?,
        cache: byte(16)?,
        numa: byte(17)?,
        class: byte(18)?,
        flags: Flags(byte(19)?),
    })
}

fn u32_at(record: &[u8], at: usize) -> Option<u32> {
    field::<4>(record, at).map(u32::from_le_bytes)
}

fn field<const N: usize>(record: &[u8], at: usize) -> Option<[u8; N]> {
    let end = at.checked_add(N)?;
    record.get(at..end)?.try_into().ok()
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    reason = "test code"
)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn record(size: u32, kind: u32, group: u16, logical: u8, core: u8, class: u8) -> Vec<u8> {
        let mut bytes = vec![0u8; usize::try_from(size).unwrap().max(RECORD)];
        bytes[0..4].copy_from_slice(&size.to_le_bytes());
        bytes[4..8].copy_from_slice(&kind.to_le_bytes());
        bytes[12..14].copy_from_slice(&group.to_le_bytes());
        bytes[14] = logical;
        bytes[15] = core;
        bytes[18] = class;
        bytes.truncate(usize::try_from(size).unwrap().max(RECORD));
        bytes
    }

    #[test]
    fn records_are_walked_by_their_own_size_and_other_types_are_skipped() {
        let mut bytes = record(32, 0, 0, 0, 0, 1);
        bytes.extend(record(40, 7, 0, 9, 9, 9));
        bytes.extend(record(32, 0, 1, 3, 2, 0));
        let sets = parse(&bytes).unwrap();
        let read: Vec<(u16, u8, u8, u8)> = sets
            .iter()
            .map(|s| (s.group, s.logical, s.core, s.class))
            .collect();
        assert_eq!(read, vec![(0, 0, 0, 1), (1, 3, 2, 0)]);
    }

    #[test]
    fn a_record_shorter_than_a_record_or_past_the_end_stops_the_walk() {
        let mut short = record(32, 0, 0, 0, 0, 0);
        short[0..4].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(parse(&short), Err(Malformed { offset: 0 }));
        let mut long = record(32, 0, 0, 0, 0, 0);
        long.extend(record(32, 0, 0, 1, 1, 0));
        long[32..36].copy_from_slice(&64u32.to_le_bytes());
        assert_eq!(parse(&long), Err(Malformed { offset: 32 }));
        assert_eq!(parse(&long[..20]), Err(Malformed { offset: 0 }));
    }

    proptest! {
        /// The Rust side's fuzz of the walk: any bytes at all are read
        /// without a panic, and an answer is a list or the first bad record.
        #[test]
        fn any_bytes_are_walked_without_reading_past_them(bytes in proptest::collection::vec(any::<u8>(), 0..512)) {
            match parse(&bytes) {
                Ok(sets) => prop_assert!(sets.len() * RECORD <= bytes.len()),
                Err(Malformed { offset }) => prop_assert!(offset < bytes.len()),
            }
        }
    }
}
