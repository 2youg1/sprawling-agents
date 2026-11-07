-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

namespace Benchstat.Estimators

/-- One-based nearest-rank index after the caller has computed `ceil(q*n)`.
The minimum also handles a quantile above one without an out-of-range read. -/
def nearestRankIndex (n ceiling : Nat) : Nat :=
  min n (max 1 ceiling)

/-- Automatic block lengths are accepted only inside the observed series. -/
def boundedBlockLength (n selected : Nat) : Nat :=
  min n (max 1 selected)

/-- A degenerate pilot asks for one more complete cohort, never for zero samples. -/
def nextPilotSize (n cohort : Nat) : Nat :=
  n + cohort

theorem nearest_rank_is_positive (n ceiling : Nat) (hn : 0 < n) :
    0 < nearestRankIndex n ceiling := by
  unfold nearestRankIndex
  exact Nat.lt_of_lt_of_le (Nat.le_max_left 1 ceiling) (Nat.min_le_left _ _)

theorem nearest_rank_does_not_exceed_the_sample (n ceiling : Nat) :
    nearestRankIndex n ceiling ≤ n := by
  exact Nat.min_le_left _ _

theorem block_length_is_positive (n selected : Nat) (hn : 0 < n) :
    0 < boundedBlockLength n selected := by
  unfold boundedBlockLength
  exact Nat.lt_of_lt_of_le (Nat.le_max_left 1 selected) (Nat.min_le_left _ _)

theorem block_length_does_not_exceed_the_sample (n selected : Nat) :
    boundedBlockLength n selected ≤ n := by
  exact Nat.min_le_left _ _

theorem a_degenerate_pilot_grows (n cohort : Nat) (hc : 0 < cohort) :
    n < nextPilotSize n cohort := by
  unfold nextPilotSize
  exact Nat.lt_add_of_pos_right hc

/-- The normal path is realizable: two observations can use rank one and one fresh cohort grows 100 to 125. -/
example : nearestRankIndex 2 1 = 1 ∧ nextPilotSize 100 25 = 125 := by
  decide

end Benchstat.Estimators
