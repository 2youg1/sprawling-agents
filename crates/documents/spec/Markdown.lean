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
5. **回复的收束点只进不退，不过它读过的行**（`closed_stays`、`closed_le_length`、`streaming_le_settled`）：接着写下去的字不会把已经收束的块收回去；还在说的回复读的不比结算之后多（D30、D31）。顶格开启的代码块闭合时收束，缩进开启的不收束（`a_margin_fence_closes`、`an_indented_fence_closes_nothing`）。

咬得动的演示：不判协议就放过 `javascript:`（`withoutCheck_admits_other`）；不设上限，十七层的引用读出十七层（`withoutCap_exceeds`）；不要求末尾在起点之后，窗口停在原地（`withoutGuard_stalls`）；不认围栏，代码块里的空行被当成收束点（`withoutFence_closes_inside_code`）。
-/

namespace Documents.Markdown

/-! ## 链接的去处（D23）

Rust 先按 WHATWG URL 的读法去掉制表符与换行、去掉开头的控制字符与空格，再取第一个 `:` 之前、合乎 RFC 3986 `scheme` 文法的那一段；没有这一段的是相对地址。本模型从分好的类开始：怎样从字符串分出类是 Rust 的 `markdown::target::scheme_of`，由 `markdown::target::tests` 的表判。 -/

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

/-! ## 回复里已经不会再变的块（D30、D31）

Rust 的 `layout::closed` 逐行读一段还在写的 Markdown，只读以换行结束的完整行，答最后一个收束点之后的字节位置。模型以行计位置：字节怎样切成行、一行属于哪一类是 Rust 的 `layout` 的事（`layout::tests::closed_*` 判），这里从分好类的行开始。围栏的界定符 `D` 与「这一行闭合那一个」的关系 `closes` 是参数：Rust 里是 `layout::Delimiter` 与它的 `closes`（同一字符、不短于开启行）。 -/

/-- 一个围栏行从第一列开始，还是缩进了一到三列。 -/
inductive Margin where
  | atMargin
  | indented
  deriving DecidableEq, Repr

/-- 一行完整的字按收束规则分的类：只有空白、顶格的 ATX 标题、围栏、其余。Rust 先判围栏，再判空白与标题，所以四类不相交。 -/
inductive Line (D : Type) where
  | blank
  | heading
  | fence (delimiter : D) (margin : Margin)
  | other

/-- 读到某一行时的状态：开着的代码块（它的界定符与开启处）、读过几行、收束点在第几行之后。 -/
structure Scan (D : Type) where
  code : Option (D × Margin)
  seen : Nat
  closed : Nat

def Scan.init {D : Type} : Scan D := ⟨none, 0, 0⟩

/-- 这一行是不是收束点：代码块之外的空行与顶格标题是；闭合一个顶格开启的代码块的那一行是；代码块里的其余一切都只是代码。 -/
def closesHere {D : Type} (closes : D → D → Bool) : Option (D × Margin) → Line D → Bool
  | none, .blank => true
  | none, .heading => true
  | some (o, .atMargin), .fence d _ => closes d o
  | _, _ => false

/-- 读完这一行之后开着的代码块：代码块之外的围栏开启一个，闭合它的围栏关上它。 -/
def nextCode {D : Type} (closes : D → D → Bool) : Option (D × Margin) → Line D → Option (D × Margin)
  | none, .fence d m => some (d, m)
  | some (o, om), .fence d _ => if closes d o then none else some (o, om)
  | code, _ => code

def step {D : Type} (closes : D → D → Bool) (s : Scan D) (l : Line D) : Scan D :=
  { code := nextCode closes s.code l
    seen := s.seen + 1
    closed := if closesHere closes s.code l then s.seen + 1 else s.closed }

/-- 收束点：读完这些完整行之后，已经不会再变的块止于第几行之后。 -/
def closedUpTo {D : Type} (closes : D → D → Bool) (lines : List (Line D)) : Nat :=
  (lines.foldl (step closes) Scan.init).closed

theorem step_moves_on {D : Type} (closes : D → D → Bool) (s : Scan D) (l : Line D)
    (inv : s.closed ≤ s.seen) :
    s.closed ≤ (step closes s l).closed ∧ (step closes s l).closed ≤ (step closes s l).seen
      ∧ (step closes s l).seen = s.seen + 1 := by
  show s.closed ≤ (if closesHere closes s.code l then s.seen + 1 else s.closed)
    ∧ (if closesHere closes s.code l then s.seen + 1 else s.closed) ≤ s.seen + 1
    ∧ s.seen + 1 = s.seen + 1
  by_cases here : closesHere closes s.code l = true <;> simp [here] <;> omega

theorem fold_moves_on {D : Type} (closes : D → D → Bool) (lines : List (Line D)) :
    ∀ s : Scan D, s.closed ≤ s.seen →
      s.closed ≤ (lines.foldl (step closes) s).closed
        ∧ (lines.foldl (step closes) s).closed ≤ (lines.foldl (step closes) s).seen
        ∧ (lines.foldl (step closes) s).seen = s.seen + lines.length := by
  induction lines with
  | nil => intro s inv; simp only [List.foldl_nil, List.length_nil]; omega
  | cons l rest ih =>
    intro s inv
    obtain ⟨grows, within, counted⟩ := step_moves_on closes s l inv
    obtain ⟨grows', within', counted'⟩ := ih (step closes s l) within
    simp only [List.foldl_cons, List.length_cons]
    exact ⟨Nat.le_trans grows grows', within', by omega⟩

/-- 收束点不过读过的行。 -/
theorem closed_le_length {D : Type} (closes : D → D → Bool) (lines : List (Line D)) :
    closedUpTo closes lines ≤ lines.length := by
  obtain ⟨_, within, counted⟩ := fold_moves_on closes lines Scan.init (Nat.le_refl 0)
  unfold closedUpTo
  rw [counted] at within
  simpa [Scan.init] using within

/-- 收束点只进不退：接着写下去的行不会把已经收束的块收回去。 -/
theorem closed_stays {D : Type} (closes : D → D → Bool) (lines more : List (Line D)) :
    closedUpTo closes lines ≤ closedUpTo closes (lines ++ more) := by
  unfold closedUpTo
  rw [List.foldl_append]
  obtain ⟨_, within, _⟩ := fold_moves_on closes lines Scan.init (Nat.le_refl 0)
  exact (fold_moves_on closes more _ within).1

/-- 一段回复的状态：还在说，或已结算（Rust 的 `ReplyState`）。 -/
inductive ReplyState where
  | streaming
  | settled

/-- `reply` 读到第几行之前：结算了的回复整段读，还在说的读到收束点。 -/
def reach {D : Type} (closes : D → D → Bool) : ReplyState → List (Line D) → Nat
  | .settled, lines => lines.length
  | .streaming, lines => closedUpTo closes lines

theorem streaming_le_settled {D : Type} (closes : D → D → Bool) (lines : List (Line D)) :
    reach closes .streaming lines ≤ reach closes .settled lines :=
  closed_le_length closes lines

/-- 界定符相同即闭合的一个具体例子，演示用。 -/
def sameMark (closing opened : Nat) : Bool := closing == opened

theorem a_margin_fence_closes :
    closedUpTo sameMark [.fence 3 .atMargin, .other, .fence 3 .atMargin] = 3 := by
  decide

theorem an_indented_fence_closes_nothing :
    closedUpTo sameMark [.fence 3 .indented, .other, .fence 3 .indented] = 0 := by
  decide

/-- 不认围栏的读法：围栏行当作其余的行。 -/
def withoutFences {D : Type} : Line D → Line D
  | .fence _ _ => .other
  | l => l

/-- 一个还开着的代码块里有一个空行：认围栏时什么也没收束，不认时空行成了收束点，代码块的前半被当成读完了的块。 -/
theorem withoutFence_closes_inside_code :
    closedUpTo sameMark ([.fence 3 .atMargin, .blank, .other].map withoutFences)
      ≠ closedUpTo sameMark [.fence 3 .atMargin, .blank, .other] := by
  decide

end Documents.Markdown
