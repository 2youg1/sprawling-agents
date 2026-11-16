-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# snapshot：模型看得见一页的什么

规定 `crates/browser/src/snapshot.rs`（`browser::snapshot`，`PageSnapshot::read` 与 `truncate_label`）。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。

一次快照把驱动的无障碍树压平成 `ref role "name"` 的行：三个字段一行，因为模型下一件要做的事是把 ref 抄回来。原始 DOM 恒不入窗；角色取自一张封闭的白名单（入口 §10 第 3 条）。

label 是页面给的名字，页面是别人写的。这里证明它进窗之前的两件事：

1. **label 里没有控制字符**（`label_has_no_control`）：换行是控制字符，所以一个 label 恒在一行之内，模型读到的行数就是节点数。
2. **label 有上界**（`label_bounded`）：至多 `LABEL_MAX` 个字符再加一个省略号。

哪些字符算控制字符是参数 `control`，空白怎么压是参数 `squeeze`：前者是 Unicode 的事实，后者是 Rust 的 `split_whitespace`；模型只要求 `squeeze` 不造出新字符（它只能留下已有的字符或空格），以及空格与省略号不是控制字符。

**拿掉「控制字符换成空格」那一步，一个换行就进了窗**（`withoutCleaning_keeps_a_newline`）：本模型咬得动的演示。
-/

namespace Browser.Snapshot

def LABEL_MAX : Nat := 120

/-- 超长则截到 `LABEL_MAX` 再加省略号。 -/
def cut (tidy : List Char) : List Char :=
  if tidy.length ≤ LABEL_MAX then tidy else tidy.take LABEL_MAX ++ ['…']

/-- `truncate_label`：控制字符换成空格，压空白，再截。 -/
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
    (raw : List Char) : (label control squeeze raw).length ≤ LABEL_MAX + 1 := by
  unfold label cut
  split
  · omega
  · simp only [List.length_append, List.length_take, List.length_singleton]
    omega

/-!
## 咬得动的演示

不把控制字符换成空格（这里 `squeeze` 取恒等），`label_has_no_control` 不再成立：一个换行原样留在 label 里。
-/

def labelWithoutCleaning (raw : List Char) : List Char :=
  cut raw

theorem withoutCleaning_keeps_a_newline : '\n' ∈ labelWithoutCleaning ['a', '\n', 'b'] := by
  decide

end Browser.Snapshot
