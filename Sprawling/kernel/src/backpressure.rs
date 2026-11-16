// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueStats {
    pub depth: u64,
    pub capacity: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemMeta {
    pub cost: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShedReason {
    CapacityExhausted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Admission {
    Admit,
    Shed { reason: ShedReason },
}

pub fn admit(stats: &QueueStats, item: &ItemMeta) -> Admission {
    match stats.depth.checked_add(item.cost) {
        Some(total) if total <= stats.capacity => Admission::Admit,
        _ => Admission::Shed {
            reason: ShedReason::CapacityExhausted,
        },
    }
}

#[cfg(kani)]
mod verification {

    use super::*;

    #[kani::proof]
    fn admit_is_total_and_monotone_in_depth() {
        let capacity: u64 = kani::any();
        let deep: u64 = kani::any();
        let shallow: u64 = kani::any();
        kani::assume(shallow <= deep);
        let item = ItemMeta { cost: kani::any() };
        let deep_verdict = admit(
            &QueueStats {
                depth: deep,
                capacity,
            },
            &item,
        );
        let shallow_verdict = admit(
            &QueueStats {
                depth: shallow,
                capacity,
            },
            &item,
        );
        if deep_verdict == Admission::Admit {
            assert_eq!(shallow_verdict, Admission::Admit);
        }
    }

    #[kani::proof]
    fn an_overflowing_sum_never_admits() {
        let stats = QueueStats {
            depth: kani::any(),
            capacity: kani::any(),
        };
        let item = ItemMeta { cost: kani::any() };
        kani::assume(stats.depth.checked_add(item.cost).is_none());
        assert_eq!(
            admit(&stats, &item),
            Admission::Shed {
                reason: ShedReason::CapacityExhausted
            }
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
    use proptest::prelude::*;

    #[test]
    fn admits_up_to_capacity_and_sheds_past_it() {
        let stats = QueueStats {
            depth: 9,
            capacity: 10,
        };
        assert_eq!(admit(&stats, &ItemMeta { cost: 1 }), Admission::Admit);
        assert_eq!(
            admit(&stats, &ItemMeta { cost: 2 }),
            Admission::Shed {
                reason: ShedReason::CapacityExhausted
            }
        );
    }

    #[test]
    fn overflow_sheds_instead_of_wrapping() {
        let stats = QueueStats {
            depth: u64::MAX,
            capacity: u64::MAX,
        };
        assert_eq!(
            admit(&stats, &ItemMeta { cost: 1 }),
            Admission::Shed {
                reason: ShedReason::CapacityExhausted
            }
        );

        assert_eq!(admit(&stats, &ItemMeta { cost: 0 }), Admission::Admit);
    }

    proptest! {

        #[test]
        fn shallower_is_never_harder(capacity in any::<u64>(), deep in any::<u64>(),
                                     delta in any::<u64>(), cost in any::<u64>()) {
            let shallow = deep.saturating_sub(delta);
            let item = ItemMeta { cost };
            let deep_verdict = admit(&QueueStats { depth: deep, capacity }, &item);
            let shallow_verdict = admit(&QueueStats { depth: shallow, capacity }, &item);
            if deep_verdict == Admission::Admit {
                prop_assert_eq!(shallow_verdict, Admission::Admit);
            }
        }

        #[test]
        fn an_overflowing_sum_sheds(capacity in any::<u64>(), depth in any::<u64>(),
                                    cost in any::<u64>()) {
            if depth.checked_add(cost).is_none() {
                prop_assert_eq!(
                    admit(&QueueStats { depth, capacity }, &ItemMeta { cost }),
                    Admission::Shed { reason: ShedReason::CapacityExhausted }
                );
            }
        }
    }
}
