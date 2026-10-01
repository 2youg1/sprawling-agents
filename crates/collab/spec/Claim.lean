-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# Claim：分一行要先握着它，落地只核认领

规定 `crates/collab/src/claim_tool.rs`（`collab::claim_tool`，`ClaimDesk` 的 `claim`、`finish`、`block`、`release`、`split`）、`crates/collab/src/claim_effect.rs`（`collab::claim_effect`，`still_true`）与 `crates/accounting/src/effect.rs`（`accounting::effect`，`Claims::of`）。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。

一个 run 的桌子拿着派活那一刻的计划副本，模型每调一次 `plan`，桌子判一次、改副本、排一条效应；run 落地时，工人把同一组效应按次序重放到盘上此刻的那份计划上（`land`）。「分一行要不要握着它」只在桌子判（`Desk.act` 的 `split` 一支，collab D6）；落地不再为拆分另判状态，只核认领（`stillTrue`）——认领是唯一从共享计划里拿走一行的效应，也就是唯一会因为别的写者动过计划而失真的效应。

四条性质：

1. **桌子拒绝分一行没握着的计划**（`split_needs_hold`）：模型当场拿到拒词，桌子不排效应、不改副本。
2. **桌子答应的，落在派活那份计划上全部落下**（`admitted_lands`）：工具答成功而落地一字不写，在没有别的写者的时候不可达。第八个发现（`tools/adversary/Spec.lean` §4）正是这条的反例：旧的落地对拆分另判 `In progress`，而旧的桌子不要求握着。
3. **一行被别人动过，这个 run 对它的拆分落不下**（`split_of_moved_row_is_stale`）：每条拆分之前都有本 run 对同一行的认领，而且那条认领是这一行的第一条效应（`first_touch_is_claim`），所以盘上那一行已不是 `Not started` 时，整次落地判过时。
4. **拿掉握持守卫，两个 run 把同一行各分一次**（`withoutHold_splits_twice`）：本模型咬得动的演示。从同一份计划派出的两个 run 都不认领就分第 1 行，两次落地都核不出什么，第二次在第一次的子行之后再长一组。

模型的简化：计划的一行只有状态，依赖与叶子判定（`kernel::PlanTree::claim` 的就绪集）收成「`Not started`」一条；拆分只记「这一行被分过一次」（`divided`），子行怎样编号是 `kernel::spine::insert_children` 的事，本模型不重述。拆分在 Rust 里还要求新表能解析、能建树，那是一次与握持无关的拒绝，由 `crates/collab/src/claim_tool/tests.rs` 经生产入口断言。
-/

namespace Collab.Claim

/-- 一行计划的状态。`blocked` 代表两种红（`Blocked`、`Awaiting approval`），本模型分不出它们也不需要分。 -/
inductive Status where
  | notStarted
  | inProgress
  | done
  | blocked
  deriving DecidableEq, Repr

/-- 一份计划：每行的状态，与每次落下的拆分。同一行在 `divided` 里出现两次，就是它的子行长了两组。 -/
structure Plan where
  status : Nat → Status
  divided : List Nat

def Plan.set (p : Plan) (n : Nat) (s : Status) : Plan :=
  { p with status := fun m => if m = n then s else p.status m }

/-- `ClaimEffect`：认领、放下（带出口给的状态）、拆分。 -/
inductive Effect where
  | claimed (n : Nat)
  | putDown (n : Nat) (to : Status)
  | split (n : Nat)
  deriving DecidableEq, Repr

/-- `ClaimEffect::id`。 -/
def Effect.node : Effect → Nat
  | .claimed n | .putDown n _ | .split n => n

/-- `ClaimEffect::apply`：桌子改副本与工人改盘上那份用的是同一个定义。 -/
def Effect.apply (p : Plan) : Effect → Plan
  | .claimed n => p.set n .inProgress
  | .putDown n s => p.set n s
  | .split n => { p with divided := n :: p.divided }

/-- `still_true`：只有认领问计划，问的是那一行是否仍没人开始。放下与拆分只作用于本 run 握着的那一行，
它们的新鲜由握持之前的那条认领担保（collab D6），落地不为它们另判。 -/
def stillTrue (p : Plan) : Effect → Bool
  | .claimed n => p.status n == .notStarted
  | .putDown _ _ | .split _ => true

/-- `Claims::of`：按次序把效应重放到 `p` 上，每一条在前面几条留下的计划上核；有一条不真，整次落地过时（`none`），一字不写。 -/
def land (p : Plan) : List Effect → Option Plan
  | [] => some p
  | e :: rest => if stillTrue p e then land (e.apply p) rest else none

/-- 桌子：副本、握着的那一行、排好的效应。 -/
structure Desk where
  plan : Plan
  held : Option Nat
  effects : List Effect

def Desk.start (p : Plan) : Desk := ⟨p, none, []⟩

def Desk.take (d : Desk) (e : Effect) (held : Option Nat) : Desk :=
  ⟨e.apply d.plan, held, d.effects ++ [e]⟩

/-- 模型能调的动作；`finish`、`block`、`release` 收成一个带出口状态的 `putDown`。 -/
inductive Action where
  | claim (n : Nat)
  | putDown (n : Nat) (to : Status)
  | split (n : Nat)

/-- 桌子判一次调用：`none` 是拒绝，桌子不变。一次只握一行；放下与拆分都只作用于握着的那一行，并放开它。 -/
def Desk.act (d : Desk) : Action → Option Desk
  | .claim n =>
      if d.held = none ∧ d.plan.status n = .notStarted then some (d.take (.claimed n) (some n))
      else none
  | .putDown n s => if d.held = some n then some (d.take (.putDown n s) none) else none
  | .split n => if d.held = some n then some (d.take (.split n) none) else none

/-- 从派活那份计划出发、每一步都被桌子答应的桌子。 -/
inductive Reach (p : Plan) : Desk → Prop where
  | start : Reach p (Desk.start p)
  | step {d d' : Desk} (a : Action) : Reach p d → d.act a = some d' → Reach p d'

theorem split_needs_hold {d : Desk} {n : Nat} (h : d.held ≠ some n) :
    d.act (.split n) = none := by
  simp [Desk.act, h]

theorem land_append (p : Plan) (l : List Effect) (e : Effect) :
    land p (l ++ [e]) =
      (land p l).bind fun q => if stillTrue q e then some (e.apply q) else none := by
  induction l generalizing p with
  | nil => simp [land]
  | cons x rest ih =>
    simp only [List.cons_append, land]
    split <;> simp [ih]

/-- 桌子答应一步时，这一步的效应在桌子的副本上是真的，副本也正是重放它的结果。 -/
theorem act_true {d d' : Desk} {a : Action} (h : d.act a = some d') :
    ∃ e held, d' = d.take e held ∧ stillTrue d.plan e = true := by
  cases a with
  | claim n =>
    simp only [Desk.act] at h
    split at h
    · rename_i hc
      exact ⟨_, _, (Option.some.inj h).symm, by simp [stillTrue, hc.2]⟩
    · cases h
  | putDown n s =>
    simp only [Desk.act] at h
    split at h
    · exact ⟨_, _, (Option.some.inj h).symm, rfl⟩
    · cases h
  | split n =>
    simp only [Desk.act] at h
    split at h
    · exact ⟨_, _, (Option.some.inj h).symm, rfl⟩
    · cases h

theorem admitted_lands {p : Plan} {d : Desk} (h : Reach p d) :
    land p d.effects = some d.plan := by
  induction h with
  | start => rfl
  | step a _ hact ih =>
    obtain ⟨e, held, rfl, htrue⟩ := act_true hact
    simp [Desk.take, land_append, ih, htrue]

/-- 一行的第一条效应。 -/
def firstTouch (n : Nat) (l : List Effect) : Option Effect :=
  l.find? fun e => e.node == n

/-- 桌子守住的两件事：握着的那一行已经有效应；每一行的第一条效应是认领。 -/
def Settled (d : Desk) : Prop :=
  (∀ n, d.held = some n → (firstTouch n d.effects).isSome) ∧
  (∀ n e, firstTouch n d.effects = some e → e = .claimed n)

theorem firstTouch_append (n : Nat) (l : List Effect) (e : Effect) :
    firstTouch n (l ++ [e]) =
      (firstTouch n l).or (if e.node == n then some e else none) := by
  simp only [firstTouch, List.find?_append, List.find?_cons, List.find?_nil]
  split <;> simp_all

theorem settled_take {d : Desk} {e : Effect} {held : Option Nat} (hs : Settled d)
    (hnew : (firstTouch e.node d.effects).isSome ∨ e = .claimed e.node)
    (hheld : ∀ n, held = some n → n = e.node) :
    Settled (d.take e held) := by
  obtain ⟨hHeld, hFirst⟩ := hs
  refine ⟨fun n hn => ?_, fun n x hx => ?_⟩
  · have := hheld n hn
    subst this
    simp only [Desk.take, firstTouch_append, beq_self_eq_true, if_true]
    cases firstTouch e.node d.effects <;> simp
  · simp only [Desk.take, firstTouch_append] at hx
    cases hf : firstTouch n d.effects with
    | some y =>
      rw [hf] at hx
      simp only [Option.some_or, Option.some.injEq] at hx
      subst hx
      exact hFirst n y hf
    | none =>
      rw [hf] at hx
      simp only [Option.none_or] at hx
      split at hx
      · rename_i hn
        have hn : e.node = n := by simpa using hn
        subst hn
        cases Option.some.inj hx
        rcases hnew with hs | hc
        · rw [hf] at hs; cases hs
        · exact hc
      · cases hx

theorem reach_settled {p : Plan} {d : Desk} (h : Reach p d) : Settled d := by
  induction h with
  | start =>
    unfold Settled
    constructor
    · intro _ h
      simp [Desk.start] at h
    · intro _ _ h
      simp [Desk.start, firstTouch] at h
  | step a _ hact ih =>
    rename_i d d'
    cases a with
    | claim n =>
      simp only [Desk.act] at hact
      split at hact
      · cases hact
        exact settled_take ih (Or.inr rfl) (fun m hm => by cases hm; rfl)
      · cases hact
    | putDown n s =>
      simp only [Desk.act] at hact
      split at hact
      · rename_i hh
        cases hact
        exact settled_take ih (Or.inl (ih.1 n hh)) (fun _ hm => by cases hm)
      · cases hact
    | split n =>
      simp only [Desk.act] at hact
      split at hact
      · rename_i hh
        cases hact
        exact settled_take ih (Or.inl (ih.1 n hh)) (fun _ hm => by cases hm)
      · cases hact

theorem first_touch_is_claim {p : Plan} {d : Desk} {n : Nat} (h : Reach p d)
    (hs : Effect.split n ∈ d.effects) : firstTouch n d.effects = some (.claimed n) := by
  have hsome : (firstTouch n d.effects).isSome := by
    unfold firstTouch
    rw [List.find?_isSome]
    exact ⟨_, hs, by simp [Effect.node]⟩
  obtain ⟨e, he⟩ := Option.isSome_iff_exists.mp hsome
  rw [he, (reach_settled h).2 n e he]

theorem apply_other {q : Plan} {e : Effect} {n : Nat} (h : e.node ≠ n) :
    (e.apply q).status n = q.status n := by
  cases e with
  | claimed m =>
    simp only [Effect.node] at h
    simp [Effect.apply, Plan.set, Ne.symm h]
  | putDown m s =>
    simp only [Effect.node] at h
    simp [Effect.apply, Plan.set, Ne.symm h]
  | split m => rfl

theorem land_stale {n : Nat} :
    ∀ (l : List Effect) (q : Plan), firstTouch n l = some (.claimed n) →
      q.status n ≠ .notStarted → land q l = none
  | [], _, h, _ => by cases h
  | e :: rest, q, h, hq => by
    simp only [firstTouch, List.find?_cons] at h
    by_cases he : e.node = n
    · simp only [he, beq_self_eq_true, Option.some.injEq] at h
      subst h
      simp [land, stillTrue, hq]
    · have hb : (e.node == n) = false := by simpa using he
      rw [hb] at h
      simp only [land]
      split
      · exact land_stale rest (e.apply q) h (by rw [apply_other he]; exact hq)
      · rfl

theorem split_of_moved_row_is_stale {p q : Plan} {d : Desk} {n : Nat} (h : Reach p d)
    (hs : Effect.split n ∈ d.effects) (hq : q.status n ≠ .notStarted) :
    land q d.effects = none :=
  land_stale d.effects q (first_touch_is_claim h hs) hq

/-!
## 咬得动的演示

把 `split` 的握持守卫拿掉（第八个发现之前的桌子），两个从同一份计划派出的 run 都不认领就分第 1 行。落地只核认领，于是第一次落下之后第二次照样落下，第 1 行的子行长了两组。守卫在时，每个 run 都得先认领第 1 行，第二次落地按 `split_of_moved_row_is_stale` 判过时。
-/

def Desk.actWithoutHold (d : Desk) : Action → Option Desk
  | .split n => some (d.take (.split n) none)
  | a => d.act a

theorem withoutHold_splits_twice :
    let p : Plan := ⟨fun _ => .notStarted, []⟩
    let run := ((Desk.start p).actWithoutHold (.split 1)).map Desk.effects
    (run.bind fun es => (land p es).bind fun q => land q es).map Plan.divided = some [1, 1] := by
  rfl

end Collab.Claim
