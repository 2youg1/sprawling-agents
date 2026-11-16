-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# views：一个动作只落在它看过的那一代上

规定 `crates/desktop/src/platform/windows/views.rs`（`desktop::platform::windows::views`，`Views::mint` 与 `Views::resolve`）。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威（D5，§10 设计六）。

一次 `desktop.snapshot` 为一扇窗口（按句柄，不按标题）铸一代 ref，记下这扇窗口当时的外框；`desktop.act` 带着它看过的那一代来。代与外框都对得上、ref 是这一代铸过的，动作才被接受。

五条性质：

1. **快照推进这扇窗口的代**（`a_snapshot_moves_the_window_on`）。
2. **旧的一代被拒**（`an_older_generation_is_refused`）：对着一份快照做的决定，恒不落到另一份快照上。
3. **挪动或改过尺寸的窗口被拒**（`a_moved_window_is_refused`）：ref 的屏幕坐标指向挪动前的位置。
4. **给一扇窗口拍快照不动别的窗口**（`another_window_keeps_its_generation`）：代按句柄记，两扇同名窗口不共用一份记录。
5. **被接受的动作落在看过的那一代上**（`acted_on_what_was_seen`）。

剩余限制（不是定理，写在入口 D5）：窗口内部重排而外框不动时旧的一代仍被接受；句柄被复用而外框恰好相同时也看不出来。

**拿掉外框的比对，一扇挪动过的窗口照样收下动作**（`withoutBounds_acts_on_a_moved_window`）：本模型咬得动的演示。
-/

namespace Desktop.Platform.Windows.Views

/-- 一扇窗口被看过的那一代：代号、外框、铸了几个 ref（`e1` 到 `e<refs>`）。外框的类型是参数：模型只比较它相等与否。 -/
structure Seen (β : Type) where
  generation : Nat
  bounds : β
  refs : Nat

/-- 每扇窗口（按句柄）现在是哪一代。 -/
abbrev Views (β : Type) := Nat → Option (Seen β)

inductive Refusal where
  | unseen
  | stale
  | moved
  | unknownRef
  deriving DecidableEq, Repr

variable {β : Type}

/-- 这扇窗口下一代的代号。 -/
def next (v : Views β) (aim : Nat) : Nat :=
  match v aim with
  | none => 1
  | some s => s.generation + 1

/-- `mint`：一次快照为 `aim` 这扇窗口铸新的一代。 -/
def mint (v : Views β) (aim : Nat) (bounds : β) (refs : Nat) : Views β :=
  fun a => if a = aim then some ⟨next v aim, bounds, refs⟩ else v a

/-- `resolve`：一个对着第 `generation` 代、此刻外框为 `bounds` 的窗口做的动作，点的是哪个 ref。 -/
def resolve [DecidableEq β] (v : Views β) (aim : Nat) (bounds : β) (generation ref : Nat) :
    Except Refusal Nat :=
  match v aim with
  | none => .error .unseen
  | some s =>
    if generation ≠ s.generation then .error .stale
    else if bounds ≠ s.bounds then .error .moved
    else if ref = 0 ∨ ref > s.refs then .error .unknownRef
    else .ok ref

theorem a_snapshot_moves_the_window_on (v : Views β) (aim : Nat) (old : Seen β)
    (seen : v aim = some old) (bounds : β) (refs : Nat) :
    ∃ s, mint v aim bounds refs aim = some s ∧ s.generation = old.generation + 1 := by
  refine ⟨⟨old.generation + 1, bounds, refs⟩, ?_, rfl⟩
  simp [mint, next, seen]

theorem an_older_generation_is_refused [DecidableEq β] (v : Views β) (aim : Nat) (old : Seen β)
    (seen : v aim = some old) (bounds : β) (refs ref : Nat) :
    resolve (mint v aim bounds refs) aim bounds old.generation ref = .error .stale := by
  simp [resolve, mint, next, seen]

theorem a_moved_window_is_refused [DecidableEq β] (v : Views β) (aim : Nat) (s : Seen β)
    (seen : v aim = some s) (now : β) (moved : now ≠ s.bounds) (ref : Nat) :
    resolve v aim now s.generation ref = .error .moved := by
  simp [resolve, seen, moved]

theorem another_window_keeps_its_generation (v : Views β) (aim other : Nat)
    (apart : other ≠ aim) (bounds : β) (refs : Nat) :
    mint v aim bounds refs other = v other := by
  simp [mint, apart]

theorem acted_on_what_was_seen [DecidableEq β] {v : Views β} {aim : Nat} {bounds : β} {generation ref r : Nat}
    (h : resolve v aim bounds generation ref = .ok r) :
    ∃ s, v aim = some s ∧ generation = s.generation ∧ bounds = s.bounds ∧
      1 ≤ r ∧ r ≤ s.refs := by
  unfold resolve at h
  split at h
  · simp at h
  · next s seen =>
    by_cases stale : generation ≠ s.generation
    · simp [stale] at h
    · by_cases moved : bounds ≠ s.bounds
      · simp [stale, moved] at h
      · by_cases unknown : ref = 0 ∨ ref > s.refs
        · simp [stale, moved, unknown] at h
        · simp only [stale, moved, unknown, ite_false, Except.ok.injEq] at h
          subst h
          exact ⟨s, seen, by simpa using stale, by simpa using moved, by omega, by omega⟩

/-!
## 咬得动的演示

拿掉外框的比对，`a_moved_window_is_refused` 不再成立：第 1 代看过的窗口从 0 挪到 500 之后，对它 `e1` 的点击照样被接受，落在挪动前的坐标上。
-/

def resolveWithoutBounds (v : Views β) (aim : Nat) (_bounds : β) (generation ref : Nat) :
    Except Refusal Nat :=
  match v aim with
  | none => .error .unseen
  | some s =>
    if generation ≠ s.generation then .error .stale
    else if ref = 0 ∨ ref > s.refs then .error .unknownRef
    else .ok ref

theorem withoutBounds_acts_on_a_moved_window :
    resolveWithoutBounds (mint (fun _ => none) 7 (0 : Nat) 3) 7 500 1 1 = .ok 1 := by
  rfl

end Desktop.Platform.Windows.Views
