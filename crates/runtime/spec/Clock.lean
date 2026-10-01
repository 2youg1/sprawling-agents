-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::clock 的模型

证明 `clock`（`crates/runtime/src/` 下同名的文件）必须守住的性质。一刻的唯一拼法、结果上的戳与它的发放规则、一跑的驱动最近读到的那一刻、按 UTC 选一段时间。runtime 的规格仍是 `crates/runtime/runtime-SPEC.md`，它的 §16 引本分部作为这些性质的权威。
-/

namespace Runtime.Clock

/-- 一件工具的结果与时间的关系（`kernel::Temporal`）：`Timestamped` 的结果离开它那一刻就说不清，`Timeless` 的结果隔多久读都一样。 -/
inductive Temporal where
  | Timestamped
  | Timeless
  deriving DecidableEq, Repr

/-- `StampGate` 的状态。`width` 是粒度的桶宽（毫秒），`none` 是 `Off`（`bucket_ms` 答 `None`）；`last_bucket` 是上一次观察落在的桶，`none` 兼任「这一跑还没有过结果」。时区表只进戳的渲染，不进这里的判定，所以模型不带它。 -/
structure StampGate where
  width : Option Nat
  last_bucket : Option Nat
  deriving DecidableEq, Repr

/-- `StampGate::observe` 里那一个布尔式：`Timestamped` 恒带戳，`Timeless` 只在桶变了时带，这一跑的第一条结果恒带。Rust 把它写成 `(… ) || last_bucket.is_none()`；这里按 `last_bucket` 先分两臂，值相同。 -/
def due (last : Option Nat) (bucket : Nat) : Temporal → Bool
  | .Timestamped => true
  | .Timeless => match last with
    | none => true
    | some seen => seen != bucket

/-- `StampGate::observe`：这一条结果带不带戳，以及观察之后的门。 -/
def observe (gate : StampGate) (now : Nat) (temporal : Temporal) : Bool × StampGate :=
  match gate.width with
  | none => (false, gate)
  | some width =>
    if due gate.last_bucket (now / width) temporal then
      (true, { gate with last_bucket := some (now / width) })
    else (false, gate)

/-- A18 零字节：`Off` 时什么都不带。 -/
theorem off_never_stamps (last : Option Nat) (now : Nat) (temporal : Temporal) :
    (observe ⟨none, last⟩ now temporal).1 = false := rfl

/-- 一跑的第一条结果恒带戳，所以开着的门不是一扇永远不开的门。 -/
theorem the_first_result_is_stamped (width now : Nat) (temporal : Temporal) :
    (observe ⟨some width, none⟩ now temporal).1 = true := by
  cases temporal <;> simp [observe, due]

/-- `Timestamped` 的结果每一条都带戳。 -/
theorem a_timestamped_result_is_always_stamped (width now : Nat) (last : Option Nat) :
    (observe ⟨some width, last⟩ now .Timestamped).1 = true := by
  simp [observe, due]

/-- 观察之后，门记下的恒是这一刻的桶，不论这一条带没带戳。 -/
theorem observing_records_the_bucket (width now : Nat) (last : Option Nat) (temporal : Temporal) :
    (observe ⟨some width, last⟩ now temporal).2 = ⟨some width, some (now / width)⟩ := by
  cases temporal with
  | Timestamped => simp [observe, due]
  | Timeless =>
    cases last with
    | none => simp [observe, due]
    | some seen =>
      by_cases same : seen = now / width
      · subst same
        simp [observe, due]
      · simp [observe, due, same]

/-- **`Timeless` 的结果一个桶里至多带一次戳。** 同一个桶里，无论前一条是哪一种、带没带戳，下一条 `Timeless` 的结果都不带。这是粒度存在的理由：频率归粒度，精度恒到秒。 -/
theorem a_timeless_result_is_stamped_at_most_once_per_bucket (width earlier later : Nat)
    (last : Option Nat) (first : Temporal) (sameBucket : later / width = earlier / width) :
    (observe (observe ⟨some width, last⟩ earlier first).2 later .Timeless).1 = false := by
  rw [observing_records_the_bucket]
  simp [observe, due, sameBucket]

/-- 桶变了，`Timeless` 的结果就带戳：门不会在一个新桶里沉默。 -/
theorem a_new_bucket_stamps_a_timeless_result (width now seen : Nat)
    (moved : seen ≠ now / width) :
    (observe ⟨some width, some seen⟩ now .Timeless).1 = true := by
  simp [observe, due, moved]

/-- 拒绝的稳定码：这里只有一种，`E_INVALID_ARGS`。 -/
inductive Code where
  | InvalidArgs
  deriving DecidableEq, Repr

/-- `clock::UtcSpan`：按 UTC 选一段时间，两端各可缺席。`until` 在 Lean 里是关键字，故写作 `«until»`。 -/
structure UtcSpan where
  since : Option Nat
  «until» : Option Nat
  deriving DecidableEq, Repr

/-- `UtcSpan::new`：`until` 不晚于 `since` 的区间里没有任何一刻，构造时拒绝；缺一端即那一端不设界。 -/
def UtcSpan.new (since «until» : Option Nat) : Except Code UtcSpan :=
  match since, «until» with
  | some from_, some to =>
    if to ≤ from_ then .error .InvalidArgs else .ok ⟨since, «until»⟩
  | some _, none => .ok ⟨since, «until»⟩
  | none, some _ => .ok ⟨since, «until»⟩
  | none, none => .ok ⟨since, «until»⟩

/-- `UtcSpan::contains`：半开区间 `[since, until)`。Rust 的参数名是 `at`，那在 Lean 里是关键字，所以这里叫 `moment`。 -/
def UtcSpan.contains (span : UtcSpan) (moment : Nat) : Bool :=
  (match span.since with
    | none => true
    | some from_ => decide (from_ ≤ moment)) &&
  (match span.«until» with
    | none => true
    | some to => decide (moment < to))

/-- 矛盾的区间在值里拒绝，而不是答出一段看起来正常的空历史。 -/
theorem a_contradictory_span_is_refused (from_ to : Nat) (backwards : to ≤ from_) :
    UtcSpan.new (some from_) (some to) = .error .InvalidArgs := by
  simp [UtcSpan.new, backwards]

/-- `Default`：两端都不设界，每一刻都在里面。 -/
theorem an_unbounded_span_contains_every_moment (moment : Nat) :
    (UtcSpan.mk none none).contains moment = true := rfl

/-- 相邻的两段不重叠：交界那一毫秒只属于后一段。 -/
theorem adjacent_spans_share_no_moment (a b c moment : Nat) :
    ¬ ((UtcSpan.mk (some a) (some b)).contains moment = true ∧
       (UtcSpan.mk (some b) (some c)).contains moment = true) := by
  simp [UtcSpan.contains]
  omega

/-- 相邻的两段不漏：落在合起来那一段里的一刻，落在两段之一里。 -/
theorem adjacent_spans_leave_no_gap (a b c moment : Nat) :
    (UtcSpan.mk (some a) (some c)).contains moment = true →
      (UtcSpan.mk (some a) (some b)).contains moment = true ∨
        (UtcSpan.mk (some b) (some c)).contains moment = true := by
  simp [UtcSpan.contains]
  omega

end Runtime.Clock
