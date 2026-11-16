// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::score::{AssetUse, Score};

pub const ASSET_IDLE_DAYS: u32 = 90;

pub const ASSET_FLOOR_PER_MILLE: u32 = 1_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Disposal {
    Keep,

    Warn { because: String },

    Retire { because: String },
}

impl Disposal {
    #[must_use]
    pub fn because(&self) -> &str {
        match self {
            Disposal::Keep => "it is used often enough for what it costs",
            Disposal::Warn { because } | Disposal::Retire { because } => because,
        }
    }
}

#[must_use]
pub fn dispose(usage: &AssetUse, score: Score, warned_already: bool) -> Disposal {
    let idle = usage.idle_days >= ASSET_IDLE_DAYS;
    let thin = score.per_mille < ASSET_FLOOR_PER_MILLE;
    if !idle && !thin {
        return Disposal::Keep;
    }
    let because = if idle && thin {
        format!(
            "nothing has used it for {} days, and it was reached for {} times for the room it takes",
            usage.idle_days, usage.uses
        )
    } else if idle {
        format!("nothing has used it for {} days", usage.idle_days)
    } else {
        format!(
            "it was reached for {} times for the room it takes in every prompt that offers it",
            usage.uses
        )
    };
    if warned_already {
        Disposal::Retire { because }
    } else {
        Disposal::Warn { because }
    }
}

#[must_use]
pub fn sweep<T: Clone>(assets: &[(T, AssetUse, Score, bool)]) -> Vec<(T, Disposal)> {
    assets
        .iter()
        .map(|(subject, usage, score, warned)| (subject.clone(), dispose(usage, *score, *warned)))
        .collect()
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
    use crate::score::score;
    use kernel::ByteLen;

    fn usage(uses: u32, bytes: u64, idle_days: u32) -> AssetUse {
        AssetUse {
            uses,
            resident: ByteLen::new(bytes),
            idle_days,
        }
    }

    #[test]
    fn a_full_cycle_warns_first_and_retires_second() {
        let forgotten = usage(0, 4_000, 120);
        let standing = score(&forgotten);
        let first = dispose(&forgotten, standing, false);
        assert!(matches!(first, Disposal::Warn { .. }));
        let second = dispose(&forgotten, standing, true);
        assert!(matches!(second, Disposal::Retire { .. }));
        assert!(
            second.because().contains("120 days"),
            "the reason travels with the verdict: {}",
            second.because()
        );
    }

    #[test]
    fn an_asset_that_pays_for_its_room_is_left_alone() {
        let busy = usage(60, 3_000, 2);
        assert_eq!(dispose(&busy, score(&busy), true), Disposal::Keep);
    }

    #[test]
    fn a_heavy_asset_that_is_barely_used_is_noticed_even_while_it_is_fresh() {
        let bloated = usage(1, 200_000, 0);
        let verdict = dispose(&bloated, score(&bloated), false);
        assert!(matches!(verdict, Disposal::Warn { .. }));
        assert!(verdict.because().contains("room it takes"));
    }

    #[test]
    fn a_sweep_reports_in_the_registers_own_order() {
        let a = usage(0, 4_000, 200);
        let b = usage(50, 1_000, 0);
        let verdicts = sweep(&[("old", a, score(&a), true), ("busy", b, score(&b), false)]);
        assert_eq!(verdicts.len(), 2);
        assert_eq!(verdicts[0].0, "old");
        assert!(matches!(verdicts[0].1, Disposal::Retire { .. }));
        assert_eq!(verdicts[1].1, Disposal::Keep);
    }
}
