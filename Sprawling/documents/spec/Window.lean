-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# window：一次答复带多少、在哪里切

规定 `crates/documents/src/window.rs`（`documents::cut`、`lift`、`head`，`WINDOW_BYTES_MAX`）。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。

一个窗口是一个版本里的一段半开字节区间。哪些偏移是字符边界是参数 `boundary`：它由编码决定（UTF-8 的非续字节、UTF-16 的偶数偏移且不是低代理），版本的两端恒是边界。Rust 的切法是：起点取 `min(wanted.start, size)` 往回退到边界，终点取 `boundedEnd` 往回退到边界，再不小于起点。往回退不低于抬起的第一个字节 `floor`（`Lifted.at`）。

证明的性质：

1. **往回退只会变短**（`stepBack_le`），**不低于 floor**（`floor_le_stepBack`）。
2. **退到的地方是边界，或已经退到 floor**（`stepBack_boundary`）：窗口不劈开字符，除非抬起的字节里没有边界，那时解码拒绝（Rust 的 `E_INVALID_ARGS`）。
3. **窗口有界**（`window_end_le_size`、`window_end_le_bound`）：终点不过版本末尾，也不过请求起点之后 `WINDOW_BYTES_MAX` 字节。`head` 的终点是一个不超过 `WINDOW_BYTES_MAX` 的块末尾或这个界本身，所以它同样有界。
4. **有边界就有进展**（`stepBack_reaches`）：起点与界之间只要有一个边界，终点就不早于它，所以逐窗读完整个版本不会停在原地。

**不往回退、直接切在界上，窗口就劈开一个字符**（`withoutStepBack_splits`）：本模型咬得动的演示。
-/

namespace Documents.Window

def WINDOW_BYTES_MAX : Nat := 65536

/-- 从 `a` 往回找第一个边界；到了 `floor` 或更低就停。 -/
def stepBack (boundary : Nat → Bool) (floor : Nat) : Nat → Nat
  | 0 => 0
  | k + 1 => if k + 1 ≤ floor then k + 1
    else if boundary (k + 1) then k + 1
    else stepBack boundary floor k

/-- 一个从 `start` 起、到 `wantedEnd` 止的请求在长 `size` 的版本里最多能到哪。 -/
def boundedEnd (start wantedEnd size : Nat) : Nat :=
  max (min start size) (min (min wantedEnd size) (min start size + WINDOW_BYTES_MAX))

theorem stepBack_le (b : Nat → Bool) (floor a : Nat) : stepBack b floor a ≤ a := by
  induction a with
  | zero => simp [stepBack]
  | succ k ih =>
    simp only [stepBack]
    split
    · exact Nat.le_refl _
    · split
      · exact Nat.le_refl _
      · omega

theorem floor_le_stepBack (b : Nat → Bool) (floor a : Nat) (h : floor ≤ a) :
    floor ≤ stepBack b floor a := by
  induction a with
  | zero => exact h
  | succ k ih =>
    simp only [stepBack]
    split
    · omega
    · split
      · omega
      · exact ih (by omega)

theorem stepBack_boundary (b : Nat → Bool) (floor a : Nat) :
    b (stepBack b floor a) = true ∨ stepBack b floor a ≤ floor := by
  induction a with
  | zero => right; simp [stepBack]
  | succ k ih =>
    simp only [stepBack]
    split
    · right; assumption
    · split
      · left; assumption
      · exact ih

theorem stepBack_reaches (b : Nat → Bool) (floor a k : Nat)
    (hk : b k = true) (below : k ≤ a) : k ≤ stepBack b floor a := by
  induction a with
  | zero => omega
  | succ j ih =>
    simp only [stepBack]
    split
    · omega
    · split
      · omega
      · rename_i _ notBoundary
        by_cases same : k = j + 1
        · rw [same] at hk; simp [hk] at notBoundary
        · exact ih (by omega)

theorem boundedEnd_le_size (start wantedEnd size : Nat) :
    boundedEnd start wantedEnd size ≤ size := by
  unfold boundedEnd; omega

theorem boundedEnd_le_bound (start wantedEnd size : Nat) :
    boundedEnd start wantedEnd size ≤ start + WINDOW_BYTES_MAX := by
  unfold boundedEnd; omega

theorem window_end_le_size (b : Nat → Bool) (floor start wantedEnd size : Nat) :
    stepBack b floor (boundedEnd start wantedEnd size) ≤ size :=
  Nat.le_trans (stepBack_le _ _ _) (boundedEnd_le_size _ _ _)

theorem window_end_le_bound (b : Nat → Bool) (floor start wantedEnd size : Nat) :
    stepBack b floor (boundedEnd start wantedEnd size) ≤ start + WINDOW_BYTES_MAX :=
  Nat.le_trans (stepBack_le _ _ _) (boundedEnd_le_bound _ _ _)

/-!
## 咬得动的演示

一个三字节的字符占着偏移 1、2、3，所以 2 与 3 不是边界。请求切在 3：不往回退就切在 3，那里不是边界，`stepBack_boundary` 的结论不成立。
-/

def threeByteCharacter (i : Nat) : Bool := i != 2 && i != 3

def cutWithoutStepBack (_ : Nat → Bool) (_ : Nat) (a : Nat) : Nat := a

theorem withoutStepBack_splits :
    ¬ (threeByteCharacter (cutWithoutStepBack threeByteCharacter 0 3) = true ∨
       cutWithoutStepBack threeByteCharacter 0 3 ≤ 0) := by
  decide

end Documents.Window
