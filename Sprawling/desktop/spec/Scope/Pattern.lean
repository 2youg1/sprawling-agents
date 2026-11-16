-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# scope::pattern：一行 allowlist 匹配什么

规定 `crates/desktop/src/scope/pattern.rs`（`desktop::scope::pattern`，`Pattern::new` 与 `Pattern::matches`）。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。

pattern 是给人写的：只认 `*`（任意一段字符，含空），其余每个字符代表它自己，按 ASCII 大小写不敏感比较，因为 Windows 的进程名就是这样比的（§10 第 4 条）。一整套正则会让「我到底放开了什么」变成一个需要推演的问题。

下面的 `fits` 是参考语义：按 pattern 结构递归，所以它对每个输入都终止，这就是「glob 一定终止」在模型里的证明。Rust 用的是回溯指针的迭代写法（文本下标只增不减，到达文本末尾即失败，§10 第 6 条）；两者一致由 `pattern.rs` 旁的测试按例比对，模型的证明不是 Rust 的证明。

三条性质：

1. **`*` 匹配任何名字，包括空名字**（`a_star_matches_any_name`）。
2. **不带 `*` 的 pattern 只匹配与它等长的名字**（`a_pattern_without_a_star_matches_only_its_own_length`）：`Calculator` 不匹配 `Calculator Plus`，也不匹配 `The Calculator`。
3. **大小写按 ASCII 不计**（`case_is_not_counted`）。

**拿掉「文本要读完」，`Calculator` 也匹配 `Calculator Plus`**（`withoutTheEnd_matches_a_longer_name`）：本模型咬得动的演示。
-/

namespace Desktop.Scope.Pattern

/-- pattern 里的一个位置：`*` 或一个字面字符。 -/
inductive Glob where
  | star
  | lit (c : Char)
  deriving DecidableEq, Repr

/-- `Pattern::new`：`*` 是星号，其余是字面字符。 -/
def read (line : List Char) : List Glob :=
  line.map fun c => if c = '*' then .star else .lit c

/-- 一段文字的每一个后缀，从整段到空。 -/
def suffixes : List Char → List (List Char)
  | [] => [[]]
  | c :: rest => (c :: rest) :: suffixes rest

/-- 参考语义：按 pattern 结构递归。`*` 吞掉文字的一个前缀，剩下的某个后缀要被余下的 pattern 读完。 -/
def fits : List Glob → List Char → Bool
  | [], text => text.isEmpty
  | .star :: rest, text => (suffixes text).any fun suffix => fits rest suffix
  | .lit _ :: _, [] => false
  | .lit c :: rest, d :: text => c.toLower == d.toLower && fits rest text

theorem some_suffix_is_empty (text : List Char) :
    (suffixes text).any (fun s => s.isEmpty) = true := by
  induction text with
  | nil => rfl
  | cons c rest ih => simp [suffixes, ih]

theorem a_star_matches_any_name (text : List Char) : fits [.star] text = true := by
  simp only [fits]
  exact some_suffix_is_empty text

theorem a_pattern_without_a_star_matches_only_its_own_length (line text : List Char)
    (h : fits (line.map .lit) text = true) : text.length = line.length := by
  induction line generalizing text with
  | nil =>
    cases text with
    | nil => rfl
    | cons d rest => simp [fits] at h
  | cons c rest ih =>
    cases text with
    | nil => simp [fits] at h
    | cons d more =>
      simp only [List.map_cons, fits, Bool.and_eq_true] at h
      simp [ih more h.2]

/-!
## 咬得动的演示

模式读完时不要求文本也读完，`a_pattern_without_a_star_matches_only_its_own_length` 不再成立：允许 `Calculator` 的一行，也允许了 `Calculator Plus`。
-/

def fitsWithoutTheEnd : List Glob → List Char → Bool
  | [], _ => true
  | .star :: rest, text => (suffixes text).any fun suffix => fitsWithoutTheEnd rest suffix
  | .lit _ :: _, [] => false
  | .lit c :: rest, d :: text => c.toLower == d.toLower && fitsWithoutTheEnd rest text

theorem withoutTheEnd_matches_a_longer_name :
    fitsWithoutTheEnd (read "Calculator".toList) "Calculator Plus".toList = true := by
  decide

end Desktop.Scope.Pattern
