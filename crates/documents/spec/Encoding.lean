-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# encoding：一个版本的字节拼出哪些字符

规定 `crates/documents/src/encoding.rs`（`documents::Encoding`、`Reading::of`、`Encoding::of_mark`、`Encoding::decode`）。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。

字节顺序标记先判：有标记就照标记读，没有就是 UTF-8 或什么都不是（D4）。某串字节在 UTF-8、UTF-16LE、UTF-16BE 下是否合法是参数 `Validity`：那是 Unicode 的事实，Rust 用标准库的 `str::from_utf8` 与 `char::decode_utf16` 判它，模型不重述。

证明的性质：

1. **带标记的 UTF-16 是文本，不管里面有多少 NUL**（`marked_utf16le_is_text`）：旧的判法凭 NUL 判二进制，正好把每个 ASCII 字符旁边都有一个 NUL 的 UTF-16 文件判成二进制（refrain 路线图附录 C）。
2. **没有标记而含 NUL 的不是文本**（`nul_without_mark_is_opaque`）：文本文件不含 NUL，城自己存的东西（redb、CAS 里的块）含。
3. **判成文本的，在它的编码下拼得出**（`text_is_spelled`）：判定与解码读同一条规则，所以一个判成文本的版本，它的每个窗口都解得出（窗口切在字符边界上，`spec/Window.lean`）。

**照旧的 NUL 判法，带标记的 UTF-16 文件就成了不透明的**（`nulJudgement_calls_utf16_opaque`）：本模型咬得动的演示。
-/

namespace Documents.Encoding

inductive Encoding where
  | utf8
  | utf8Bom
  | utf16Le
  | utf16Be
  deriving DecidableEq, Repr

inductive Reading where
  | text (encoding : Encoding)
  | opaque
  deriving DecidableEq, Repr

/-- 三种编码各自的合法性，由 Unicode 定义，是模型的参数。 -/
structure Validity where
  utf8 : List Nat → Bool
  utf16Le : List Nat → Bool
  utf16Be : List Nat → Bool

/-- 前三个字节里的标记说的编码；没有标记就是 UTF-8。 -/
def ofMark : List Nat → Encoding
  | 0xEF :: 0xBB :: 0xBF :: _ => .utf8Bom
  | 0xFF :: 0xFE :: _ => .utf16Le
  | 0xFE :: 0xFF :: _ => .utf16Be
  | _ => .utf8

/-- 一串字节在这种编码下拼不拼得出文本。 -/
def spells (v : Validity) : Encoding → List Nat → Bool
  | .utf8, s => v.utf8 s && !s.contains 0
  | .utf8Bom, s => v.utf8 s
  | .utf16Le, s => v.utf16Le s
  | .utf16Be, s => v.utf16Be s

/-- `Reading::of`：标记先判，再看字节拼不拼得出。 -/
def reading (v : Validity) (s : List Nat) : Reading :=
  if spells v (ofMark s) s then .text (ofMark s) else .opaque

theorem marked_utf16le_is_text (v : Validity) (rest : List Nat)
    (valid : v.utf16Le (0xFF :: 0xFE :: rest) = true) :
    reading v (0xFF :: 0xFE :: rest) = .text .utf16Le := by
  simp [reading, ofMark, spells, valid]

theorem nul_without_mark_is_opaque (v : Validity) (s : List Nat)
    (unmarked : ofMark s = .utf8) (nul : 0 ∈ s) : reading v s = .opaque := by
  simp [reading, unmarked, spells, nul]

theorem text_is_spelled (v : Validity) (s : List Nat) (e : Encoding)
    (judged : reading v s = .text e) : spells v e s = true := by
  unfold reading at judged
  split at judged
  · rename_i spelled
    cases judged
    exact spelled
  · cases judged

/-!
## 咬得动的演示

旧的判法：含 NUL 就是二进制，否则当 UTF-8 读。一个带标记的 UTF-16LE 文件 `"a"` 是 `FF FE 61 00`，它被判成不透明的。
-/

def nulJudgement (s : List Nat) : Reading :=
  if s.contains 0 then .opaque else .text .utf8

theorem nulJudgement_calls_utf16_opaque :
    nulJudgement [0xFF, 0xFE, 0x61, 0x00] = .opaque := by
  decide

end Documents.Encoding
