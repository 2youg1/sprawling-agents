-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# 一个回合要几道落盘屏障：只读工具不在执行前等落盘

规定 `crates/runtime/src/turn/wave.rs` 与 `crates/runtime/src/turn/ledger.rs` 把一个回合的记录交给 `kernel::Ledger` 的时机，以及 `crates/accounting/src/worker/relay.rs` 为这些记录付的屏障（`crates/runtime/Spec.lean` §8-3，`crates/sprawling/Spec.lean` §8-42-2）。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。

一次屏障的价钱与它带几条记录几乎无关（`kernel::Ledger::append_all` 的文档：一条一屏障约 585 µs，五十条一屏障每条约 13 µs）。所以要省的是屏障的次数，而不是记录的条数。

**参照次序**是 ARCHITECTURE §5 第 4 步「Every effect becomes an event first」的逐条读法：每条记录追加即落盘，一次追加一道屏障。一个回合于是付 `4 + 2 × 调用数` 道屏障（run 自己的两行 `prompt_shape_compared` 与 `checkpoint_committed`，`model_called`、`model_returned`，每条调用的 `tool_called` 与 `tool_result`）。

**TF1 次序**（D1）只在没有对外效果的地方省屏障：
* 只读调用（`kernel::Effect::Read`）追加 `tool_called` 后不等它落盘就执行，`tool_result` 同样只追加；
* 写调用追加 `tool_called` 后先过一道屏障再动手——意图先落盘，再动手；它的 `tool_result` 搭下一道屏障；
* 下一个对外可见的效果（下一次模型调用、一次写、run 的冻结）之前一道屏障，把此前追加的全部记录一起落盘。

于是一个回合付 `1 + 写调用数` 道屏障；一波最多一条写调用时不超过 2。

五组定理，全部对任意回合序列、任意崩溃点成立：
* **对外效果之前，此前追加的每条记录都已落盘**（`tf1_effect_after_durability`）；
* **写调用的意图先落盘**（`tf1_write_intent_durable`）；
* **记录按调用序**——TF1 与参照追加的记录逐条相同，落盘的总是这串记录的前缀（`tf1_records_match_reference`、`tf1_durable_is_reference_prefix`）；所以崩溃后 `resume` 读到的，是参照次序在某个崩溃点也会留下的历史：缺的只有只读调用的记录，而只读调用没有对外效果，重做它们不改变世界；写调用的 `tool_called` 已落盘，`replay::DanglingCalls` 照旧把没有结果的那条补成 `E_TOOL_OUTCOME_UNKNOWN`；
* **`EventRef` 只给已落盘的记录**（`refs_are_durable`）；
* **屏障数**（`tf1_turn_barriers`、`reference_turn_barriers`、`tf1_turn_at_most_two`）。

崩溃点写成「轨迹 = 已做 ++ 未做」：`exec State.empty done` 是在 `done` 之后掉电时的状态，`durable` 是重启后还在盘上的记录。只读调用的执行在模型里排成一列，因为它们没有对外效果、不改变状态，所以它们实际并行时彼此怎样交错不影响任何一条性质。
-/

namespace Runtime.Turn.Durability

/-- 一条工具调用会不会改变回合之外的世界（`kernel::Effect`：`Read` 对 `read`，其余对 `write`）。 -/
inductive Effect where
  | read
  | write
  deriving Repr, DecidableEq

/-- 一条工具调用：第几回合的第几条。 -/
abbrev CallId := Nat × Nat

/-- 回合写下的记录，连同 run 在回合的相与相之间写的两行：组装之后的 `prompt_shape_compared`（`shapeCompared`），回答之后、工具波之前的 `checkpoint_committed`（`checkpointCommitted`）。 -/
inductive Record where
  | shapeCompared (turn : Nat)
  | modelCalled (turn : Nat)
  | modelReturned (turn : Nat)
  | checkpointCommitted (turn : Nat)
  | toolCalled (call : CallId)
  | toolResult (call : CallId)
  deriving Repr, DecidableEq

/-- 回合的一步：追加一条记录（未落盘），一道屏障，执行一条工具，或一个对外边界（模型调用、run 冻结）。 -/
inductive Step where
  | append (record : Record)
  | barrier
  | act (call : CallId) (effect : Effect)
  | boundary
  deriving Repr, DecidableEq

/-- 回合之外的人或程序看得见的一步。 -/
def Step.visible : Step → Bool
  | .act _ .write => true
  | .boundary => true
  | .append _ => false
  | .barrier => false
  | .act _ .read => false

/-- 已落盘的、已追加未落盘的、已交出 `EventRef` 的记录，以及已经发生的对外效果。 -/
structure State where
  durable : List Record
  pending : List Record
  refs : List Record
  effects : List Step

def State.empty : State := ⟨[], [], [], []⟩

/-- 一道屏障把排队的记录一起落盘，并且只在此刻为它们交出 `EventRef`。 -/
def State.step (s : State) : Step → State
  | .append r => { s with pending := s.pending ++ [r] }
  | .barrier => { s with durable := s.durable ++ s.pending, refs := s.refs ++ s.pending, pending := [] }
  | .act c .write => { s with effects := s.effects ++ [.act c .write] }
  | .boundary => { s with effects := s.effects ++ [.boundary] }
  | .act _ .read => s

def exec (s : State) (steps : List Step) : State := steps.foldl State.step s

def Step.appended : Step → Option Record
  | .append r => some r
  | .barrier => none
  | .act _ _ => none
  | .boundary => none

/-- 一段轨迹按次序追加的记录；账本给它们的 seq 就是这个次序。 -/
def appended (steps : List Step) : List Record := steps.filterMap Step.appended

@[simp] theorem appended_append (r : Record) (xs : List Step) :
    appended (.append r :: xs) = r :: appended xs := rfl
@[simp] theorem appended_barrier (xs : List Step) : appended (.barrier :: xs) = appended xs := rfl
@[simp] theorem appended_act (c : CallId) (e : Effect) (xs : List Step) :
    appended (.act c e :: xs) = appended xs := rfl
@[simp] theorem appended_boundary (xs : List Step) : appended (.boundary :: xs) = appended xs := rfl

/-! ## 两种次序 -/

/-- D1：只读调用不等 `tool_called` 落盘；写调用在动手前过一道屏障。 -/
def tf1Call (c : CallId) : Effect → List Step
  | .read => [.append (.toolCalled c), .act c .read, .append (.toolResult c)]
  | .write => [.append (.toolCalled c), .barrier, .act c .write, .append (.toolResult c)]

/-- 参照次序：每条记录追加即落盘。 -/
def refCall (c : CallId) (e : Effect) : List Step :=
  [.append (.toolCalled c), .barrier, .act c e, .append (.toolResult c), .barrier]

def wave (call : CallId → Effect → List Step) (t : Nat) : Nat → List Effect → List Step
  | _, [] => []
  | i, e :: es => call (t, i) e ++ wave call t (i + 1) es

/-- TF1 的一个回合：run 的 `prompt_shape_compared` 与 `model_called` 一起落盘后才调模型；`model_returned`、run 的 `checkpoint_committed` 与只读调用的记录搭下一道屏障。检查点本身（把写域提交成一个 commit）不是对外边界：掉电时它只留下一个账本没有提到的 commit，世界里谁也看不见它，重启后的回合会另提一个。 -/
def tf1Turn (t : Nat) (calls : List Effect) : List Step :=
  [.append (.shapeCompared t), .append (.modelCalled t), .barrier, .boundary,
    .append (.modelReturned t), .append (.checkpointCommitted t)] ++ wave tf1Call t 0 calls

def refTurn (t : Nat) (calls : List Effect) : List Step :=
  [.append (.shapeCompared t), .barrier, .append (.modelCalled t), .barrier, .boundary,
    .append (.modelReturned t), .barrier, .append (.checkpointCommitted t), .barrier] ++
    wave refCall t 0 calls

/-- 一个 run：各回合依次，最后在冻结（对外边界）之前一道屏障。 -/
def run (turn : Nat → List Effect → List Step) : Nat → List (List Effect) → List Step
  | _, [] => [.barrier, .boundary]
  | t, w :: ws => turn t w ++ run turn (t + 1) ws

/-! ## 对任意轨迹成立的两条引理 -/

/-- 落盘的与排队的，合起来恰是追加过的，次序不变。 -/
theorem accounted (s : State) (steps : List Step) :
    (exec s steps).durable ++ (exec s steps).pending = s.durable ++ s.pending ++ appended steps := by
  induction steps generalizing s with
  | nil => simp [exec, appended]
  | cons x xs ih =>
    have h := ih (s.step x)
    simp only [exec, List.foldl_cons] at h ⊢
    rw [h]
    cases x with
    | append r => simp [State.step]
    | barrier => simp [State.step]
    | act c e => cases e <;> simp [State.step]
    | boundary => simp [State.step]

/-- `EventRef` 只在屏障处交出，所以交出过 ref 的记录恰是已落盘的记录。 -/
theorem refs_are_durable (s : State) (steps : List Step) (h : s.refs = s.durable) :
    (exec s steps).refs = (exec s steps).durable := by
  induction steps generalizing s with
  | nil => simpa [exec] using h
  | cons x xs ih =>
    apply ih
    cases x with
    | append r => simpa [State.step] using h
    | barrier => simp [State.step, h]
    | act c e => cases e <;> simpa [State.step] using h
    | boundary => simpa [State.step] using h

/-- 任一崩溃点，盘上的记录是整段轨迹追加记录的前缀。 -/
theorem durable_prefix (done rest : List Step) :
    (exec State.empty done).durable <+: appended (done ++ rest) := by
  have h := accounted State.empty done
  simp only [State.empty, List.nil_append] at h
  have hd : (exec State.empty done).durable <+: appended done := ⟨_, h⟩
  have hr : appended done <+: appended (done ++ rest) := by
    simp [appended, List.filterMap_append]
  exact hd.trans hr

/-! ## 对外效果之前没有排队的记录 -/

/-- 沿轨迹数排队的记录；遇到对外效果时队列必须为空。 -/
def guarded : Nat → List Step → Bool
  | _, [] => true
  | p, .append _ :: rest => guarded (p + 1) rest
  | _, .barrier :: rest => guarded 0 rest
  | p, .act _ .read :: rest => guarded p rest
  | p, .act _ .write :: rest => p == 0 && guarded p rest
  | p, .boundary :: rest => p == 0 && guarded p rest

def pendingAfter : Nat → List Step → Nat
  | p, [] => p
  | p, .append _ :: rest => pendingAfter (p + 1) rest
  | _, .barrier :: rest => pendingAfter 0 rest
  | p, .act _ _ :: rest => pendingAfter p rest
  | p, .boundary :: rest => pendingAfter p rest

theorem guarded_append (p : Nat) (a b : List Step) :
    guarded p (a ++ b) = (guarded p a && guarded (pendingAfter p a) b) := by
  induction a generalizing p with
  | nil => simp [guarded, pendingAfter]
  | cons x xs ih =>
    cases x with
    | append r => simp [guarded, pendingAfter, ih]
    | barrier => simp [guarded, pendingAfter, ih]
    | act c e => cases e <;> simp [guarded, pendingAfter, ih, Bool.and_assoc]
    | boundary => simp [guarded, pendingAfter, ih, Bool.and_assoc]

/-- 守住 `guarded` 的轨迹上，每个对外效果发生时没有未落盘的记录。 -/
theorem guarded_visible (s : State) (done rest : List Step) (v : Step)
    (hg : guarded s.pending.length (done ++ v :: rest) = true) (hv : v.visible = true) :
    (exec s done).pending = [] := by
  induction done generalizing s with
  | nil =>
    simp only [exec, List.foldl_nil, List.nil_append] at hg ⊢
    cases v with
    | append r => simp [Step.visible] at hv
    | barrier => simp [Step.visible] at hv
    | act c e =>
      cases e with
      | read => simp [Step.visible] at hv
      | write => simp [guarded] at hg; exact hg.1
    | boundary => simp [guarded] at hg; exact hg.1
  | cons x xs ih =>
    apply ih (s.step x)
    cases x with
    | append r => simpa [State.step, guarded] using hg
    | barrier => simpa [State.step, guarded] using hg
    | act c e =>
      cases e with
      | read => simpa [State.step, guarded] using hg
      | write =>
        simp [guarded] at hg
        simpa [State.step, hg.1] using hg.2
    | boundary =>
      simp [guarded] at hg
      simpa [State.step, hg.1] using hg.2

theorem tf1Call_guarded (p : Nat) (c : CallId) (e : Effect) : guarded p (tf1Call c e) = true := by
  cases e <;> simp [tf1Call, guarded]

theorem tf1_wave_guarded (t i p : Nat) (calls : List Effect) :
    guarded p (wave tf1Call t i calls) = true := by
  induction calls generalizing i p with
  | nil => simp [wave, guarded]
  | cons e es ih => simp [wave, guarded_append, tf1Call_guarded, ih]

theorem tf1_run_guarded (t p : Nat) (ws : List (List Effect)) :
    guarded p (run tf1Turn t ws) = true := by
  induction ws generalizing t p with
  | nil => simp [run, guarded]
  | cons w ws ih => simp [run, tf1Turn, guarded_append, guarded, tf1_wave_guarded, ih]

/-- **对外效果之前，此前追加的每条记录都已落盘**：模型调用、写与冻结看到的历史，就是重启后的历史。 -/
theorem tf1_effect_after_durability (ws : List (List Effect)) (done rest : List Step) (v : Step)
    (h : done ++ v :: rest = run tf1Turn 0 ws) (hv : v.visible = true) :
    (exec State.empty done).pending = [] ∧ (exec State.empty done).durable = appended done := by
  have hg : guarded State.empty.pending.length (done ++ v :: rest) = true := by
    rw [h]; exact tf1_run_guarded 0 0 ws
  have hp := guarded_visible State.empty done rest v hg hv
  refine ⟨hp, ?_⟩
  have ha := accounted State.empty done
  rw [hp, List.append_nil] at ha
  simpa [State.empty] using ha

/-! ## 写调用的意图先落盘 -/

def Step.intents : Step → List CallId
  | .append (.toolCalled c) => [c]
  | .append (.shapeCompared _) => []
  | .append (.modelCalled _) => []
  | .append (.modelReturned _) => []
  | .append (.checkpointCommitted _) => []
  | .append (.toolResult _) => []
  | .barrier => []
  | .act _ _ => []
  | .boundary => []

def Step.writes : Step → List CallId
  | .act c .write => [c]
  | .act _ .read => []
  | .append _ => []
  | .barrier => []
  | .boundary => []

/-- 每条写在它之前已经追加过自己的 `tool_called`。 -/
def intentFirst (seen : List CallId) : List Step → Bool
  | [] => true
  | x :: rest => x.writes.all (fun c => decide (c ∈ seen)) && intentFirst (x.intents ++ seen) rest

def seenAfter (seen : List CallId) : List Step → List CallId
  | [] => seen
  | x :: rest => seenAfter (x.intents ++ seen) rest

theorem intentFirst_append (seen : List CallId) (a b : List Step) :
    intentFirst seen (a ++ b) = (intentFirst seen a && intentFirst (seenAfter seen a) b) := by
  induction a generalizing seen with
  | nil => simp [intentFirst, seenAfter]
  | cons x xs ih => simp [intentFirst, seenAfter, ih, Bool.and_assoc]

theorem tf1Call_intentFirst (seen : List CallId) (c : CallId) (e : Effect) :
    intentFirst seen (tf1Call c e) = true := by
  cases e <;> simp [tf1Call, intentFirst, Step.writes, Step.intents]

theorem tf1_wave_intent_first (t i : Nat) (seen : List CallId) (calls : List Effect) :
    intentFirst seen (wave tf1Call t i calls) = true := by
  induction calls generalizing i seen with
  | nil => simp [wave, intentFirst]
  | cons e es ih => simp [wave, intentFirst_append, tf1Call_intentFirst, ih]

theorem tf1_run_intentFirst (t : Nat) (seen : List CallId) (ws : List (List Effect)) :
    intentFirst seen (run tf1Turn t ws) = true := by
  induction ws generalizing t seen with
  | nil => simp [run, intentFirst, Step.writes, Step.intents]
  | cons w ws ih =>
    simp [run, tf1Turn, intentFirst_append, intentFirst, Step.writes, Step.intents,
      tf1_wave_intent_first, ih]

theorem intent_before_write (seen : List CallId) (done rest : List Step) (c : CallId)
    (h : intentFirst seen (done ++ .act c .write :: rest) = true) :
    c ∈ seen ∨ c ∈ done.flatMap Step.intents := by
  induction done generalizing seen with
  | nil =>
    simp [intentFirst, Step.writes] at h
    exact Or.inl h.1
  | cons x xs ih =>
    simp only [List.cons_append, intentFirst, Bool.and_eq_true] at h
    rcases ih (x.intents ++ seen) h.2 with hs | hd
    · rcases List.mem_append.mp hs with hx | hs'
      · exact Or.inr (by simp [List.flatMap_cons, hx])
      · exact Or.inl hs'
    · exact Or.inr (by simp [List.flatMap_cons, hd])

theorem intent_appended (done : List Step) (c : CallId) (h : c ∈ done.flatMap Step.intents) :
    Record.toolCalled c ∈ appended done := by
  induction done with
  | nil => simp at h
  | cons x xs ih =>
    simp only [List.flatMap_cons, List.mem_append] at h
    rcases h with hx | hs
    · cases x with
      | append r =>
        cases r <;> simp [Step.intents] at hx
        simp [appended, Step.appended, hx]
      | barrier => simp [Step.intents] at hx
      | act _ _ => simp [Step.intents] at hx
      | boundary => simp [Step.intents] at hx
    · have := ih hs
      cases x <;> simp_all [appended, Step.appended]

/-- **写调用的意图先落盘**：任一条写动手时，它的 `tool_called` 已在盘上。 -/
theorem tf1_write_intent_durable (ws : List (List Effect)) (done rest : List Step) (c : CallId)
    (h : done ++ .act c .write :: rest = run tf1Turn 0 ws) :
    Record.toolCalled c ∈ (exec State.empty done).durable := by
  have ⟨_, hd⟩ := tf1_effect_after_durability ws done rest (.act c .write) h rfl
  have hi : intentFirst [] (done ++ .act c .write :: rest) = true := by
    rw [h]; exact tf1_run_intentFirst 0 [] ws
  rcases intent_before_write [] done rest c hi with hn | hm
  · simp at hn
  · rw [hd]; exact intent_appended done c hm

/-! ## 记录次序与参照相同 -/

theorem call_records (c : CallId) (e : Effect) : appended (tf1Call c e) = appended (refCall c e) := by
  cases e <;> rfl

theorem wave_records (t i : Nat) (calls : List Effect) :
    appended (wave tf1Call t i calls) = appended (wave refCall t i calls) := by
  induction calls generalizing i with
  | nil => simp [wave]
  | cons e es ih =>
    simp only [wave, appended, List.filterMap_append] at ih ⊢
    rw [ih]
    have := call_records (t, i) e
    simp only [appended] at this
    rw [this]

/-- **TF1 与参照追加的记录逐条相同**：seq 次序即调用次序，与今天一样。 -/
theorem tf1_records_match_reference (t : Nat) (ws : List (List Effect)) :
    appended (run tf1Turn t ws) = appended (run refTurn t ws) := by
  induction ws generalizing t with
  | nil => rfl
  | cons w ws ih =>
    simp only [run, tf1Turn, refTurn, appended, List.filterMap_append] at ih ⊢
    rw [ih]
    have := wave_records t 0 w
    simp only [appended] at this
    rw [this]
    rfl

/-- 任一崩溃点，TF1 盘上的记录是参照次序追加记录的前缀：`resume` 面对的是参照也会留下的历史。 -/
theorem tf1_durable_is_reference_prefix (ws : List (List Effect)) (done rest : List Step)
    (h : done ++ rest = run tf1Turn 0 ws) :
    (exec State.empty done).durable <+: appended (run refTurn 0 ws) := by
  have := durable_prefix done rest
  rw [h, tf1_records_match_reference] at this
  exact this

/-! ## 屏障数 -/

def barriers (steps : List Step) : Nat := steps.countP (fun s => decide (s = .barrier))

def writes (calls : List Effect) : Nat := calls.countP (fun e => decide (e = .write))

theorem tf1_wave_barriers (t i : Nat) (calls : List Effect) :
    barriers (wave tf1Call t i calls) = writes calls := by
  induction calls generalizing i with
  | nil => simp [wave, barriers, writes]
  | cons e es ih =>
    simp only [barriers, writes] at ih ⊢
    cases e <;> simp [wave, tf1Call, ih] <;> omega

theorem ref_wave_barriers (t i : Nat) (calls : List Effect) :
    barriers (wave refCall t i calls) = 2 * calls.length := by
  induction calls generalizing i with
  | nil => simp [wave, barriers]
  | cons e es ih =>
    simp only [barriers] at ih ⊢
    simp [wave, refCall, ih]
    omega

/-- TF1 的一个回合付 `1 + 写调用数` 道屏障。 -/
theorem tf1_turn_barriers (t : Nat) (calls : List Effect) :
    barriers (tf1Turn t calls) = 1 + writes calls := by
  have := tf1_wave_barriers t 0 calls
  simp only [barriers] at this ⊢
  simp [tf1Turn, this]
  omega

/-- 参照次序的一个回合付 `4 + 2 × 调用数` 道屏障。 -/
theorem reference_turn_barriers (t : Nat) (calls : List Effect) :
    barriers (refTurn t calls) = 4 + 2 * calls.length := by
  have := ref_wave_barriers t 0 calls
  simp only [barriers] at this ⊢
  simp [refTurn, this]
  omega

/-- 一波最多一条写调用时，TF1 的一个回合不超过两道屏障。 -/
theorem tf1_turn_at_most_two (t : Nat) (calls : List Effect) (h : writes calls ≤ 1) :
    barriers (tf1Turn t calls) ≤ 2 := by
  rw [tf1_turn_barriers]; omega

/-! ## 回合收尾付一道屏障（Rust 今天的形状）

`runtime::turn` 的 `Journal` 只活一个回合：run 自己的两行经回合的账本门（`Turn::hold_run_line`）追加，攒下的记录于是跨过组装、调用、工具波这几个相，但不跨回合——回合收尾（`record` 与取消的 `Journal::close`）付一道屏障，因为 `TurnReport` 交出的每个 ref 都必须指向已落盘的记录。run 的第一回合多攒一条 `prompt_assembled`，它搭 `model_called` 那道屏障，不多付。下面证明前三组性质对这个形状原样成立，屏障数是 `2 + 写调用数`，只读调用一道也不加。收尾那道是 `tf1Turn` 的 `1 + 写调用数` 之外多出的一道：要省掉它，攒下的记录得跨过回合，而 `TurnReport` 的 ref 得改成不必全部已落盘。 -/

/-- Rust 的回合：TF1 的一个回合，再加收尾的一道屏障。 -/
def closedTurn (t : Nat) (calls : List Effect) : List Step :=
  tf1Turn t calls ++ [.barrier]

theorem closed_run_guarded (t p : Nat) (ws : List (List Effect)) :
    guarded p (run closedTurn t ws) = true := by
  induction ws generalizing t p with
  | nil => simp [run, guarded]
  | cons w ws ih =>
    simp [run, closedTurn, tf1Turn, guarded_append, guarded, tf1_wave_guarded, ih]

/-- 收尾屏障之下，对外效果之前此前追加的每条记录仍都已落盘。 -/
theorem closed_effect_after_durability (ws : List (List Effect)) (done rest : List Step) (v : Step)
    (h : done ++ v :: rest = run closedTurn 0 ws) (hv : v.visible = true) :
    (exec State.empty done).pending = [] ∧ (exec State.empty done).durable = appended done := by
  have hg : guarded State.empty.pending.length (done ++ v :: rest) = true := by
    rw [h]; exact closed_run_guarded 0 0 ws
  have hp := guarded_visible State.empty done rest v hg hv
  refine ⟨hp, ?_⟩
  have ha := accounted State.empty done
  rw [hp, List.append_nil] at ha
  simpa [State.empty] using ha

theorem closed_run_intentFirst (t : Nat) (seen : List CallId) (ws : List (List Effect)) :
    intentFirst seen (run closedTurn t ws) = true := by
  induction ws generalizing t seen with
  | nil => simp [run, intentFirst, Step.writes, Step.intents]
  | cons w ws ih =>
    simp [run, closedTurn, tf1Turn, intentFirst_append, intentFirst, Step.writes, Step.intents,
      tf1_wave_intent_first, ih]

/-- 收尾屏障之下，任一条写动手时它的 `tool_called` 仍已在盘上。 -/
theorem closed_write_intent_durable (ws : List (List Effect)) (done rest : List Step) (c : CallId)
    (h : done ++ .act c .write :: rest = run closedTurn 0 ws) :
    Record.toolCalled c ∈ (exec State.empty done).durable := by
  have ⟨_, hd⟩ := closed_effect_after_durability ws done rest (.act c .write) h rfl
  have hi : intentFirst [] (done ++ .act c .write :: rest) = true := by
    rw [h]; exact closed_run_intentFirst 0 [] ws
  rcases intent_before_write [] done rest c hi with hn | hm
  · simp at hn
  · rw [hd]; exact intent_appended done c hm

theorem closed_records_match_reference (t : Nat) (ws : List (List Effect)) :
    appended (run closedTurn t ws) = appended (run refTurn t ws) := by
  induction ws generalizing t with
  | nil => rfl
  | cons w ws ih =>
    simp only [run, closedTurn, tf1Turn, refTurn, appended, List.filterMap_append] at ih ⊢
    rw [ih]
    have := wave_records t 0 w
    simp only [appended] at this
    rw [this]
    first
      | rfl
      | simp [Step.appended, List.filterMap_cons]

/-- 收尾屏障之下，盘上的记录仍是参照次序追加记录的前缀。 -/
theorem closed_durable_is_reference_prefix (ws : List (List Effect)) (done rest : List Step)
    (h : done ++ rest = run closedTurn 0 ws) :
    (exec State.empty done).durable <+: appended (run refTurn 0 ws) := by
  have := durable_prefix done rest
  rw [h, closed_records_match_reference] at this
  exact this

/-- Rust 的一个回合付 `2 + 写调用数` 道屏障：`model_called` 一道，每条写一道，收尾一道。 -/
theorem closed_turn_barriers (t : Nat) (calls : List Effect) :
    barriers (closedTurn t calls) = 2 + writes calls := by
  have := tf1_turn_barriers t calls
  simp only [barriers] at this ⊢
  simp [closedTurn, this]
  omega

end Runtime.Turn.Durability
