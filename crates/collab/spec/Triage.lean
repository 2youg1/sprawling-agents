-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# Triage：外来的东西落在哪，以及它能不能自己开工

规定 `crates/collab/src/triage.rs`（`collab::triage`，`Triage::decide`）。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。

一张人手写的路由表：规则自上而下，第一条命中的胜；子串、不分大小写，不是模式语言，因为这张表被读的次数远多于被写，正则会在出事那一刻多一件要 debug 的东西。命中的判法在这里是参数 `hits`，它是 Rust 的事。

三条性质：

1. **结果恒是一个地址**：`land` 是全函数，四种反射只决定「多重的反应体来接」；派活面因此只有一种形状。判不出不是错误，而是「一个人读它」（`unmatched_goes_to_a_person`）。
2. **污染件可以被路由，不可以被开工**（`tainted_never_starts_work`）：规则写 `light` 或 `full`，污染件也只到 `notify`，降级的理由写进 `because`，这一层唯一的可观测面。
3. **拿掉降级，污染件就会自己开工**（`withoutCap_starts_work`）：本模型咬得动的演示。
-/

namespace Collab.Triage

/-- 多重的反应体来接：丢弃、告诉人、轻量开工、全量开工。 -/
inductive Reflex where
  | discard
  | notify
  | light
  | full
  deriving DecidableEq, Repr

structure Rule (Arrival : Type) where
  hits : Arrival → Bool
  landing : Nat
  reflex : Reflex

structure Landing where
  addr : Nat
  reflex : Reflex
  /-- 一条规则的反射被降级过。Rust 那边是 `because` 里的那句话。 -/
  held : Bool
  deriving DecidableEq, Repr

/-- 污染件的上限：会开工的两种降为 `notify`。 -/
def cap (tainted : Bool) : Reflex → Reflex
  | .light => if tainted then .notify else .light
  | .full => if tainted then .notify else .full
  | .discard => .discard
  | .notify => .notify

def land {Arrival : Type} (rules : List (Rule Arrival)) (fallback : Nat)
    (tainted : Arrival → Bool) (a : Arrival) : Landing :=
  match rules.find? (·.hits a) with
  | some r => ⟨r.landing, cap (tainted a) r.reflex, cap (tainted a) r.reflex != r.reflex⟩
  | none => ⟨fallback, .notify, false⟩

theorem cap_tainted (r : Reflex) : cap true r ≠ .light ∧ cap true r ≠ .full := by
  cases r <;> simp [cap]

theorem tainted_never_starts_work {Arrival : Type} (rules : List (Rule Arrival))
    (fallback : Nat) (tainted : Arrival → Bool) (a : Arrival) (h : tainted a = true) :
    (land rules fallback tainted a).reflex ≠ .light ∧
      (land rules fallback tainted a).reflex ≠ .full := by
  unfold land
  cases hf : rules.find? (·.hits a) with
  | some r => simp only [h]; exact cap_tainted r.reflex
  | none => simp

theorem unmatched_goes_to_a_person {Arrival : Type} (rules : List (Rule Arrival))
    (fallback : Nat) (tainted : Arrival → Bool) (a : Arrival)
    (h : rules.find? (·.hits a) = none) :
    land rules fallback tainted a = ⟨fallback, .notify, false⟩ := by
  simp [land, h]

/-!
## 咬得动的演示

把 `cap` 拿掉、规则的反射原样交出，`tainted_never_starts_work` 不再成立：一件从外面来的东西命中一条 `full` 规则，就自己开工了。
-/

def landWithoutCap {Arrival : Type} (rules : List (Rule Arrival)) (fallback : Nat)
    (a : Arrival) : Landing :=
  match rules.find? (·.hits a) with
  | some r => ⟨r.landing, r.reflex, false⟩
  | none => ⟨fallback, .notify, false⟩

theorem withoutCap_starts_work :
    (landWithoutCap [⟨fun (_ : Unit) => true, 3, .full⟩] 0 ()).reflex = .full := by
  rfl

end Collab.Triage
