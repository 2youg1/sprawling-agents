-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# outline：一扇窗口读给模型的名字

规定 `crates/desktop/src/outline.rs`（`desktop::outline`，`label` 与 `LABEL_MOST`）。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威（D10 (b)）。

一扇窗口的树折成一段大纲，一行一个元素：`e<n> <role> "<name>"`。name 是应用给控件起的名字，应用是别人写的。这里证明它进窗之前的两件事：

1. **名字里没有控制字符**（`label_has_no_control`）：换行是控制字符，所以一个名字恒在一行之内，模型读到的行数就是元素数。
2. **名字有上界**（`label_bounded`）：至多 `LABEL_MOST` 个字符再加一个省略号。

哪些字符算控制字符是参数 `control`（Rust 的 `char::is_control`），空白怎么压是参数 `squeeze`（Rust 的 `split_whitespace` 再以空格接起）：模型只要求 `squeeze` 不造出新字符（只留下已有字符或空格），以及空格与省略号不是控制字符。

**拿掉「控制字符换成空格」那一步，一个换行就进了窗**（`withoutCleaning_keeps_a_newline`）：本模型咬得动的演示。
-/

namespace Desktop.Outline

/-- `LABEL_MOST`：一个名字最多的字符数（§14）。 -/
def LABEL_MOST : Nat := 80

/-- 超长则截到 `LABEL_MOST` 再加省略号。 -/
def cut (tidy : List Char) : List Char :=
  if tidy.length ≤ LABEL_MOST then tidy else tidy.take LABEL_MOST ++ ['…']

/-- `label`：控制字符换成空格，压空白，再截。 -/
def label (control : Char → Bool) (squeeze : List Char → List Char) (raw : List Char) :
    List Char :=
  cut (squeeze (raw.map fun c => if control c then ' ' else c))

theorem cut_keeps (tidy : List Char) : ∀ c ∈ cut tidy, c ∈ tidy ∨ c = '…' := by
  intro c hc
  unfold cut at hc
  split at hc
  · exact Or.inl hc
  · simp only [List.mem_append, List.mem_singleton] at hc
    rcases hc with taken | dots
    · exact Or.inl (List.mem_of_mem_take taken)
    · exact Or.inr dots

theorem label_has_no_control (control : Char → Bool) (squeeze : List Char → List Char)
    (keeps : ∀ l, ∀ c ∈ squeeze l, c ∈ l ∨ c = ' ')
    (space : control ' ' = false) (ellipsis : control '…' = false)
    (raw : List Char) : ∀ c ∈ label control squeeze raw, control c = false := by
  intro c hc
  rcases cut_keeps _ c hc with tidy | dots
  · rcases keeps _ c tidy with mem | sp
    · simp only [List.mem_map] at mem
      obtain ⟨d, _, rfl⟩ := mem
      by_cases hd : control d = true
      · simp [hd, space]
      · simp [hd]
    · rw [sp]; exact space
  · rw [dots]; exact ellipsis

theorem label_bounded (control : Char → Bool) (squeeze : List Char → List Char)
    (raw : List Char) : (label control squeeze raw).length ≤ LABEL_MOST + 1 := by
  unfold label cut
  split
  · omega
  · simp only [List.length_append, List.length_take, List.length_singleton]
    omega

/-!
## 咬得动的演示

不把控制字符换成空格（这里 `squeeze` 取恒等），`label_has_no_control` 不再成立：一个换行原样留在名字里，大纲多出一行不是元素的行。
-/

def labelWithoutCleaning (raw : List Char) : List Char :=
  cut raw

theorem withoutCleaning_keeps_a_newline : '\n' ∈ labelWithoutCleaning ['a', '\n', 'b'] := by
  decide

end Desktop.Outline
