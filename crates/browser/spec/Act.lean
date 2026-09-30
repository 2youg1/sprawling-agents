-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# act：对一页动手，只用刚看过的那一页

规定 `crates/browser/src/act.rs`（`browser::act`，`frame_for`、`ensure_fresh`、`selector_of`）与 `crates/browser/src/verb.rs` 里 `act` 一臂的判定。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。

一个动作带着它被决定时的 generation。ref 是本次快照里的位置，不是页面里的稳定标识（D3）：页面是别人写的，不配合 `id` 或 `data-testid`；把 ref 定成位置，「页面动过了」就成了一个可判定的事实。代价是每次动手前必须先看一眼，这正是要的顺序。

三条性质：

1. **陈旧的 generation 恒拒**（`stale_refused`）：一个对着某一版页面决定的点击，对着另一版被拒，而不是落在恰好挪到那个位置上的东西上；拒绝而不重试，因为调用方得再看一眼才知道自己此刻在点什么。
2. **没看过的页面不能动手**（`unseen_refused`）：没有快照，就没有 generation 可比。
3. **拿掉 generation 的守卫，陈旧的动作就会出网**（`withoutGuard_acts_on_a_stale_page`）：本模型咬得动的演示。

指针与按键动作（drag、scroll、press）不走 `frame_for`，它们是 BiDi 的 `input` 模块；这里的模型只说 script 的三臂（click、type、read）。页面文本进表达式的唯一位置是 `quote`，逐字符转义，U+2028 与 U+2029 在内（JS 里它们是行终止符），所以页面文本恒是数据，不是代码。
-/

namespace Browser.Act

/-- script 的三臂；带 ref 的都要先解析。 -/
inductive Action where
  | click (ref : Nat)
  | type (ref : Nat) (text : String)
  | read (ref : Nat)
  deriving DecidableEq, Repr

inductive Refusal where
  | unseen
  | stale
  | unknownRef
  deriving DecidableEq, Repr

/-- 调用方看过的那一页：它的 generation 与快照铸出的 ref 个数（`e1` 到 `e<n>`）。 -/
structure Seen where
  generation : Nat
  refs : Nat
  deriving DecidableEq, Repr

def Action.ref : Action → Nat
  | .click r => r
  | .type r _ => r
  | .read r => r

/-- 一帧 script：这里只记它点的是哪个 ref。 -/
def frameFor (seen : Option Seen) (generation : Nat) (a : Action) : Except Refusal Nat :=
  match seen with
  | none => .error .unseen
  | some s =>
    if generation ≠ s.generation then .error .stale
    else if a.ref = 0 ∨ a.ref > s.refs then .error .unknownRef
    else .ok a.ref

theorem stale_refused (s : Seen) (g : Nat) (a : Action) (h : g ≠ s.generation) :
    frameFor (some s) g a = .error .stale := by
  simp [frameFor, h]

theorem unseen_refused (g : Nat) (a : Action) : frameFor none g a = .error .unseen := rfl

theorem acted_on_the_seen_page {s : Seen} {g : Nat} {a : Action} {r : Nat}
    (h : frameFor (some s) g a = .ok r) : g = s.generation ∧ 1 ≤ r ∧ r ≤ s.refs := by
  unfold frameFor at h
  by_cases stale : g ≠ s.generation
  · simp [stale] at h
  · by_cases unknown : a.ref = 0 ∨ a.ref > s.refs
    · simp [stale, unknown] at h
    · simp only [stale, unknown, if_false, Except.ok.injEq] at h
      subst h
      omega

/-!
## 咬得动的演示

拿掉「generation 不等即拒」那条守卫，`stale_refused` 不再成立：对着第 3 版决定的点击，在第 4 版上照样出网。
-/

def frameForWithoutGuard (seen : Option Seen) (_generation : Nat) (a : Action) :
    Except Refusal Nat :=
  match seen with
  | none => .error .unseen
  | some s => if a.ref = 0 ∨ a.ref > s.refs then .error .unknownRef else .ok a.ref

theorem withoutGuard_acts_on_a_stale_page :
    frameForWithoutGuard (some ⟨4, 2⟩) 3 (.click 1) = .ok 1 := by
  rfl

end Browser.Act
