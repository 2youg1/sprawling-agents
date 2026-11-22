-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# comments 与 length 的文件面

整数判定与 Rust 的 `length::FILE_LINE_LIMIT` 对照。词法分类由 Rust 夹具看守；
这里证明文件行数边界，以及非 MPL 注释零容忍的可达性。
-/

namespace Comments

def fileLineLimit : Nat := 500

structure Reading where
  fileLines : Nat
  commentLines : Nat
  deriving Repr, DecidableEq

def admits (reading : Reading) : Prop :=
  reading.fileLines ≤ fileLineLimit ∧ reading.commentLines = 0

instance (reading : Reading) : Decidable (admits reading) := by
  unfold admits
  infer_instance

theorem comment_free_source_is_admitted :
    admits { fileLines := 500, commentLines := 0 } := by
  decide

theorem empty_input_is_admitted :
    admits { fileLines := 0, commentLines := 0 } := by
  decide

theorem overlong_file_is_rejected :
    ¬admits { fileLines := 501, commentLines := 0 } := by
  decide

theorem one_comment_is_rejected :
    ¬admits { fileLines := 100, commentLines := 1 } := by
  decide

end Comments
