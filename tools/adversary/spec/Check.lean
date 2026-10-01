-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# 收缩，以及一次只借出一个的端口

规定检验器 `Sprawling.Check` 的收缩与 `Sprawling.Ground` 的端口池必须守的性质
（`tools/adversary/src/Sprawling/Check.lean`、`tools/adversary/src/Sprawling/Ground.lean`）。

**收缩的每一个候选都更短。** 候选是从一条失败的轨迹里删掉一段相邻的元素（先整块、减半，
再逐个）；元素本身从不变小，因为每个值都是固定演员表里的一个词（`Spec.lean` §14）。删掉
至少一个、且删的起点在表内，候选就比原表短，于是「一个更小的候选仍失败就换上它」这一链
不会原地打转：它在原表长度那么多步之内停下，与检验器给收缩设的步数上限无关。

**一个端口一次只借给一个场地。** 两个场地抢同一个端口时不是相撞，而是**换城**：输的一边
的城绑不上端口退出，它自己的探活却在同一个口上接到赢的一边的城（`Spec.lean` §16）。所以
端口池是一个表，借出取表头、归还放回表头，借出的那个不在剩下的表里。
-/

namespace Adversary.Check

/-- 从 `values` 里删掉自 `index` 起相邻的 `count` 个。 -/
def removeAt (values : List α) (index count : Nat) : List α :=
  values.take index ++ values.drop (index + count)

/-- 删掉至少一个、起点在表内的候选严格更短。 -/
theorem a_removal_is_shorter (values : List α) (index count : Nat)
    (removes : 0 < count) (inside : index < values.length) :
    (removeAt values index count).length < values.length := by
  simp only [removeAt, List.length_append, List.length_take, List.length_drop]
  omega

/-- 一个更短的候选反复替换当前轨迹，替换的次数不超过原轨迹的长度。

`chain` 是一串依次替换的轨迹长度，每一项都比前一项小。 -/
theorem a_shrinking_chain_is_no_longer_than_the_trace (chain : List Nat) (start : Nat)
    (descends : List.Pairwise (· > ·) (start :: chain)) : chain.length ≤ start := by
  induction chain generalizing start with
  | nil => simp
  | cons next rest ih =>
    simp only [List.pairwise_cons, List.mem_cons, forall_eq_or_imp] at descends
    have below : next < start := descends.1.1
    have tail : rest.length ≤ next := ih next (List.pairwise_cons.mpr ⟨descends.2.1, descends.2.2⟩)
    simp only [List.length_cons]
    omega

/-- 借一个端口：表头，与剩下的池；池空时借不到。 -/
def claim : List Nat → Option (Nat × List Nat)
  | [] => none
  | port :: rest => some (port, rest)

/-- 还一个端口。 -/
def release (pool : List Nat) (port : Nat) : List Nat := port :: pool

/-- 池里没有重复时，借出去的那个不在剩下的池里：同一个端口不会再借给第二个场地。 -/
theorem a_lent_port_is_not_lent_again (pool rest : List Nat) (port : Nat)
    (distinct : pool.Nodup) (lent : claim pool = some (port, rest)) : port ∉ rest := by
  cases pool with
  | nil => simp [claim] at lent
  | cons head tail =>
    simp only [claim, Option.some.injEq, Prod.mk.injEq] at lent
    obtain ⟨rfl, rfl⟩ := lent
    exact (List.nodup_cons.mp distinct).1

/-- 还回一个借出去的端口，池仍然没有重复。 -/
theorem returning_a_lent_port_keeps_the_pool_distinct (pool rest : List Nat) (port : Nat)
    (distinct : pool.Nodup) (lent : claim pool = some (port, rest)) :
    (release rest port).Nodup := by
  cases pool with
  | nil => simp [claim] at lent
  | cons head tail =>
    simp only [claim, Option.some.injEq, Prod.mk.injEq] at lent
    obtain ⟨rfl, rfl⟩ := lent
    exact distinct

end Adversary.Check
