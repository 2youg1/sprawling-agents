-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::run::checkpoint 的模型

证明 `run::checkpoint`（`crates/runtime/src/` 下同名的文件）必须守住的性质。一波之前立不立 checkpoint 的唯一权威。runtime 的规格仍是 `crates/runtime/runtime-SPEC.md`，它的 §16 引本分部作为这些性质的权威。
-/

namespace Runtime.Run.Checkpoint

/-- 一波之前立不立 checkpoint 的答案（`run::checkpoint::WaveCheckpoint`）。 -/
inductive WaveCheckpoint where
  | Skip
  | Stage
  deriving DecidableEq, Repr

/-- 一波按它的调用会不会写分成三类（`run::checkpoint::Wave`）：`Wave::of` 逐个调用问 `RunHooks::writes`，没有调用是 `Empty`，每个调用都答 `Writes::Nothing` 是 `ReadOnly`，其余是 `MayWrite`。 -/
inductive Wave where
  | Empty
  | ReadOnly
  | MayWrite
  deriving DecidableEq, Repr

/-- 相对本 run 上一次 checkpoint 的树（`run::checkpoint::SinceCheckpoint`）。 -/
inductive SinceCheckpoint where
  | NotYet
  | Checkpointed
  | Changed
  deriving DecidableEq, Repr

/-- `CheckpointPolicy::for_wave`，§8-45 的判定表，一臂一行。 -/
def for_wave : SinceCheckpoint → Wave → WaveCheckpoint
  | .Changed, _ => .Stage
  | .NotYet, .MayWrite => .Stage
  | .NotYet, .Empty => .Skip
  | .NotYet, .ReadOnly => .Skip
  | .Checkpointed, _ => .Skip

/-- `CheckpointPolicy::record_wave`，§8-45 的状态转移：可能写的一波之后树就变了，被取消打断的那一波也算；只读与空的一波只在立了 checkpoint 时把状态推到 `Checkpointed`。 -/
def record_wave (since : SinceCheckpoint) : WaveCheckpoint → Wave → SinceCheckpoint
  | _, .MayWrite => .Changed
  | .Stage, .Empty => .Checkpointed
  | .Stage, .ReadOnly => .Checkpointed
  | .Skip, .Empty => since
  | .Skip, .ReadOnly => since

/-- run 的循环在每一波之前做的事（`run::lifecycle` 的 `advance`）：先问 `for_wave`，再把答案与这一波交给 `record_wave`。 -/
def step (since : SinceCheckpoint) (wave : Wave) : SinceCheckpoint :=
  record_wave since (for_wave since wave) wave

/-- 树的真相，按最坏情形算：可能写的一波确实改了树，只读与空的一波一个字节都不动。`ran`：上一次提交之后（还没有提交时，run 开张之后）跑过可能写的一波；`committed`：本 run 立过至少一次 checkpoint。策略只有三态，这里有四种真相，下面的定理说三态足以作答。 -/
structure Truth where
  ran : Bool
  committed : Bool
  deriving DecidableEq, Repr

/-- run 开张时的真相：什么都没跑，什么都没提交。 -/
def Truth.opening : Truth := ⟨false, false⟩

/-- 一波之后的真相：先立的 checkpoint 提交这一刻的树，再由这一波决定树变没变。 -/
def Truth.after (truth : Truth) (checkpoint : WaveCheckpoint) (wave : Wave) : Truth :=
  let committed : Truth := match checkpoint with
    | .Stage => ⟨false, true⟩
    | .Skip => truth
  match wave with
  | .MayWrite => { committed with ran := true }
  | .Empty => committed
  | .ReadOnly => committed

/-- 策略的一态是不是这份真相的忠实抽象。 -/
def tracks : SinceCheckpoint → Truth → Bool
  | .NotYet, truth => !truth.ran && !truth.committed
  | .Checkpointed, truth => !truth.ran && truth.committed
  | .Changed, truth => truth.ran

/-- 一波之后，抽象仍然忠实。 -/
theorem a_wave_keeps_the_policy_faithful (since : SinceCheckpoint) (ran committed : Bool)
    (wave : Wave) :
    tracks since ⟨ran, committed⟩ = true →
      tracks (step since wave) (Truth.after ⟨ran, committed⟩ (for_wave since wave) wave) = true := by
  cases since <;> cases wave <;> cases ran <;> cases committed <;> decide

/-- 从 `since` 与 `truth` 起，依次走过 `waves`。 -/
def walk : SinceCheckpoint → Truth → List Wave → SinceCheckpoint × Truth
  | since, truth, [] => (since, truth)
  | since, truth, wave :: rest =>
    walk (step since wave) (truth.after (for_wave since wave) wave) rest

/-- 任意多波之后，抽象仍然忠实。 -/
theorem any_waves_keep_the_policy_faithful :
    ∀ (waves : List Wave) (since : SinceCheckpoint) (truth : Truth),
      tracks since truth = true →
        tracks (walk since truth waves).1 (walk since truth waves).2 = true
  | [], _, _, faithful => faithful
  | wave :: rest, since, ⟨ran, committed⟩, faithful =>
    any_waves_keep_the_policy_faithful rest _ _
      (a_wave_keeps_the_policy_faithful since ran committed wave faithful)

/-- **判定表恰好在该立的时候立。** 策略忠实于真相时，它立 checkpoint 当且仅当上一次提交之后跑过可能写的一波（不立，那些写就没有任何提交持有），或者 run 还没有提交过而这一波可能写（不立，这一波删改的东西就没有提交可以回去）。别的时候都不立：提交一棵与上一个提交相同的树只多付一次 stage 与 commit。 -/
theorem stages_exactly_when_needed (since : SinceCheckpoint) (ran committed : Bool) (wave : Wave) :
    tracks since ⟨ran, committed⟩ = true →
      (for_wave since wave = .Stage ↔ (ran = true ∨ (committed = false ∧ wave = .MayWrite))) := by
  cases since <;> cases wave <;> cases ran <;> cases committed <;> decide

/-- 从 run 开张起任意多波之后，下一波的判定仍是上面那条。 -/
theorem every_wave_stages_exactly_when_needed (waves : List Wave) (wave : Wave) :
    let reached := walk .NotYet Truth.opening waves
    for_wave reached.1 wave = .Stage ↔
      (reached.2.ran = true ∨ (reached.2.committed = false ∧ wave = .MayWrite)) := by
  intro reached
  have faithful := any_waves_keep_the_policy_faithful waves .NotYet Truth.opening rfl
  exact stages_exactly_when_needed reached.1 reached.2.ran reached.2.committed wave faithful

/-- 被否的「空波一律跳过」会丢掉写：一个可能写的波之后，结束回合的空波前那次 checkpoint 是把这些写带进提交的唯一时机，判定表在那里立。 -/
theorem the_closing_empty_wave_commits_the_last_writes :
    for_wave (step .NotYet .MayWrite) .Empty = .Stage := by
  decide

/-- 被否的「每波都 checkpoint」多付一次提交：一个只跑过只读波的 run，树仍是开张时那棵，判定表不立。 -/
theorem a_read_only_run_commits_nothing (waves : List Wave)
    (readOnly : ∀ wave ∈ waves, wave ≠ .MayWrite) :
    walk .NotYet Truth.opening waves = (.NotYet, Truth.opening) := by
  induction waves with
  | nil => rfl
  | cons wave rest ih =>
    have notWrite : wave ≠ .MayWrite := readOnly wave (by simp)
    have restReadOnly : ∀ w ∈ rest, w ≠ .MayWrite :=
      fun w member => readOnly w (by simp [member])
    cases wave with
    | MayWrite => exact absurd rfl notWrite
    | Empty => exact ih restReadOnly
    | ReadOnly => exact ih restReadOnly

end Runtime.Run.Checkpoint
