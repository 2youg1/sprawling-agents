// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::ByteLen;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AssetUse {

    pub uses: u32,

    pub resident: ByteLen,

    pub idle_days: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Score {

    pub per_mille: u32,

    pub idle_days: u32,
}

#[must_use]
pub fn score(usage: &AssetUse) -> Score {
    let bytes = usage.resident.get();
    let per_mille = if bytes == 0 {
        usage.uses.saturating_mul(1000)
    } else {
        let scaled = u64::from(usage.uses)
            .saturating_mul(1000)
            .saturating_mul(1000);
        let ratio = scaled.checked_div(bytes).unwrap_or(0);
        u32::try_from(ratio).unwrap_or(u32::MAX)
    };
    Score {
        per_mille,
        idle_days: usage.idle_days,
    }
}

#[must_use]
pub fn worst_first<T: Clone>(assets: &[(T, Score)]) -> Vec<(T, Score)> {
    let mut sorted = assets.to_vec();
    sorted.sort_by(|left, right| {
        left.1
            .per_mille
            .cmp(&right.1.per_mille)
            .then(right.1.idle_days.cmp(&left.1.idle_days))
    });
    sorted
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
    fn an_asset_that_is_reached_for_scores_above_one_that_is_not() {
        let used = score(&AssetUse {
            uses: 40,
            resident: ByteLen::new(2_000),
            idle_days: 0,
        });
        let hoarded = score(&AssetUse {
            uses: 1,
            resident: ByteLen::new(2_000),
            idle_days: 90,
        });
        assert!(used.per_mille > hoarded.per_mille);
    }

    #[test]
    fn size_is_what_a_use_is_measured_against() {
        let small = score(&AssetUse {
            uses: 10,
            resident: ByteLen::new(500),
            ..AssetUse::default()
        });
        let large = score(&AssetUse {
            uses: 10,
            resident: ByteLen::new(50_000),
            ..AssetUse::default()
        });
        assert!(
            small.per_mille > large.per_mille,
            "the same usefulness costs more when it takes more room in every prompt"
        );
    }

    #[test]
    fn nothing_resident_is_charged_for_nothing() {
        let free = score(&AssetUse {
            uses: 3,
            resident: ByteLen::new(0),
            ..AssetUse::default()
        });
        assert_eq!(free.per_mille, 3_000);
    }

    #[test]
    fn the_list_is_read_worst_first_and_idleness_breaks_the_tie() {
        let sorted = worst_first(&[
            (
                "fresh",
                Score {
                    per_mille: 0,
                    idle_days: 1,
                },
            ),
            (
                "useful",
                Score {
                    per_mille: 900,
                    idle_days: 0,
                },
            ),
            (
                "forgotten",
                Score {
                    per_mille: 0,
                    idle_days: 200,
                },
            ),
        ]);
        let order: Vec<&str> = sorted.iter().map(|(name, _)| *name).collect();
        assert_eq!(order, vec!["forgotten", "fresh", "useful"]);
    }
}
