-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::run::checkpoint

规定 `run::checkpoint`（`crates/runtime/src/` 下同名的文件）。一波之前立不立 checkpoint 的唯一权威。本文件是 `crates/runtime/Spec.lean` 的一个分部；下面每一节保留它在 runtime 规格里的标签 §8-n，别处引作 `crates/runtime/Spec.lean §8-n`。
-/

/-!
### 8-45 runtime::run::checkpoint（形状 1 判定；**一波前立不立 checkpoint 的唯一权威**）


```rust
pub(crate) enum WaveCheckpoint { Skip, Stage }
pub(crate) enum Wave { Empty, ReadOnly, MayWrite }  // 由 RunHooks::writes 逐个调用读出
enum SinceCheckpoint { NotYet, Checkpointed, Changed }   // 相对本 run 上一次 checkpoint 的树
pub(crate) struct CheckpointPolicy { since: SinceCheckpoint }
impl CheckpointPolicy {
    pub(crate) fn opening() -> Self;                                   // NotYet
    pub(crate) fn for_wave(&self, wave: Wave) -> WaveCheckpoint;                // 只读
    pub(crate) fn record_wave(&mut self, checkpoint: WaveCheckpoint, wave: Wave);    // 只改状态
}
// RunHooks 上：
pub writes: &'a dyn Fn(&ToolCall) -> kernel::Writes;   // 按声明的 Effect 答（Writes::of），未注册的名字答 Domain
```

- **checkpoint 做两件事**：一是给这一波可能删改的东西留一个能回退的提交；二是把上一波写下的文件带进一个提交——否则那些写既进不了 diff，也还原不回来。所以判定看两样：这一波要调用什么，以及上一次 checkpoint 之后有没有调用跑过。
- **波的分类**：没有调用 → `Empty`；每个调用的 `RunHooks::writes` 都答 `Nothing` → `ReadOnly`；否则 `MayWrite`。
- **判定表**：`Changed`（上次 checkpoint 后跑过可能写的调用）→ `Stage`，空波与只读波也一样；`NotYet` 且 `MayWrite` → `Stage`；`NotYet` 且 `Empty`／`ReadOnly` → `Skip`（树就是 run 开张时那棵）；`Checkpointed`（checkpoint 之后没有可能写的调用跑过）→ `Skip`，上一个提交已经是这棵树。
- **状态转移**：`MayWrite` → `Changed`（被取消打断的波也算，它的部分调用可能已经跑了）；`Empty`／`ReadOnly` 且立了 checkpoint → `Checkpointed`；`Empty`／`ReadOnly` 且跳过 → 不变。只读波不改树，所以它既不需要自己的 checkpoint，也不让下一波的 checkpoint 多出一次提交。`Run<Active>` 持一个 `CheckpointPolicy`，`advance` 只在 `Stage` 时调用 `RunHooks::checkpoint` 并写 `checkpoint_committed`。
- **为什么是一个模块**：「这一波要不要 checkpoint」是一个判定，后面两条规则（只读波、按写过的路径 stage）都只改这一处。
- **`Stage` 带什么由调用方定**：`RunHooks::checkpoint` 仍只收时刻；stage 哪些路径，由持有 `ToolBench` 的一侧决定，因为只有 bench 知道每个调用的工具。`ToolBench::invoke` 在 `BenchOutcome::Ran.wrote` 里交回工具的 `Tool::writes`（`crates/kernel/Spec.lean` §8-23 的 `Writes`）；sprawling 的 lane 把上次 checkpoint 以来各调用的 `wrote` 用 `Writes::and` 并起来，下一次 checkpoint 只 stage 这些路径，并在 checkpoint 后清零。run 的第一次 checkpoint、以及并出来是 `Domain` 或 `Nothing` 的那次，stage 整个写域：第一次之前的树没有任何本 run 的提交担保；`Nothing` 出现在 run 的第一道 checkpoint：那时还没有调用跑过。**失败的调用并入 `Domain`**：`ToolBench::invoke` 答 `Err` 时没有 `wrote`，而工具可能写到一半才失败，它自己对写了什么的说法不再可信；lane 于是把 `Domain` 并进去，下一次 checkpoint stage 整个写域。只丢掉它、留下同波其他调用的 `Paths`，会让那半截写不进任何提交。**被否**：`RunHooks::checkpoint` 收一个范围参数——run 驱动拿不到工具的 `Effect`，这个参数只能由 lane 填，等于把同一个并集在两层各拼一次。
- **调用可能不可能写，由 `RunHooks::writes` 答**：`ToolDef` 只有名字、描述与 schema，`Effect` 住 `ToolBench` 的注册表里，所以 lane 在把 bench 借给 `invoke` 之前取出 `ToolBench::declared_writes`（名字到 `Writes::of(effect)` 的表），`writes` 查这张表。它按声明答，不按参数答：判定发生在波跑之前，而 `Tool::writes` 读的是一条跑完的调用。**被否**：`RunPlan` 带名字到 `Effect` 的表——`RunPlan` 是冻结的 run 描述，进账本的重放读它，而工具的 `Effect` 是 bench 注册时的事实，不该在两处各记一份。
- **否决「空波一律跳过」**：结束回合的空波前那次 checkpoint，是把上一波的写带进提交的唯一时机；跳过它，run 写下的文件就没有任何提交持有。
- **否决「每波都 checkpoint」**：一个没跑过任何调用的 run，提交的是一棵没变的树，却多付一次 stage 与 commit。
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
