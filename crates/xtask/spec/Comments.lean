-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# comments 与 length 的文件面

整数判定与 Rust 的 `length::FILE_LINE_LIMIT`、`comments::COMMENT_PERCENT_LIMIT`
对照。词法分类由 Rust 夹具看守；这里证明预算比较的边界与可达性。
-/

namespace Comments

def fileLineLimit : Nat := 500
def commentPercentLimit : Nat := 10

structure Reading where
  fileLines : Nat
  commentLines : Nat
  judgedLines : Nat
  deriving Repr, DecidableEq

def overCommentBudget (reading : Reading) : Prop :=
  reading.commentLines * 100 > reading.judgedLines * commentPercentLimit

def admits (reading : Reading) : Prop :=
  reading.fileLines ≤ fileLineLimit ∧ ¬overCommentBudget reading

instance (reading : Reading) : Decidable (admits reading) := by
  unfold admits overCommentBudget
  infer_instance

theorem exact_boundary_is_admitted :
    admits { fileLines := 500, commentLines := 10, judgedLines := 100 } := by
  decide

theorem empty_input_is_admitted :
    admits { fileLines := 0, commentLines := 0, judgedLines := 0 } := by
  decide

theorem overlong_file_is_rejected :
    ¬admits { fileLines := 501, commentLines := 0, judgedLines := 1 } := by
  decide

theorem excess_comments_are_rejected :
    ¬admits { fileLines := 100, commentLines := 11, judgedLines := 100 } := by
  decide

theorem one_comment_can_fill_a_file :
    ¬admits { fileLines := 1, commentLines := 1, judgedLines := 1 } := by
  decide

end Comments
