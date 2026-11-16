-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# guide：启动时进不进快速开始，与跳过之后落在哪

规定 `client/src/views/welcome/guide.ts` 的 `opensGuide` 与 `skipAll`，以及 `client/src/app.svelte` 在启动时据它做的那一次换页（`client/Spec.lean` §7G、D54）。TypeScript 是「怎样守住」的权威，本模型是「必须守住哪些性质」的权威；`guide.test.ts` 判 `opensGuide` 与这里的定义读出同样的答案。

启动的判定只做一次：城第一次答出端点表时，外壳若停在对话页而 `opensGuide` 为真，就换到上手指南；此后的回答不再换页。只做一次，是为了让「跳过全部可选项」之后进的对话页不被下一个回答又送回指南。

性质：

1. **城里没有任何 provider 端点时，启动总是进快速开始**，无论指南记着已离开、这个浏览器走没走过、有没有 `main` 模型（`no_endpoint_always_opens_the_guide`）。
2. **跳过全部可选项之后落在对话页，并且停在那里**：之后任意多个端点表的回答都不再把人送回指南（`skipping_lands_in_main_for_good`）。

咬得动的演示：每个回答都判一次的写法（`stepEvery`）在跳过之后被下一个回答送回指南（`judging_every_answer_sends_the_person_back`）。
-/

namespace Client.Views.Guide

/-- 启动判定读的事实：端点数、有没有 `main` 模型、这个浏览器走没走过指南、城记着指南已离开。 -/
structure Launch where
  endpoints : Nat
  main : Bool
  welcomed : Bool
  left : Bool

/-- `opensGuide`：没有端点时总是进；有端点而没有 `main` 时，只在这个浏览器没走过指南时进。 -/
def opensGuide (l : Launch) : Bool :=
  l.endpoints == 0 || (!l.main && !l.welcomed)

/-- 外壳停在哪类页上。 -/
inductive View where
  | talk
  | welcome
  | other
  deriving DecidableEq

/-- 外壳的状态：启动判定做过没有，停在哪类页上。 -/
structure Shell where
  decided : Bool
  view : View

/-- 外壳收到的事。 -/
inductive Event where
  /-- 城答出端点表。 -/
  | answered (l : Launch)
  /-- 人在指南上按「跳过全部可选项」。 -/
  | skipAll
  /-- 人换到另一页。 -/
  | go (v : View)

def step (s : Shell) : Event → Shell
  | .answered l =>
    if s.decided then s
    else { decided := true, view := if opensGuide l && s.view == .talk then .welcome else s.view }
  | .skipAll => { s with view := .talk }
  | .go v => { s with view := v }

def run (s : Shell) (events : List Event) : Shell := events.foldl step s

theorem no_endpoint_always_opens_the_guide (main welcomed left : Bool) :
    (step ⟨false, .talk⟩ (.answered ⟨0, main, welcomed, left⟩)).view = .welcome := by
  simp [step, opensGuide]

theorem answers_after_the_decision_change_nothing :
    ∀ (ls : List Launch) (s : Shell), s.decided = true → run s (ls.map Event.answered) = s := by
  intro ls
  induction ls with
  | nil => intro s _; rfl
  | cons l ls ih =>
    intro s h
    simp only [List.map, run, List.foldl]
    have hs : step s (.answered l) = s := by simp [step, h]
    rw [hs]
    exact ih s h

theorem skipping_lands_in_main_for_good (s : Shell) (h : s.decided = true) (ls : List Launch) :
    (run (step s .skipAll) (ls.map Event.answered)).view = .talk := by
  rw [answers_after_the_decision_change_nothing ls _ (by simp [step, h])]
  rfl

/-- 每个回答都判一次的写法。 -/
def stepEvery (s : Shell) : Event → Shell
  | .answered l => { s with view := if opensGuide l && s.view == .talk then .welcome else s.view }
  | .skipAll => { s with view := .talk }
  | .go v => { s with view := v }

theorem judging_every_answer_sends_the_person_back :
    ([Event.skipAll, .answered ⟨0, false, true, true⟩].foldl stepEvery ⟨true, .welcome⟩).view = .welcome := by
  rfl

end Client.Views.Guide
