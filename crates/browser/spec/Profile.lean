-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# profile：一栋楼的登录态住在哪里，属于谁

规定 `crates/browser/src/profile.rs`（`browser::profile`，`Profile::of`）。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。

一栋楼的 profile 住在城的保留区下：`<reserved>/browser-profiles/<楼名>`（`PROFILES_DIR`）。`Profile::of` 只收楼的 `Address`：保留区本身不是楼，房间也不是楼（地址里带 `/` 即拒），因为登录态属于项目而不属于某个房间。confidential 楼的拒绝只有一个家：`city::policy::evaluate` 读到 `confidential = true` 与 `browser = true` 或 `usersbrowser` 并存即 `E_CONFIG_INVALID`，本 crate 没有第二个拒绝它的臂。

路径按段写成一个列表。两条性质：

1. **两栋楼的 profile 互不包含**（`profiles_apart`）：楼名不同，一个 profile 不是另一个的前缀，于是一栋楼的浏览器读不到另一栋楼的 cookie。
2. **profile 住在保留区里**（`profile_reserved`）：它不在任何楼的写域内。

**若房间也能拿 profile，一栋楼的 profile 就包含它房间的**（`rooms_would_nest`）：本模型咬得动的演示，说明「地址里带 `/` 即拒」为什么要在。
-/

namespace Browser.Profile

/-- 一个地址按段写开。 -/
abbrev Path := List String

def reserved : String := ".sprawling"
def profilesDir : String := "browser-profiles"

/-- 一栋楼只有一段；`none` 就是 `E_INVALID_ARGS`。 -/
def of (building : Path) : Option Path :=
  match building with
  | [name] => if name = reserved then none else some [reserved, profilesDir, name]
  | _ => none

theorem of_shape {b p : Path} (h : of b = some p) :
    ∃ name, b = [name] ∧ p = [reserved, profilesDir, name] := by
  match b, h with
  | [name], h =>
    refine ⟨name, rfl, ?_⟩
    by_cases r : name = reserved
    · simp [of, r] at h
    · simp only [of, r, ite_false, Option.some.injEq] at h
      exact h.symm
  | [], h => simp [of] at h
  | _ :: _ :: _, h => simp [of] at h

theorem profile_reserved {b p : Path} (h : of b = some p) : p.head? = some reserved := by
  obtain ⟨_, _, rfl⟩ := of_shape h
  rfl

theorem profiles_apart {b₁ b₂ p₁ p₂ : Path} (h₁ : of b₁ = some p₁) (h₂ : of b₂ = some p₂)
    (apart : b₁ ≠ b₂) : ¬ p₁ <+: p₂ := by
  obtain ⟨n₁, rfl, rfl⟩ := of_shape h₁
  obtain ⟨n₂, rfl, rfl⟩ := of_shape h₂
  intro pre
  have same := List.IsPrefix.eq_of_length pre (by simp)
  simp only [List.cons.injEq, and_true, true_and] at same
  exact apart (by rw [same])

/-!
## 咬得动的演示

若 `of` 也收房间（带第二段的地址），楼 `acme` 的 profile 就是房间 `acme/lab` 的 profile 的前缀：一栋楼的浏览器读得到它房间的登录态，隔离不再按楼成立。
-/

def ofWithoutRooms (address : Path) : Path := [reserved, profilesDir] ++ address

theorem rooms_would_nest : ofWithoutRooms ["acme"] <+: ofWithoutRooms ["acme", "lab"] :=
  ⟨["lab"], rfl⟩

end Browser.Profile
