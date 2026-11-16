-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# workbench：分隔线拖不出一栏比两栏窄的工作台

规定 `client/src/core/workbench.ts` 的 `resized`、`widest` 与 `moved`，分隔线的键表在 `client/spec/Views/Workspace.lean` §7-11（APG Window Splitter）；栏宽存在浏览器里的理由是 client D24。TypeScript 是「怎样守住」的权威，本模型是「必须守住哪些性质」的权威；`workbench.test.ts` 判实现。

检阅档的工作台是三栏，共 `COLUMNS`（12）栏，每栏至少 `NARROWEST`（2）栏。一条分隔线只在它两侧的一对栏之间挪宽度：←／→ 是前一栏宽一栏或窄一栏，Home／End 是前一栏到最窄或最宽，Enter 复位到默认。这几个键都经 `resized` 落地，所以只要 `resized` 守住两条，每个键都守住：

1. **一对栏的总宽不变**（`resized_keeps_the_pair`），所以三栏的和仍是 12。
2. **两栏都不窄于 `NARROWEST`**（`resized_keeps_both_wide`）。

`moved` 换两栏的位置，不改任何一栏的宽（`moving_keeps_the_sum`）。

右侧编辑器与终端之间的分隔线是同一个形状，以行计、每区至少六行（§7-11）；`split` 带下限参数，两条分隔线是它的两次实例。
-/

namespace Client.Core.Workbench

/-- 外壳的栏数。 -/
def COLUMNS : Nat := 12

/-- 一栏至少几栏宽。 -/
def NARROWEST : Nat := 2

/-- 一对相邻区域按想要的宽度 `want` 重分：前一个钳在 `[floor, before + after - floor]` 之内，后一个取余下的。 -/
def split (floor before after want : Nat) : Nat × Nat :=
  (min (before + after - floor) (max floor want),
   before + after - min (before + after - floor) (max floor want))

/-- 工作台的 `resized`：下限是 `NARROWEST` 的 `split`。 -/
def resized (before after want : Nat) : Nat × Nat := split NARROWEST before after want

theorem split_keeps_the_pair (floor before after want : Nat) :
    (split floor before after want).1 + (split floor before after want).2 = before + after := by
  simp only [split]
  have : min (before + after - floor) (max floor want) ≤ before + after :=
    Nat.le_trans (Nat.min_le_left _ _) (Nat.sub_le _ _)
  omega

theorem split_keeps_both_wide (floor before after want : Nat) (room : 2 * floor ≤ before + after) :
    floor ≤ (split floor before after want).1 ∧ floor ≤ (split floor before after want).2 := by
  simp only [split]
  have upper : min (before + after - floor) (max floor want) ≤ before + after - floor :=
    Nat.min_le_left _ _
  have lower : floor ≤ min (before + after - floor) (max floor want) :=
    Nat.le_min.mpr ⟨by omega, Nat.le_max_left _ _⟩
  omega

theorem resized_keeps_the_pair (before after want : Nat) :
    (resized before after want).1 + (resized before after want).2 = before + after :=
  split_keeps_the_pair NARROWEST before after want

theorem resized_keeps_both_wide (before after want : Nat) (room : 4 ≤ before + after) :
    NARROWEST ≤ (resized before after want).1 ∧ NARROWEST ≤ (resized before after want).2 :=
  split_keeps_both_wide NARROWEST before after want room

/-- 三栏的宽，按从左到右的次序。 -/
abbrev Bench := Nat × Nat × Nat

/-- `moved`：相邻两栏换位（第几对由 `pair` 说：`0` 是左边一对，`1` 是右边一对）。 -/
def moved (bench : Bench) : Nat → Bench
  | 0 => (bench.2.1, bench.1, bench.2.2)
  | 1 => (bench.1, bench.2.2, bench.2.1)
  | _ => bench

/-- 三栏宽之和。 -/
def sum (bench : Bench) : Nat := bench.1 + bench.2.1 + bench.2.2

theorem moving_keeps_the_sum (bench : Bench) (pair : Nat) : sum (moved bench pair) = sum bench := by
  match pair with
  | 0 => simp only [moved, sum]; omega
  | 1 => simp only [moved, sum]; omega
  | _ + 2 => rfl

example : resized 3 5 0 = (2, 6) := by decide

example : resized 3 5 99 = (6, 2) := by decide

end Client.Core.Workbench
