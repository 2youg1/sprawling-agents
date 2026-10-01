-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# markdown：一个文法把一个版本的窗口读成块

规定 `crates/documents/src/markdown.rs`（`documents::preview`）与它的两个部分 `markdown/tree.rs`、`markdown/lowering.rs`。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。CommonMark 与 GFM 的文法本身归 comrak（D20、D22），不在这里重写：模型只管本 crate 在 comrak 的树之外加的三条判定和一条答复的形状。

证明的性质：

1. **链接只指向四类去处**（`admitted`、`other_refused`、`listed_admitted`）：相对地址、`http`、`https`、`mailto`；其余的协议（`javascript:`、`data:`、`file:`）不成为链接，它的文字留下（D23）。
2. **嵌套有上限**（`wraps_lower_le`）：读出的树套不过 `NESTING_MAX` 层，再深的一层整个成为「不支持」并带着原文；**上限之内一层不丢**（`lower_fits`）（D24）。
3. **预览窗口止于块末，并且总有进展**（`settle_le_cut`、`settle_progress`、`settle_at_end`）：窗口不过 `cut` 切出的终点；起点之后只要还有字节，终点就在起点之后，所以逐窗读完整个版本不会停在原地；到了版本末尾的窗口不被截短（D25）。
4. **「不支持」与「内容为空」是两种答复**（`empty_is_not_unsupported`）：一个空窗口答零个块，一份本文法不读的编码答「不支持」，两者不相等（A4，D25）。

咬得动的演示：不判协议就放过 `javascript:`（`withoutCheck_admits_other`）；不设上限，十七层的引用读出十七层（`withoutCap_exceeds`）；不要求末尾在起点之后，窗口停在原地（`withoutGuard_stalls`）。
-/

namespace Documents.Markdown

/-! ## 链接的去处（D23）

Rust 先按 WHATWG URL 的读法去掉制表符与换行、去掉开头的控制字符与空格，再取第一个 `:` 之前、合乎 RFC 3986 `scheme` 文法的那一段；没有这一段的是相对地址。本模型从分好的类开始：怎样从字符串分出类是 Rust 的 `scheme_of`，由 `markdown::tests` 的语料判。 -/

/-- 一个链接目标按协议分成的类。 -/
inductive Scheme where
  | relative
  | http
  | https
  | mailto
  | other
  deriving DecidableEq, Repr

/-- 链接成为链接的条件；不成立时它的文字作为文字留下。 -/
def admitted : Scheme → Bool
  | .relative | .http | .https | .mailto => true
  | .other => false

theorem other_refused : admitted .other = false := rfl

theorem listed_admitted (s : Scheme) (listed : s ≠ .other) : admitted s = true := by
  cases s <;> simp_all [admitted]

/-- 不判协议的读法：每一个目标都成为链接。 -/
def admittedWithoutCheck : Scheme → Bool := fun _ => true

theorem withoutCheck_admits_other : admittedWithoutCheck .other ≠ admitted .other := by
  decide

/-! ## 嵌套的上限（D24）

一条嵌套链是一串容器（引用、列表项、强调、链接）包着最里面的内容；一棵树的深度是它最深的那条链。`lower room n` 是 Rust 的 `lowering` 在还剩 `room` 层时读 `n`：没有余地时整个成为 `cut`（Rust 的 `Unsupported { construct: Nesting, .. }`，带着原文），否则照原样读出一层、余地减一。 -/

def NESTING_MAX : Nat := 16

inductive Nest where
  | leaf
  | wrap (inner : Nest)

inductive Laid where
  | leaf
  | wrap (inner : Laid)
  | cut

def lower : Nat → Nest → Laid
  | 0, _ => .cut
  | _ + 1, .leaf => .leaf
  | k + 1, .wrap inner => .wrap (lower k inner)

/-- 读出的树套了几层。 -/
def wraps : Laid → Nat
  | .leaf => 0
  | .cut => 0
  | .wrap inner => wraps inner + 1

/-- 源文里的链套了几层。 -/
def depth : Nest → Nat
  | .leaf => 0
  | .wrap inner => depth inner + 1

/-- 照原样读出，不设上限。 -/
def embed : Nest → Laid
  | .leaf => .leaf
  | .wrap inner => .wrap (embed inner)

theorem wraps_lower_le (k : Nat) (n : Nest) : wraps (lower k n) ≤ k := by
  induction k generalizing n with
  | zero => simp [lower, wraps]
  | succ k ih =>
    cases n with
    | leaf => simp [lower, wraps]
    | wrap inner =>
      simp only [lower, wraps]
      have := ih inner
      omega

theorem lower_fits (n : Nest) (k : Nat) (fits : depth n < k) : lower k n = embed n := by
  induction n generalizing k with
  | leaf =>
    cases k with
    | zero => simp [depth] at fits
    | succ k => simp [lower, embed]
  | wrap inner ih =>
    cases k with
    | zero => simp [depth] at fits
    | succ k =>
      simp only [lower, embed]
      rw [ih k (by simp [depth] at fits; omega)]

def deep : Nat → Nest
  | 0 => .leaf
  | k + 1 => .wrap (deep k)

theorem wraps_embed_deep (k : Nat) : wraps (embed (deep k)) = k := by
  induction k with
  | zero => rfl
  | succ k ih => simp [deep, embed, wraps, ih]

theorem withoutCap_exceeds : NESTING_MAX < wraps (embed (deep (NESTING_MAX + 1))) := by
  rw [wraps_embed_deep]
  omega

/-! ## 预览窗口止于块末（D25）

`cutEnd` 是 `documents::cut` 切出的终点（不劈开字符，不过 `WINDOW_BYTES_MAX`，`spec/Window.lean`），`size` 是版本的长度。窗口没到版本末尾时，它最后一块可能还在窗口之外接着写，所以窗口止于倒数第二块的末尾 `penultimate`（Rust 里是 `layout::blocks` 读窗口字节得到的块，加上窗口的起点）；没有倒数第二块、或它不在起点之后，就留在 `cutEnd`。 -/

def settle (start cutEnd size : Nat) (penultimate : Option Nat) : Nat :=
  if size ≤ cutEnd then cutEnd
  else match penultimate with
    | some e => if start < e then e else cutEnd
    | none => cutEnd

theorem settle_le_cut (start cutEnd size : Nat) (p : Option Nat)
    (within : ∀ e, p = some e → e ≤ cutEnd) : settle start cutEnd size p ≤ cutEnd := by
  cases p with
  | none =>
    show (if size ≤ cutEnd then cutEnd else cutEnd) ≤ cutEnd
    split <;> exact Nat.le_refl _
  | some e =>
    have inside := within e rfl
    show (if size ≤ cutEnd then cutEnd else if start < e then e else cutEnd) ≤ cutEnd
    split
    · exact Nat.le_refl _
    · split
      · exact inside
      · exact Nat.le_refl _

theorem settle_progress (start cutEnd size : Nat) (p : Option Nat) (more : start < cutEnd) :
    start < settle start cutEnd size p := by
  cases p with
  | none =>
    show start < (if size ≤ cutEnd then cutEnd else cutEnd)
    split <;> exact more
  | some e =>
    show start < (if size ≤ cutEnd then cutEnd else if start < e then e else cutEnd)
    split
    · exact more
    · split
      · assumption
      · exact more

theorem settle_at_end (start cutEnd size : Nat) (p : Option Nat) (atEnd : size ≤ cutEnd) :
    settle start cutEnd size p = cutEnd := by
  unfold settle
  simp [atEnd]

/-- 不要求倒数第二块的末尾在起点之后。 -/
def settleWithoutGuard (cutEnd size : Nat) (p : Option Nat) : Nat :=
  if size ≤ cutEnd then cutEnd else p.getD cutEnd

theorem withoutGuard_stalls : ¬ (3 < settleWithoutGuard 10 20 (some 3)) := by
  decide

/-! ## 「不支持」与「内容为空」（A4，D25）

答复的形状：读出的块（可以是零个），或者「这份版本的编码本文法不读」。Rust 里是 `documents::Preview::{Laid, Unsupported}`；块一级的「不支持」是 `Block::Unsupported`，与一个内容为空的代码块 `Block::Code { text: "" }` 同样是两个构造子。 -/

inductive Preview where
  | laid (blocks : Nat)
  | unsupported

theorem empty_is_not_unsupported : Preview.laid 0 ≠ Preview.unsupported := by
  intro same
  cases same

end Documents.Markdown
