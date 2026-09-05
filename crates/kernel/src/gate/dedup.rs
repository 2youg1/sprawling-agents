// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeSet;

use crate::idem::IdemKey;

/// Deliberately exhaustive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DedupVerdict {
    Fresh,
    Duplicate,
}

/// Idempotent dedup: judged before any unreplayable side effect —
/// decrypting, billing, outward delivery all come after (8.2). The seen
/// set is the caller's state; kernel only judges membership.
pub fn dedup(seen: &BTreeSet<IdemKey>, key: &IdemKey) -> DedupVerdict {
    if seen.contains(key) {
        DedupVerdict::Duplicate
    } else {
        DedupVerdict::Fresh
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
    use std::collections::BTreeSet;
    #[test]
    fn dedup_judges_membership_only() {
        let run = crate::event::RunId::CITY;
        let key = IdemKey::derive(&run, crate::event::Seq::FIRST, b"send mail");
        let mut seen = BTreeSet::new();
        assert_eq!(dedup(&seen, &key), DedupVerdict::Fresh);
        seen.insert(key);
        assert_eq!(dedup(&seen, &key), DedupVerdict::Duplicate);
        let other = IdemKey::derive(&run, crate::event::Seq::FIRST, b"send other mail");
        assert_eq!(dedup(&seen, &other), DedupVerdict::Fresh);
    }
}
