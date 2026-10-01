-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# 门说了什么：接受、拒绝、静默

规定检验器 `Sprawling.Door` 的 `interpret` 与 `Answer` 必须守的性质（`tools/adversary/src/Sprawling/Door.lean`）。
检验器不 import 本规格，本规格也不 import 检验器（ARCHITECTURE.md §11）：这里给出的是
参照定义与它的定理，检验器是实现，两者的一致由 `just adversary` 的门契约那一组检查从门外问
（`Spec.lean` §16）。

**静默不是接受。** `sprawling call` 要等满静默窗口才返回，一条还在被服务的命令与一条什么都
没产生的命令在门外长得一样；把两者并成一个，就会把一次被拒的动作读成一次成功（`Spec.lean`
§4 第一个发现）。握手与日志同样不算回答：城在日志那条通道上**也**叙述它的拒绝，把叙述算成
回答，就会把一条城根本没受理的命令报成受理了。
-/

namespace Adversary.Answer

/-- 一次调用在标准输出上收到的一枚帧，只按类，不带内容：`wire::ServerFrame` 的六类。 -/
inductive Heard where
  | welcome
  | log
  | event
  | answer
  | refusal
  | delta
deriving DecidableEq, Repr

/-- 门的三种回答。 -/
inductive Said where
  | accepted
  | denied
  | quiet
deriving DecidableEq, Repr

/-- 一枚帧是不是城的拒绝。 -/
def isRefusal : Heard → Bool
  | .refusal => true
  | .welcome | .log | .event | .answer | .delta => false

/-- 一枚帧算不算对命令的回答：握手与日志不算。 -/
def answers : Heard → Bool
  | .welcome | .log => false
  | .event | .answer | .refusal | .delta => true

/-- 一枚帧只是叙述：握手或日志。 -/
def narrates : Heard → Bool
  | .welcome | .log => true
  | .event | .answer | .refusal | .delta => false

/-- 门把收到的东西归成三种回答之一。

`localRefusal` 是客户端自己在开套接字之前的拒绝（标准错误上带 `E_` 码的那一行）：两条拒绝
通道意思不同，城的拒绝帧先读，客户端的拒绝其次，两者都没有时，有一枚回答帧才是接受。 -/
def classify (localRefusal : Bool) (heard : List Heard) : Said :=
  if heard.any isRefusal then .denied
  else if localRefusal then .denied
  else if heard.any answers then .accepted
  else .quiet

/-- 每个动作声明它期待哪一种回答；静默不在其中，因为它不是任何一种期待。 -/
inductive Expectation where
  | accepted
  | denied
deriving DecidableEq, Repr

def Expectation.met : Expectation → Said → Bool
  | .accepted, .accepted => true
  | .denied, .denied => true
  | .accepted, .denied | .accepted, .quiet | .denied, .accepted | .denied, .quiet => false

/-- 静默从不满足任何期待。 -/
theorem quiet_meets_no_expectation (expected : Expectation) : expected.met .quiet = false := by
  cases expected <;> rfl

/-- 只有握手与日志的一次调用是静默：叙述不是回答。 -/
theorem narration_alone_is_quiet (heard : List Heard) (only : heard.all narrates = true) :
    classify false heard = .quiet := by
  have noRefusal : heard.any isRefusal = false := by
    induction heard with
    | nil => rfl
    | cons frame rest ih =>
      simp only [List.all_cons, Bool.and_eq_true] at only
      cases frame <;> simp_all [narrates, isRefusal]
  have noAnswer : heard.any answers = false := by
    induction heard with
    | nil => rfl
    | cons frame rest ih =>
      simp only [List.all_cons, Bool.and_eq_true] at only
      cases frame <;> simp_all [narrates, answers]
  simp [classify, noRefusal, noAnswer]

/-- 一枚拒绝帧就是拒绝，不论别的帧说了什么。 -/
theorem a_refusal_frame_is_a_refusal (localRefusal : Bool) (heard : List Heard)
    (refused : heard.any isRefusal = true) : classify localRefusal heard = .denied := by
  simp [classify, refused]

/-- 门说接受，就一定收到过一枚回答帧：接受不能从沉默里推出来。 -/
theorem acceptance_was_heard (localRefusal : Bool) (heard : List Heard)
    (accepted : classify localRefusal heard = .accepted) : heard.any answers = true := by
  unfold classify at accepted
  cases hr : heard.any isRefusal <;> cases hl : localRefusal <;> cases ha : heard.any answers <;>
    simp_all

end Adversary.Answer
