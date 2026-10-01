-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# 验收世界：替身的回复怎样被几个 run 分着花，与按序走、停在第一处失败

规定验收世界 `Sprawling.Acceptance`（`tools/adversary/src/Sprawling/Acceptance/Script.lean`、
`tools/adversary/src/Sprawling/Acceptance/Walk.lean`）必须守的性质。

**替身按请求到达的次序花掉回复**（`tools/citysim/citysim-SPEC.md` §8-10），所以一份脚本只有在
它的读者一次只派一个 run 时才有确定的意义。唯一不等的是被杀的那个 run：它在被杀之前花掉了
多少条回复，取决于刀落在哪里。脚本给它的是一串同样的只读调用，接着是几句收尾的话；于是不论
它花掉了多少条只读调用，接下来花剩下回复的那个 run 都以一句收尾的话结束，而收尾的话写两遍
以上时，城回来之后即便也捡起那个死掉的 run，两个 run 各自都以收尾的话结束，谁都不会碰上
「脚本已用尽」的拒绝。

**按序走、停在第一处失败。** 每一步站在前面几步留下的城上，接着走只会把一个原因报成许多个；
报出来的是第一步失败的那一步，它之前的每一步都通过了。
-/

namespace Adversary.Acceptance

/-- 被杀的 run 与它之后的 run 分着花的那一段脚本里的一条回复。 -/
inductive Reply where
  /-- 一次只读的工具调用。 -/
  | status
  /-- 一句话、不调工具：一个 run 以它结束。 -/
  | closing
deriving DecidableEq, Repr

/-- 一个 run 怎样结束：以自己的最后一句话，或碰上替身用尽脚本的拒绝。 -/
inductive Ending where
  | ownLine
  | exhausted
deriving DecidableEq, Repr

/-- 一个 run 从剩下的回复里一直花，直到花掉一句收尾的话，或脚本用尽；答它怎样结束，以及
留给下一个 run 的回复。 -/
def runOn : List Reply → Ending × List Reply
  | [] => (.exhausted, [])
  | .closing :: rest => (.ownLine, rest)
  | .status :: rest => runOn rest

/-- 那一段脚本：`calls` 次只读调用，再 `closings` 句收尾的话。 -/
def interrupted (calls closings : Nat) : List Reply :=
  List.replicate calls .status ++ List.replicate closings .closing

/-- 只读调用一条一条被花掉，不改变接下来那句收尾的话落在谁身上。 -/
theorem calls_pass_through (calls : Nat) (rest : List Reply) :
    runOn (List.replicate calls .status ++ rest) = runOn rest := by
  induction calls with
  | zero => rfl
  | succ calls ih => simp [List.replicate_succ, runOn, ih]

/-- 被杀的 run 花掉了任意多条只读调用之后，剩下的是同样形状的一段：少了几条只读调用。 -/
theorem what_the_killed_run_leaves (calls closings spent : Nat) (within : spent ≤ calls) :
    (interrupted calls closings).drop spent = interrupted (calls - spent) closings := by
  simp [interrupted, List.drop_append_of_le_length, List.length_replicate, within,
    List.drop_replicate]

/-- 不论被杀的 run 花掉了多少条只读调用，城回来之后派的活都以自己的最后一句话结束。 -/
theorem the_work_after_the_crash_ends_on_its_own_line (calls closings spent : Nat)
    (within : spent ≤ calls) :
    (runOn ((interrupted calls (closings + 1)).drop spent)).1 = .ownLine := by
  rw [what_the_killed_run_leaves calls (closings + 1) spent within]
  simp [interrupted, calls_pass_through, List.replicate_succ, runOn]

/-- 收尾的话写两遍以上时，城回来之后若有两个 run 先后花这一段，两个都以自己的最后一句话结束。 -/
theorem two_runs_after_the_crash_both_end_on_their_own_lines (calls closings spent : Nat)
    (within : spent ≤ calls) :
    let first := runOn ((interrupted calls (closings + 2)).drop spent)
    first.1 = .ownLine ∧ (runOn first.2).1 = .ownLine := by
  rw [what_the_killed_run_leaves calls (closings + 2) spent within]
  simp [interrupted, calls_pass_through, List.replicate_succ, runOn]

/-- 一句收尾的话都没有时，接着花的 run 碰上的是用尽：所以收尾的话至少一句，这条不是多余的。 -/
theorem without_a_closing_line_the_script_runs_out (calls : Nat) :
    (runOn (interrupted calls 0)).1 = .exhausted := by
  show (runOn (List.replicate calls .status ++ [])).1 = .exhausted
  rw [calls_pass_through]
  rfl

/-- 按序走：每一步答通过或失败；走到第一处失败就停，报出它的位置。 -/
def firstBroken : List Bool → Option Nat
  | [] => none
  | true :: rest => (firstBroken rest).map (· + 1)
  | false :: _ => some 0

/-- 报出来的那一步确实失败了，它之前的每一步都通过了。 -/
theorem the_reported_step_broke_and_every_earlier_one_held (steps : List Bool) (index : Nat)
    (reported : firstBroken steps = some index) :
    steps[index]? = some false ∧ ∀ earlier, earlier < index → steps[earlier]? = some true := by
  induction steps generalizing index with
  | nil => simp [firstBroken] at reported
  | cons step rest ih =>
    cases step with
    | false =>
      simp only [firstBroken, Option.some.injEq] at reported
      subst reported
      exact ⟨rfl, fun earlier below => absurd below (Nat.not_lt_zero earlier)⟩
    | true =>
      simp only [firstBroken, Option.map_eq_some_iff] at reported
      obtain ⟨inner, found, rfl⟩ := reported
      obtain ⟨broke, held⟩ := ih inner found
      refine ⟨by simpa using broke, fun earlier below => ?_⟩
      cases earlier with
      | zero => rfl
      | succ earlier => simpa using held earlier (by omega)

/-- 每一步都通过时，没有一步被报出来。 -/
theorem a_walk_that_held_reports_nothing (steps : List Bool) (held : steps.all id = true) :
    firstBroken steps = none := by
  induction steps with
  | nil => rfl
  | cons step rest ih =>
    simp only [List.all_cons, Bool.and_eq_true, id] at held
    obtain ⟨first, others⟩ := held
    subst first
    simp [firstBroken, ih others]

end Adversary.Acceptance
