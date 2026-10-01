-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# edit：一次保存改了什么、留下什么、怎样收回

规定 `crates/documents/src/edit.rs`（`documents::Transaction`、`Edit`、`Applied`）。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。

Rust 的 `Transaction` 是一串按文档序排好、互不重叠的半开字节区间加各自的替换字节。本模型把同一件事写成从头到尾的一串 `Piece`：两段编辑之间的空隙是 `keep n`，一段编辑是 `replace old new`（换掉源文接下来的 `old` 个字节）。两种写法一一对应：按序走 Rust 的编辑，空隙长 `start - cursor` 记一个 `keep`，编辑本身记一个 `replace (end - start) bytes`；末尾没被点名的字节在两边都原样留下。字节在模型里是 `Nat`，因为性质与字节的取值范围无关。

证明的性质：

1. **编辑之外的字节原样留下**（`apply_keep`、`apply_nil`）：`keep n` 抄的就是源文的前 `n` 个字节，没被点名的尾巴原样留下（A1 的模型一半）。
2. **撤销还原基线**（`undo_restores`）：一次保存交回的逆事务作用在新版本上，得到的就是原来那一版，逐字节（D9）。
3. **同一基线的第二次保存被拒**（`later_is_refused`）：带基线的接受只在源文的摘要等于基线时落；第一次保存改动了字节之后，第二个从同一基线出发的写者被拒（A3 的模型一半，D8）。摘要是参数 `digest`，它的单射性是假设 `inj`：这是 BLAKE3 的抗碰撞性，模型不证明它。
4. **基线对得上就落**（`first_is_accepted`）：守卫不是恒拒。

**拿掉基线守卫，第二个写者就盖掉第一个**（`withoutBaseline_overwrites`）：本模型咬得动的演示。
-/

namespace Documents.Edit

/-- 一个字节；取值范围与性质无关。 -/
abbrev Bytes := List Nat

/-- 从源文头上走过的一步：留下 `n` 个字节，或把接下来的 `old` 个字节换成 `new`。 -/
inductive Piece where
  | keep (n : Nat)
  | replace (old : Nat) (new : Bytes)

/-- 照 pieces 从左到右走源文，末尾没被点名的字节原样留下。 -/
def apply : List Piece → Bytes → Bytes
  | [], s => s
  | .keep n :: t, s => s.take n ++ apply t (s.drop n)
  | .replace old new :: t, s => new ++ apply t (s.drop old)

/-- 逆事务：在新版本的坐标里，把每段 `new` 换回源文原来那一段。 -/
def inverse : List Piece → Bytes → List Piece
  | [], _ => []
  | .keep n :: t, s => .keep n :: inverse t (s.drop n)
  | .replace old new :: t, s => .replace new.length (s.take old) :: inverse t (s.drop old)

/-- 每一步都落在源文之内：Rust 的 `E_INVALID_ARGS`（编辑越过末尾）排除的就是不满足它的事务。 -/
def fits : List Piece → Bytes → Prop
  | [], _ => True
  | .keep n :: t, s => n ≤ s.length ∧ fits t (s.drop n)
  | .replace old _ :: t, s => old ≤ s.length ∧ fits t (s.drop old)

theorem apply_nil (s : Bytes) : apply [] s = s := rfl

theorem apply_keep (n : Nat) (t : List Piece) (s : Bytes) :
    apply (.keep n :: t) s = s.take n ++ apply t (s.drop n) := rfl

theorem undo_restores (t : List Piece) (s : Bytes) (h : fits t s) :
    apply (inverse t s) (apply t s) = s := by
  induction t generalizing s with
  | nil => rfl
  | cons p t ih =>
    cases p with
    | keep n =>
      obtain ⟨hn, ht⟩ := h
      have hlen : (s.take n).length = n := by
        simp [List.length_take, Nat.min_eq_left hn]
      simp only [apply, inverse]
      rw [List.take_left' hlen, List.drop_left' hlen, ih (s.drop n) ht, List.take_append_drop]
    | replace old new =>
      obtain ⟨_, ht⟩ := h
      simp only [apply, inverse]
      rw [List.drop_left, ih (s.drop old) ht, List.take_append_drop]

/-- 带基线的接受：源文的摘要等于基线才落，否则什么也不落。 -/
def accept {α : Type} [DecidableEq α] (digest : Bytes → α) (baseline : α)
    (t : List Piece) (s : Bytes) : Option Bytes :=
  if digest s = baseline then some (apply t s) else none

theorem first_is_accepted {α : Type} [DecidableEq α] (digest : Bytes → α)
    (t : List Piece) (s : Bytes) : accept digest (digest s) t s = some (apply t s) := by
  simp [accept]

theorem later_is_refused {α : Type} [DecidableEq α] (digest : Bytes → α)
    (inj : ∀ a b, digest a = digest b → a = b)
    (s : Bytes) (t₁ t₂ : List Piece) (moved : apply t₁ s ≠ s) :
    accept digest (digest s) t₂ (apply t₁ s) = none := by
  unfold accept
  split
  · rename_i same
    exact absurd (inj _ _ same) moved
  · rfl

/-!
## 咬得动的演示

不看基线（每次都落），第二个从同一版本出发的写者把第一个写者的改动盖掉：第一个写者把 `[1]` 换成 `[2]`，第二个把同一个字节换成 `[3]`，最后留下的是 `[3]`，`[2]` 不见了。
-/

def acceptWithoutBaseline (t : List Piece) (s : Bytes) : Option Bytes :=
  some (apply t s)

theorem withoutBaseline_overwrites :
    acceptWithoutBaseline [.replace 1 [3]] (apply [.replace 1 [2]] [1]) = some [3] := by
  decide

end Documents.Edit
