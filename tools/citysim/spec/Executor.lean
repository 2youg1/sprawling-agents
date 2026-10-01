-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# citysim::executor

规定 `citysim::executor`（`tools/citysim/src/executor.rs`），以及它交给 `runtime::run::drive` 的两个剧本适配器 `citysim::script_model`、`citysim::script_tools` 与筛子的世界 `citysim::sieving`。本文件是 `tools/citysim/Spec.lean` 的一个分部；下面每一节保留它在 citysim 规格里的标签 §8-n，别处引作 `tools/citysim/Spec.lean §8-n`。

确定性是这里的事（D1）：一次剧本执行重放时逐字节相同，因为剧本是写死的、时钟是一个按问询计数的计数器、执行是单线程，三者都不读外界。能写成定理的是其中三件：计数时钟交出的读数只取决于被问了几次（`the_clock_hands_out_its_count`）；场景只在回合边界说话，取消压过 steer（D7）；一次工具调用的键取每 run 一个的位次，所以同一 run 里两次调用的键永远不同，而按一波的时刻取键会把两次同名调用并成一次（D20）。剧本模型按序作答、用尽之后恒以空调用作结（`the_script_is_answered_in_order`）。回合本身怎样走是 runtime 的模型（`crates/runtime/spec/Turn.lean`、`crates/runtime/spec/Run.lean`），本文件不重述。
-/

/-!
### 8-2 剧本适配器与薄执行器

```rust
pub struct ScriptModel { /* VecDeque<ModelReturn> */ }
impl ScriptModel { pub fn new(script: Vec<ModelReturn>) -> Self; }
impl kernel::Model for ScriptModel { /* 逐次弹出；耗尽后恒回空 calls（自然收束） */ }

pub struct ScriptTool { /* meta、outcomes: VecDeque<Result<ToolOutcome, AxError>> */ }   // impl kernel::Tool
pub struct ScriptToolSet { /* BTreeMap<String, ScriptTool> */ }
                     // 未知工具名 → E_TOOL_UNKNOWN＋nearby＝已注册名；耗尽脚本 → E_TOOL_UNAVAILABLE

pub enum CancelPoint { BeforeAssemble { turn: u32 }, BeforeCall { turn: u32 }, BeforeWave { turn: u32 } }
pub struct Scenario { pub run: RunId, pub who: String, pub addr: Address, pub task: String,
                      pub goal: String, pub job_md: String, pub model: ScriptModel,
                      pub bench: ToolBench, pub config: FrozenConfig,
                      pub checkpoint: Option<(Checkpoint, Vec<String>)>,
                      pub cancel: Option<CancelPoint>, pub steer: Option<(u32, String)>,
                      pub sieve: Option<SieveWorld> }
                      // run/who 由剧本注入（citysim 无随机）；job_md 是 JOB.md 内容，假 oid 由其哈希派生
pub struct ScenarioReport { pub lines: Vec<Vec<u8>>, pub completion: &'static str /* Completion::name() */ }
/// Deterministic: t 单调递增每步 +1ms，无时钟采样；无随机。
pub fn run_scenario(scenario: Scenario) -> Result<ScenarioReport, AxError>;
/// 在一本已有历史的账本上续跑：两个 run 同链，seq 按一座城写下的次序排。
pub fn run_scenario_on(ledger: &mut MemLedger, scenario: Scenario) -> Result<ScenarioReport, AxError>;
```

- 事件序（无取消正常收束）：`checkpoint_committed`（JOB.md 先落）→ `run_started` → `prompt_assembled`（每 run 一条，`crates/runtime/Spec.lean` §8-39 第 5 条）→ 每回合 `prompt_shape_compared→model_called→model_returned[→tool_called→tool_result]*` → 空 calls 回合后 `handoff_written` → `run_frozen{completion:done, evidence:[末 model_returned]}`。
- 取消在指定边界注入 `Interrupt::Cancel`（D7）：事件序断言是 `cancel_received` 后无新 `model_called`／`tool_called`，且 `handoff_written` 恒先于 `run_frozen`，三个边界各一条剧本。
- `steer` 在给定回合的波边界递一句人话：它追加到下一个结果里，不打断正在进行的动作，故剧本断言循环照常继续。同一边界上取消压过 steer（`cancel_wins_over_steer`）。
- 回合数没有上限：驱动器跑到模型回空 calls 或被取消为止（`runtime::run::drive`）。
- 一个空的剧本模型在第一次调用就什么也不说（`ScriptModel::silent`），run 以 `Completion::Limit` 冻结；一个要走到工作结尾的剧本用 `concluding` 写下它最后那句话，而不是让剧本用尽。
-/

/-!
### 8-3 真适配器换入（单 Resident 全链走完一条回路）

剧本仍确定性（无时钟采样、无网络、无随机），四个替换点接的是产品本身：

| 替换点 | 接的是 |
|---|---|
| Ledger | MemLedger（真 jsonl 对拍已在 conformance；换盘不增新证据） |
| 工具面 | 真 `ToolBench`（edit、status、exec 的 Program 臂）对 tempdir 城根；门路由在回合层 |
| 模型 | `ScriptModel` 登录 wire 形：剧本写 Anthropic wire JSON，经 `gateway::dialect::response_from_wire` 解成 canonical 再出 ModelReturn（翻译面进链路） |
| 时钟／配置 | 逐步 +1ms，加 `FrozenConfig` 求值（clock_stamp 三层覆盖）接入 `StampGate` |

门路由归 `ToolBench`，写域住 bench 内，executor 不手写 domain 门。`ScriptToolSet` 是 `kernel::tool` 缝的第二适配器（已登记的 conformance 证据），**注册进真 ToolBench**，于是脚本工具与真 L0 工具走同一条门路由。波前的检查点是**每波一次**而非只在 exec forecast 命中时：`ToolBench` 内的 forecast 检查点是它在 exec 臂上的加强，两者不互相替代。空波仍提交（同树 oid），因为链可重建优于省一次提交。tool_result 的信封由 executor 挂（`pipeline::package` 加 `StampGate`），与 serve 同位；场景把 `exec` 的结果交给筛子时（`sieve` 为 `Some`），信封由 `sieving::package_exec` 经城自己的筛子挂。

事件序断言：edit 成功的波携 `checkpoint_committed`（波前，断言形：每个 `tool_called` 之前最近的 `checkpoint_committed` 晚于最近的 `model_returned`）；tool_result 信封可携 ClockStamp（非 Off 时）；越域写被 domain 门拒且 refusal 以 tool_result 回流；链恒可验；双跑字节对拍。gateway 进 sim 的只有 dialect 翻译面（纯函数，确定性保持）；endpoint 的 HTTP 面不入 sim（网络即非确定），其验证住 gateway 自身的回环假服务测试（`crates/gateway/Spec.lean` §2）。
-/

/-!
### 8-4 一次工具调用的键：每跑一个的位次，加上整个动作的字节

```rust
// citysim::executor——驱动块内
let placed = Cell::new(0u64);              // 位次：每跑一个计数器，与 accounting::worker 同形
let key = IdemKey::derive(&run, Seq::new(at), &call.action()?);   // 动作字节：kernel::tool 唯一一份
```

**动作字节取 `ToolCall::action`**（`crates/kernel/Spec.lean` §8-23，全库唯一一份），**位次取每跑一个的计数器**，与 `accounting::worker` 同形，故两个驱动器对「一次工具调用的键怎么算」只有一份读法（D20）。

`two_reads_in_one_wave_are_two_calls` 钉住这一条：一个回合携两次同名、参数不同的调用，两条 `tool_result` 都带结果、都不带 `error`。`IdemKey` 不进任何 payload，故账本字节与 `golden-p0` 不受它影响。
-/

namespace Citysim.Executor

/-! #### 计数时钟 -/

/-- 场景时钟：下一次被问时交出的读数（`run_scenario_on` 里的 `tick`）。 -/
structure Clock where
  tick : Nat
  deriving DecidableEq, Repr

/-- 被问一次：交出当前的计数并加一（`now`）。Rust 在 `u64` 溢出时以 `E_INVALID_ARGS` 拒，模型的 `Nat` 没有这一档（§3）。 -/
def Clock.now (clock : Clock) : Nat × Clock :=
  (clock.tick, ⟨clock.tick + 1⟩)

/-- 被问 `asks` 次，依次交出的读数与之后的时钟。 -/
def Clock.ask (clock : Clock) : Nat → List Nat × Clock
  | 0 => ([], clock)
  | asks + 1 =>
    let (readings, after) := clock.ask asks
    let (reading, last) := after.now
    (readings ++ [reading], last)

/-- 计数时钟交出的读数只取决于它被问了几次：从 `start` 起被问 `asks` 次，读数是 `start, start+1, …`。驱动器问时钟的次序是剧本定的，所以同一剧本两次执行读到同一串时刻（D1）。 -/
theorem the_clock_hands_out_its_count (start asks : Nat) :
    (Clock.ask ⟨start⟩ asks) = ((List.range asks).map (start + ·), ⟨start + asks⟩) := by
  induction asks with
  | zero => rfl
  | succ asks ih =>
    simp [Clock.ask, ih, Clock.now, List.range_succ]
    omega

/-- 相邻两次读数严格递增，差一毫秒：一次 run 里没有两件事落在同一个时刻上，除非驱动器只问了一次。 -/
theorem each_reading_is_one_past_the_last (start asks index : Nat) (inside : index + 1 < asks) :
    ((Clock.ask ⟨start⟩ asks).1)[index + 1]? = ((Clock.ask ⟨start⟩ asks).1)[index]?.map (· + 1) := by
  rw [the_clock_hands_out_its_count]
  simp [inside, Nat.lt_of_succ_lt inside]
  omega

/-! #### 场景只在回合边界说话 -/

/-- 剧本写取消的地方，按回合编号（`executor::CancelPoint`）。 -/
inductive CancelPoint where
  | BeforeAssemble (turn : Nat)
  | BeforeCall (turn : Nat)
  | BeforeWave (turn : Nat)
  deriving DecidableEq, Repr

/-- 驱动器问中断的地方（`runtime::run::SafePoint`），五个变体照抄它的形状。 -/
inductive SafePoint where
  | BeforeAssemble (turn : Nat)
  | BeforeCall (turn : Nat)
  | BeforeWave (turn : Nat)
  | BeforeToolCall (turn call : Nat)
  | BeforeSpawn (turn : Nat)
  deriving DecidableEq, Repr

/-- 场景在一个边界上的回答（`runtime::turn::Interrupt` 里场景用得到的三个）。 -/
inductive Interrupt where
  | None
  | Cancel
  | Steer (text : String)
  deriving DecidableEq, Repr

/-- D7 **场景只在回合边界取消。** `CancelPoint` 三个变体（`BeforeAssemble`、`BeforeCall`、`BeforeWave`，各带 `turn`）对应 `runtime::run::SafePoint` 里按回合编号的三个；`SafePoint` 另有 `BeforeToolCall` 与 `BeforeSpawn`，它们在一个回合里被问很多次，剧本按回合编号够不到它们，执行器在这两处恒答 `Interrupt::None`（`executor::answer_at`）。同一边界上取消压过 steer：停下与改道是相反的指令，而停下是等也等不回来的那一个。被击败的备选：给 `CancelPoint` 补齐五个变体——那要给回合内的每一次问询编号，而剧本只写回合。**重开参数**：一条剧本需要在一波中途取消时，`CancelPoint` 加一个带调用序号的变体。 -/
def answer_at (point : SafePoint) (cancel : Option CancelPoint) (steer : Option (Nat × String)) :
    Interrupt :=
  let addressed : Option (CancelPoint × Nat) :=
    match point with
    | .BeforeAssemble turn => some (.BeforeAssemble turn, turn)
    | .BeforeCall turn => some (.BeforeCall turn, turn)
    | .BeforeWave turn => some (.BeforeWave turn, turn)
    | .BeforeToolCall _ _ => none
    | .BeforeSpawn _ => none
  match addressed with
  | none => .None
  | some (here, turn) =>
    if cancel = some here then .Cancel
    else
      match point, steer with
      | .BeforeWave _, some (at_, text) => if at_ = turn then .Steer text else .None
      | _, _ => .None

/-- 一个回合里被问很多次的两个边界上，场景什么都不说。 -/
theorem inside_a_turn_the_scenario_says_nothing (turn call : Nat) (cancel : Option CancelPoint)
    (steer : Option (Nat × String)) :
    answer_at (.BeforeToolCall turn call) cancel steer = .None
      ∧ answer_at (.BeforeSpawn turn) cancel steer = .None := by
  simp [answer_at]

/-- 剧本写下的每一个取消点都在它那个边界上被答成取消，steer 写在哪里都一样：三个边界各一条剧本是可写的。 -/
theorem every_cancel_point_is_reached (cancel : CancelPoint) (steer : Option (Nat × String)) :
    ∃ point, answer_at point (some cancel) steer = .Cancel := by
  cases cancel with
  | BeforeAssemble turn => exact ⟨.BeforeAssemble turn, by simp [answer_at]⟩
  | BeforeCall turn => exact ⟨.BeforeCall turn, by simp [answer_at]⟩
  | BeforeWave turn => exact ⟨.BeforeWave turn, by simp [answer_at]⟩

/-- 同一边界上取消压过 steer。 -/
theorem cancel_wins_over_steer (turn : Nat) (text : String) :
    answer_at (.BeforeWave turn) (some (.BeforeWave turn)) (some (turn, text)) = .Cancel := by
  simp [answer_at]

/-- steer 只在它那一回合的波边界上递到，别处都听不到。 -/
theorem a_steer_is_heard_only_at_its_wave (point : SafePoint) (cancel : Option CancelPoint)
    (steer : Option (Nat × String)) (text : String) (heard : answer_at point cancel steer = .Steer text) :
    ∃ turn, point = .BeforeWave turn ∧ steer = some (turn, text) := by
  cases point with
  | BeforeAssemble turn =>
    by_cases hit : cancel = some (.BeforeAssemble turn) <;> simp [answer_at, hit] at heard
  | BeforeCall turn =>
    by_cases hit : cancel = some (.BeforeCall turn) <;> simp [answer_at, hit] at heard
  | BeforeWave turn =>
    by_cases hit : cancel = some (.BeforeWave turn)
    · simp [answer_at, hit] at heard
    · cases steer with
      | none => simp [answer_at, hit] at heard
      | some given =>
        obtain ⟨at_, said⟩ := given
        by_cases same : at_ = turn
        · subst same
          simp only [answer_at, hit, if_false, if_true, Interrupt.Steer.injEq] at heard
          exact ⟨at_, rfl, by rw [heard]⟩
        · simp [answer_at, hit, same] at heard
  | BeforeToolCall turn call => simp [answer_at] at heard
  | BeforeSpawn turn => simp [answer_at] at heard

/-! #### 一次工具调用的键 -/

/-- `IdemKey::derive` 的三样输入：哪个 run、第几次调用、整个动作的字节（`ToolCall::action`）。摘要函数本身是 kernel 的；两把键相同只可能在输入相同时，这是对摘要函数的假设，不在本模型里。 -/
structure KeyInput where
  run : Nat
  placed : Nat
  action : String
  deriving DecidableEq, Repr

/-- 一个 run 从第 `next` 次起依次调用 `actions`，每次调用的键输入（`invoke` 里的 `placed` 计数器）。 -/
def keyed (run : Nat) : Nat → List String → List KeyInput
  | _, [] => []
  | next, action :: rest => ⟨run, next, action⟩ :: keyed run (next + 1) rest

theorem keyed_places_from (run next : Nat) (actions : List String) :
    ∀ key ∈ keyed run next actions, next ≤ key.placed := by
  induction actions generalizing next with
  | nil => simp [keyed]
  | cons action rest ih =>
    intro key member
    simp only [keyed, List.mem_cons] at member
    rcases member with rfl | later
    · exact Nat.le_refl _
    · exact Nat.le_of_succ_le (ih (next + 1) key later)

/-- D20 **一次工具调用的键：每跑一个的位次，加上整个动作的字节。** 动作字节取 `ToolCall::action`（`crates/kernel/Spec.lean` §8-23，全库唯一一份），位次取每跑一个的计数器，与 `accounting::worker` 同形，故两个驱动器对「一次工具调用的键怎么算」只有一份读法。被击败的读法：位次取回合时钟 `t`、动作字节只取工具名（`byInstant`）。`runtime::run` 给一波里的每次调用同一个 `t`（一波是一个瞬间），于是一波之内两次同名调用拿到相同的 `IdemKey`，`ToolBench::invoke` 的去重把第二次判成重复，参数不同也不救；而且那个 `Seq` 是钟读数换了个类型，违反确定性规则「位次不从时钟来」（ARCHITECTURE.md §10 第 7 条）。

同一个 run 里，没有两次调用的键输入相同，无论它们的动作是什么、落在哪一波。 -/
theorem no_two_calls_of_a_run_share_a_key (run next : Nat) (actions : List String) :
    (keyed run next actions).Nodup := by
  induction actions generalizing next with
  | nil => simp [keyed]
  | cons action rest ih =>
    simp only [keyed, List.nodup_cons]
    refine ⟨?_, ih (next + 1)⟩
    intro member
    exact absurd (keyed_places_from run (next + 1) rest _ member) (Nat.not_succ_le_self next)

/-- 一次工具调用：工具名与参数。 -/
structure Call where
  name : String
  args : String
  deriving DecidableEq, Repr

/-- 被击败的读法：位次取一波的时刻 `instant`，动作只取工具名。 -/
def byInstant (run instant : Nat) (call : Call) : KeyInput :=
  ⟨run, instant, call.name⟩

/-- 那种读法下，一波之内两次同名的调用拿到同一把键，无论参数怎样不同，第二次会被判成重复。 -/
theorem keying_by_the_instant_merges_two_calls_of_one_wave (run instant : Nat) (one other : Call)
    (same_name : one.name = other.name) :
    byInstant run instant one = byInstant run instant other := by
  simp [byInstant, same_name]

/-! #### 剧本模型 -/

/-- 剧本模型的一次调用（`ScriptModel::call`）：弹出下一条；用尽之后答 `none`，Rust 里是一条空调用、空正文的回复，run 读它作结。 -/
def call : List α → Option α × List α
  | [] => (none, [])
  | next :: rest => (some next, rest)

/-- 连续调用 `count` 次得到的回答。 -/
def answers : List α → Nat → List (Option α)
  | _, 0 => []
  | script, count + 1 => (call script).1 :: answers (call script).2 count

/-- 剧本按写下的次序作答，一条不跳、一条不重。 -/
theorem the_script_is_answered_in_order (script : List α) :
    answers script script.length = script.map some := by
  induction script with
  | nil => rfl
  | cons next rest ih => simp [answers, call, ih]

/-- 用尽的剧本之后每一次调用都作结，而不是重复最后一条。 -/
theorem an_exhausted_script_concludes (count : Nat) :
    answers ([] : List α) count = List.replicate count none := by
  induction count with
  | zero => rfl
  | succ count ih => simp [answers, call, ih, List.replicate_succ]

end Citysim.Executor
