-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# target：一个名字恰好指一扇窗口，否则拒绝

规定 `crates/desktop/src/platform/windows/target.rs`（`desktop::platform::windows::target`，`choose`）。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。

一次调用以 `title`、`process` 或两者指名一扇窗口；两者合起来是一个判定 `fits`（给了的每一个都要中，`spec/Scope.lean` 的读法）。桌面上中了的窗口恰好一扇才交出它；零扇拒并指向 `desktop.windows`，两扇以上拒并列出各自的 title，**恒不**在其中挑一个——挑中的那一扇是对人想要哪份文档的猜测。

三条性质：

1. **没指名就拒**（`naming_nothing_is_refused`）。
2. **交出的恰是唯一中了的那一扇**（`chosen_is_the_only_fit`）。
3. **两扇以上中了恒不挑**（`several_fits_are_never_a_pick`）。

**拿掉「恰好一扇」，两扇同名窗口里排在前面的那扇被挑中**（`withoutUniqueness_picks_the_first_of_two`）：本模型咬得动的演示。
-/

namespace Desktop.Platform.Windows.Target

inductive Refusal where
  | unnamed
  | noFit
  | several (count : Nat)
  deriving DecidableEq, Repr

variable {α : Type}

/-- 中了的窗口在桌面列表里的下标，从 `index` 数起。 -/
def hits (fits : α → Bool) : List α → Nat → List Nat
  | [], _ => []
  | w :: ws, index => (if fits w then [index] else []) ++ hits fits ws (index + 1)

/-- `choose`：`asked` 为 `none` 是没指名。 -/
def choose (seen : List α) (asked : Option (α → Bool)) : Except Refusal Nat :=
  match asked with
  | none => .error .unnamed
  | some fits =>
    match hits fits seen 0 with
    | [only] => .ok only
    | [] => .error .noFit
    | several => .error (.several several.length)

theorem naming_nothing_is_refused (seen : List α) : choose seen none = .error .unnamed := rfl

theorem chosen_is_the_only_fit {seen : List α} {fits : α → Bool} {index : Nat}
    (h : choose seen (some fits) = .ok index) : hits fits seen 0 = [index] := by
  simp only [choose] at h
  generalize hits fits seen 0 = found at h ⊢
  rcases found with _ | ⟨a, _ | ⟨b, rest⟩⟩
  · simp at h
  · simp only [Except.ok.injEq] at h
    rw [h]
  · simp at h

theorem several_fits_are_never_a_pick {seen : List α} {fits : α → Bool}
    (two : 2 ≤ (hits fits seen 0).length) :
    choose seen (some fits) = .error (.several (hits fits seen 0).length) := by
  simp only [choose]
  generalize hits fits seen 0 = found at two ⊢
  rcases found with _ | ⟨a, _ | ⟨b, rest⟩⟩
  · simp at two
  · simp at two
  · rfl

/-!
## 咬得动的演示

拿掉「恰好一扇」，`several_fits_are_never_a_pick` 不再成立：两扇标题相同的窗口（这里标题写成同一个数 7），排在前面的那一扇被挑中。
-/

def chooseFirst (seen : List α) (asked : Option (α → Bool)) : Except Refusal Nat :=
  match asked with
  | none => .error .unnamed
  | some fits =>
    match hits fits seen 0 with
    | [] => .error .noFit
    | first :: _ => .ok first

theorem withoutUniqueness_picks_the_first_of_two :
    chooseFirst [7, 7] (some fun title => title == 7) = .ok 0 := by
  rfl

end Desktop.Platform.Windows.Target
