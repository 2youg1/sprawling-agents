-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::run

规定 `run`、`run::lifecycle`、`run::charter`、`run::harness`（`crates/runtime/src/` 下同名的文件）。run 的驱动：派发、回合、冻结，run 事件序与 harness run 事件序的唯一权威。本文件是 `crates/runtime/Spec.lean` 的一个分部；下面每一节保留它在 runtime 规格里的标签 §8-n，别处引作 `crates/runtime/Spec.lean §8-n`。
-/

/-!
### 8-56 一次 run 记下它冻下的身份版本（`runtime::run`、`runtime::run::charter`，形状 2 值类型）


```rust
pub struct RunPlan { /* …既有字段… */ pub naming: Option<B3Hash> }
```

- **照录，不读。** 身份由 accounting 在冻结前缀时按 session 取定（`crates/accounting/Spec.lean` §8-15、`crates/city/Spec.lean` §8-33），名字已经在 city 段与 resident 段的字节里；本 crate 只把那一版的摘要从 `RunPlan.naming` 抄进 `run_started.naming`（`crates/kernel/Spec.lean` §8-79），由 `Charter::open` 与运行策略同一处写。`None` 是这座城的这次 run 没有冻任何身份（测试替身、早于身份入账的构造方）。
- 被否：让 runtime 读两份治理文档自己算摘要——run 开始的那一刻读到的未必是前缀冻下的那一版，账上的摘要会与请求里的名字不符。
-/

/-!
### 8-15 runtime::run（形状 5 typestate 机；**run 事件序的唯一权威**）


```rust
pub struct Run<S> { /* plan、window、turns、last_turn_t —— 全私有 */ }
pub struct Active(/* 私有 */);   pub struct Frozen { /* completion、turns */ }

pub struct RunPlan {                 // 一个 Run 的全部常量，调用方先备齐
    pub run: RunId, pub who: String, pub addr: Address,
    pub task: String, pub goal: String, pub job: Locator,
    pub opening: Opening,                                 // 这一场是接了写下来的活，还是人在场
    pub parent: Option<RunId>,                            // 派活给它的那个 Run
    pub predecessor: Option<RunId>,                       // 把这场活交给它的前任，深度守恒
    pub dispatched_by: Who,                               // 由谁派来，写进 run_started 的 dispatched_by
    pub inherited: Vec<ChatMessage>,                      // 分支开场继承的那段对话（§8-2），不是分支则空
    pub shape: CallShape,
    pub second_threshold: Option<SecondThreshold>,        // 上下文提醒的第二级（§8-34）
    pub context: ContextReading,                          // 这一跑的上下文读数，status 与提醒现读它
    pub prefix: FrozenPrefix, pub policy: BuildingPolicy, pub tools: Vec<ToolDef>,
    pub skills: Vec<SkillPin>,                            // 阅览室准进了什么，当时各是什么字节
    pub retries: Retries,                                 // 人在端点上设的重试上限（§8-9）
    pub naming: Option<B3Hash>,                           // 这次 run 的 session 冻下的身份版本（§8-56）
}
```

- **没有 `budget_turns` 与 `budget` 两栏**：回合上限与花销天花板都不存在，一跑循环到它自己结束为止；停一件正在跑的事是 `Cancel`，停一片是 `Halt`。
- **`skills` 写进 `run_started` 载荷，且无条件写**（空则空数组），理由：一个时有时无的 key 是一个读者得猜的形状，而「这栋楼一个都没准进」本身就是一件值得记下的事。进账本而不只留在进程里，是因为「它变了没有」需要一个**早一次的读取**，而进程一走就只剩账本说得出这一轮到底拿到了哪些字节。

```rust
pub enum SafePoint { BeforeAssemble{turn:u32}, BeforeCall{turn:u32}, BeforeWave{turn:u32}, BeforeSpawn{turn:u32} }
pub enum Advance { Turned, Concluded(Completion) }        // 穷尽；新结局逼每个调用方表态

pub struct RunHooks<'a> {            // 闭包，不是 trait：本模块只有一个消费者形式；invoke 除外
    pub now: &'a mut dyn FnMut() -> Result<TimeMs, AxError>,        // 时钟由调用方注入；驱动只在自己的线程、串行阶段按「时间纪律」的时点调用它
    pub interrupt: &'a mut dyn FnMut(SafePoint) -> Interrupt,       // 安全点由我定，信号由你答
    pub checkpoint: Option<&'a mut dyn FnMut(TimeMs) -> Result<Payload, AxError>>,  // 波前 checkpoint
    pub writes: &'a dyn Fn(&ToolCall) -> Writes,                    // 这条调用会不会写（§8-45）
    pub invoke: &'a mut dyn ConcurrentInvoke,   // 一波的工具，三段（§8-3）；回合时间戳随 admit 行
    pub wait: &'a mut dyn FnMut(TimeMs) -> NextCall,               // 等到 Watchdog 给的 until；途中来了 Halt 就立刻答 Halted
    pub deltas: Option<&'a mut (dyn FnMut(&Increment) + 'a)>,      // 说到一半的话往哪去（见本节末「RunHooks 的 deltas」）
}

impl Run<Active> {
    pub fn dispatch(plan: RunPlan, ledger: &mut dyn Ledger, hooks: &mut RunHooks<'_>) -> Result<Run<Active>, AxError>;
    pub fn advance(&mut self, ledger: &mut dyn Ledger, model: &mut dyn Model, hooks: &mut RunHooks<'_>) -> Result<Advance, AxError>;
    pub fn freeze(self, ledger: &mut dyn Ledger, handoff: &Handoff, completion: Completion, hooks: &mut RunHooks<'_>) -> Result<Run<Frozen>, AxError>;
}
impl Run<Frozen> { pub fn completion(&self) -> &Completion; pub fn turns(&self) -> u32; }

pub fn drive(plan: RunPlan, ledger: &mut dyn Ledger, model: &mut dyn Model,
             hooks: &mut RunHooks<'_>, handoff: &Handoff) -> Result<Run<Frozen>, AxError>;
```

- **为什么要这个模块**：「Dispatch → N 回合 → 冻结」的事件序只有这一处。citysim 与真城各写一遍就是两个权威，而两者一旦漂开，**仿真继续绿而真城错**——仿真的全部价值恰好建立在它跑的是同一份代码上。故 citysim 是本模块的调用方，每个剧本直接验证生产回路。
- **`run_started.parent`**：只在派生开的 Run 上出现。「两行相邻」不是一个可查询的事实；写进载荷之后，前端折得出树，离线重放也折得出同一棵树。
- **时间纪律**：时钟只经 `RunHooks::now` 进来，只由驱动在自己的线程上、在串行阶段调用；工具面（`ConcurrentInvoke` 的实现、装配层的 bench、citysim 的闭包）恒不采样，只收读数，`admit` 收的仍是回合时间戳。采样点：dispatch 两次（checkpoint、run_started）；每回合开头一次，得回合时间戳；每次模型尝试发出之前一次（`model_called`），回复到齐之后一次（`model_returned`）；每条工具调用放行之后、工具起跑之前一次（`tool_called`），它的答复交给工具面的 `account` 之前一次（`tool_result`），所以工具面打戳时读到的最新读数就是答复时刻；provider 失败而冻结时一次；结束时 freeze 一次，handoff 用它、run_frozen 用它＋1；取消时 freeze 沿用被打断那个回合的时间戳，因为这次冻结属于那个回合。回合里其余的行（`prompt_assembled`、`prompt_shape_compared`、`steer_received`、`cancel_received`、波前的 `checkpoint_committed`）带回合时间戳。哪几种行记自己的时刻，只由 `turn::ledger` 的 `Authored`／`Carried` 变体定一次：那四个变体不带读数就造不出来。计数器闭包（citysim）下采样的次数与次序是剧本的函数，所以字节照样可重放。这四种行的 `t` 怎么读，以 `crates/kernel/Spec.lean` §8-4「信封 `t` 记的是什么」为准。
- **开头只读段的时刻**：各条在放行之后逐条采开始；全部 join 之后按调用序逐条入账，入账前、工具面的 `account` 之前采答复，所以一条的答复时刻是「这一波在调用序上轮到它入账的时刻」，不早于它真正答完，不晚于最慢那条答完。生成中提前起跑的读，开始时刻记成它被放行的那一刻：这两行量的是这一波为它花了多久。重开参数：出现声明 `Effect::Read` 而常超过 1 s 的工具时，改在工作线程上采样，那要一个 `Sync` 的时钟。
- **结束判定**：`calls_made == 0` 且这一答**说了话**，即 `Completion::Done(Evidence[model_returned])`；`calls_made == 0` 而内容为空、或 `stop == MaxTokens`，即 `Completion::Limit`（§8-37）；任一安全点命中 Cancel 即 `Completion::Cancelled`。三条均经 `freeze` 出口，故 **handoff_written＋run_frozen 是唯一出口**，无第二条退路。第四点 `BeforeSpawn` 与前三点同权：命中即 `Cancelled`，那个回合的 assistant 与 tool results **不入窗**，因为窗口前推是「回合成立」的后果而不是它的一部分。
- **第四种结束：回合中途的失败。** **两种 carrier 都经 `freeze` 出口，差别只在冻结之前写不写载体事件**：带 `Carrier::Event` 的码先写载体事件，`Carrier::Loadtime` 的码直接冻结。理由：「账本自身就是受害者时，没有什么真实的东西可写」对 `CasCorrupt`／`StorageFatal`／`LogVersionUnsupported` 成立，对 `WireMismatch` 不成立——供应方把兑换格式写错与账本健否无关；而对前三个码，写不进去的后果就是 `freeze` 的 append 自己失败并把那个失败向上抛，这比预先判定「写不进去」更诚实。一次没有冻结的 run 在账本上只剩 `run_started`，重启后仍报 `frozen: false`，页面就把每条消息都当 `steer` 发。冻结后**原错误仍然向上抛**：账本得到判决，调用方得到诊断，两件事不互相替代。否决「把 `WireMismatch` 重分类为 `Carrier::Event(ProviderDegraded)`」：该码在握手期也用于 wire 版本不匹配（那时连 run 都不存在），一个码两种含义去改分类表，会让 `kernel::event::kind` 那条「loadtime 白名单封死在五个」的测试变成对一件无关的事作证。
- **Conversation 归驱动持有**：入窗内容就是回合报告的前推结果（assistant＋tool results），放在调用方手里等于把一条不变量交给每个调用方自己维护。
- **闭包而非 trait，`invoke` 除外**：`now`／`interrupt`／`checkpoint`／`wait` 的第二实现尚不存在，而本库的纪律是 trait 只在已有第二实现的缝上引入；`invoke` 是 §8-3 的 `ConcurrentInvoke`，它的第二实现已在缝上。`RunHooks` 自身只是引用的容器，不持策略。
-/

/-!
### 8-15-1 RunHooks 的 deltas：说到一半的话往哪去


```rust
pub deltas: Option<&'a mut (dyn FnMut(&Increment) + 'a)>,   // RunHooks 的一个字段（§8-15）
```

**`None` 是必要前提，不是缺省值。** 一个增量改变不了 run 的任何判断：`Turn::call` 按 `Generating` 三臂选门——没人看（`Unwatched`）走阻塞门，有页面在读（`Watched`）走流式门，工具面要提前起跑只读调用（`Speculating`，§8-3）走推测门。citysim 与离线重放不看增量，所以增量不碰确定性。

**它不返回 `Result`。** 增量不是判断：下游任何东西都不得据它分支，而一个能拒绝的 sink 会让一个显示细节有能力弄失败一次调用。

**写进账本的那句话只从 `ModelReturn` 来。** 增量恒不参与拼装 `model_returned` 的载荷。于是「页面看到的」与「账本保存的」不可能出自对同一个回复的两次读法；流被切断表现为读取错误，永不表现为一个变短的回答。

**`Turn::call` 的 `'sink` 是显式命名的。** 调用方（`drive`）持有 sink 跨越整个 run 并把它交给每一轮；生命周期省略时，重借需要收缩 trait object 自己的生命周期，而 `&mut` 不允许。这不是风格，是这个签名必须显式的原因。
-/

/-!
### 8-23 runtime::run 目录化


| 文件 | 管什么 |
|---|---|
| `run.rs` | 一个 Run 的常量与状态类型（`RunPlan`／`SafePoint`／`Advance`／`RunHooks`／`Active`／`Frozen`／`Run<S>`）、`impl Run<Frozen>` 的三个读法、载荷构造 `payload`，以及驱动循环 `drive`。`drive` 带 `argument_count` 豁免，故留在原路径 |
| `run/lifecycle.rs` | 一个活着的 Run 在账本上做的三件事：`dispatch` 的调度对（job pin＋run_started）、`advance` 的一回合（四个安全点、波前检查点、报告前推入窗），以及唯一出口 `freeze`（handoff_written＋run_frozen）；连同只有 `advance` 用得上的 `fold_steer`。调度对与收尾对的字节由 `run/charter.rs` 写 |
| `run/charter.rs` | 一个 run 的开篇两行与收尾两行，模型 run 与 harness run 同一个作者（§8-52） |
| `run/harness.rs` | 回合由 harness 自己走完的 run：它在账本上写的每一行，与回答冻成哪一种 `Completion`（§8-52） |

**`impl Run<Active>` 保持为一整块，不按 dispatch／advance／freeze 三分**：`cargo public-api` 按 impl 块计数，三分会让公开面输出多出四行而规范路径不变。
-/

/-!
### 8-48 回合没有上限


`RunPlan` 没有回合上限与花销上限，`drive` 是一个 `loop`。一次跑的结束只有三种来路：一回合作出结论、一次带 carrier 事件的失败（写进历史后冻结为 cancelled）、或一个安全点送到的中断。

- **理由是刹车只留一个**：没有人能在一件事跑之前给它定价，而一个替人说停的数字，停的时刻恰好是人最不希望它停的那一刻。要停一片就 `Halt`——它会终止那片里的后台成员；要停一条就 `Cancel`。
- **`Completion::Limit` 保留**：回合上限没有了，这个词却仍有一条来路——一条什么都没说的回复（§8-37）。
-/

/-!
### 8-37 一条什么都没说的回复，不是「做完了」


`Run<Active>::advance` 的收尾判定从「本回合没有工具调用」一条，改为三问：没有工具调用、**答里有内容**、且不是停在上限上——三条同时成立才是 `Completion::Done`，否则是 `Completion::Limit`。

- **理由是证据**：`Completion::Done` 必须引一条 `model_returned` 作证据，而一条 `content` 为空的 `model_returned` 证明不了任何工作完成。判它做完，等于在「什么都没说」的那一刻告诉人「你的活干完了」，并且把那条空记录作为凭据写进账本。
- **`stop` 随回合走**：`StopReason` 随 `ToolWave` 与 `Recording` 两个 typestate 到 `TurnReport::stop()`，判定读它而不是从内容去猜——一条被截断但**有内容**的回复同样不是完成，那件事只有 `stop` 说得清。
- **否决「只看内容空不空」**：`stop == MaxTokens` 而内容非空的回复是半句话，判它完成同样是假历史；只看内容会把它漏掉。
- **citysim**：剧本以显式说一句话收尾（`citysim::concluding`），因为它们要断言的是「跑到工作做完」，而不是「模型不说话了」。剧本 `a_reply_that_says_nothing_freezes_as_limit_rather_than_done` 用**供应方真实报文**（`content: []`、`stop_reason: max_tokens`）经生产翻译喂进来，钉住这条规则。
-/

/-!
### 8-52 runtime::run::charter 与 runtime::run::harness（形状 5 typestate；**harness run 事件序的唯一权威**）


```rust
// run::charter —— 一个 run 的开篇两行与收尾两行；模型 run 与 harness run 同一个作者
pub struct Charter<'a> {
    pub run: RunId, pub who: &'a str, pub addr: &'a Address,
    pub task: &'a str, pub goal: &'a str, pub job: &'a Locator,
    pub parent: Option<RunId>, pub predecessor: Option<RunId>,
    pub dispatched_by: &'a Who, pub skills: &'a [SkillPin],
}
impl RunPlan { pub fn charter(&self) -> Charter<'_>; }      // 模型 run 的那一份，从它自己的常量借出

// run::harness —— 回合由 harness 自己走完的 run
pub enum Cut { Halt, Deadline }                  // 城为什么截断这一回合：罩住房间的停摆（人取消这个 run 也是），或楼规的墙钟上限
pub struct Conclusion<'c> {
    pub committed: Payload,                      // storage::Checkpoint::wave_pre 交回的 checkpoint_committed 载荷
    pub answered: HarnessAnswered,               // 停止原因与这一回合对城说的话
    pub handoff: &'c Handoff,
}
pub struct HarnessRun<'a> { /* charter、cut —— 私有 */ }
impl<'a> HarnessRun<'a> {
    pub fn open(charter: Charter<'a>, ledger: &mut dyn Ledger,
                now: &mut dyn FnMut() -> Result<TimeMs, AxError>) -> Result<HarnessRun<'a>, AxError>;
    pub fn report(&self, ledger: &mut dyn Ledger, reported: &HarnessReported, t: TimeMs) -> Result<(), AxError>;
    pub fn cancel(&mut self, ledger: &mut dyn Ledger, cut: Cut, t: TimeMs) -> Result<(), AxError>;
    pub fn conclude(self, ledger: &mut dyn Ledger, conclusion: Conclusion<'_>, t: TimeMs) -> Result<Completion, AxError>;
    pub fn abandon(self, ledger: &mut dyn Ledger, handoff: &Handoff, t: TimeMs) -> Result<Completion, AxError>;
}
```

- **为什么要这两个模块**：`Run::dispatch` 收整份 `RunPlan`，而 `CallShape` 与 `FrozenPrefix` 是模型的事实：harness 自己选模型、自己拼上下文，给它填一份就是写下一件城不知道的事，窗口与上下文提醒还会读它。所以一个 run 的开篇两行（`checkpoint_committed` 的 `JobPinned` 与 `run_started`）与收尾两行（`handoff_written` 与 `run_frozen`）由 `Charter` 写：`Run::dispatch` 与 `Run::freeze` 经 `RunPlan::charter` 写它们，字节不变；`HarnessRun` 经同一个 `Charter` 写。run 生命周期的行仍只有本 crate 一个作者，JobPinned 在前、`run_frozen` 比 `handoff_written` 晚 1 ms 这两条也只写在一处。
- **次序是 `crates/agent_protocols/spec/Harness/Session.lean` 定的，类型守住它**：`open` → 任意条 `report` → 至多一次生效的 `cancel` → `conclude` 或 `abandon`。`conclude` 依次写 `checkpoint_committed`、`harness_answered`、`handoff_written`、`run_frozen`（`nothing_follows_the_stop_reason`：树先提交，再记回答，再冻结）；`abandon` 只写收尾两行，冻成 `Cancelled`，不记 harness 没给过的回答（`a_lost_session_freezes_cancelled_with_no_answer`）。两者都消费 `self`，冻结之后没有值能再写一行；第二次 `cancel` 什么也不写，记住的是头一次的 `Cut`（`a_second_halt_sends_nothing`）。
- **回答冻成哪一种 `Completion`，只在 `harness::ending` 一处判**（`Session.lean` 的 `ending`）：

  | 停止原因 | 头一次的截断 | 结局 |
  |---|---|---|
  | `end_turn`，回答去掉首尾空白后非空 | 任意 | `Done`，证据是刚写下的 `harness_answered` 那一行 |
  | `end_turn`，回答为空 | 任意 | `Limit` |
  | `cancelled` | `Deadline` | `Limit` |
  | `cancelled` | `Halt` 或没有 | `Cancelled` |
  | `max_tokens`、`max_turn_requests`、`refusal` | 任意 | `Limit` |

  空回答冻成 `Limit`，与 `lifecycle::concluded` 对空模型回复的判法相同。撞上墙钟上限的 run 是撞上了什么而停，所以是 `Limit`；人或停摆取消的才是 `Cancelled`（`a_deadline_never_freezes_cancelled`）。截断之后 harness 仍答出 `end_turn` 并说了话，活已做完，照 `Done` 记：账本上 `cancel_received` 在前、`harness_answered` 在后，两件事都在。
- **时间**：`open` 与 `Run::dispatch` 一样采两次 `now`（JobPinned 一次、`run_started` 一次）；其余每一行的 `t` 由调用方给，调用方在自己的线程上读同一只钟。收尾两行用给的 `t` 与 `t + 1`，`t` 已到 `u64` 顶时答 `E_INVALID_ARGS`，与 `Run::freeze` 同一句。
- **每一行的 `who` 与 `addr`**：`who` 恒为 `Charter::who`（房间的居民）；开篇、汇报、取消、检查点与回答带 `Charter::addr`；收尾两行不带地址，与 `Run::freeze` 相同。
- **载荷**：`report` 写 kernel 的 `HarnessReported`，`conclude` 写 `HarnessAnswered`（`crates/kernel/Spec.lean` §8-4，两者都是 record-only）；`cancel_received` 的载荷是空对象，与回合里的那一条相同，截断的缘由不进账本（kernel 事件表不为它加键），由冻结的 `Limit` 与 `Cancelled` 分开。检查点的载荷由调用方从 `storage::Checkpoint::wave_pre` 取来，本 crate 不开仓库，理由与 `RunHooks::checkpoint` 相同。
- **失败**：任何一行写不进账本，原错误向上抛，run 停在它写到的那一行；这与 `Run::freeze` 自己的 append 失败时相同：账本自身就是受害者时没有真实的东西可写。
-/

namespace Runtime.Run

/-- 一次 run 冻成什么（`kernel::Completion`）。`Done` 带的证据不影响下面的性质，模型不带它。 -/
inductive Completion where
  | Done
  | Limit
  | Cancelled
  deriving DecidableEq, Repr

/-- `lifecycle::concluded`：一答没有调用时冻成什么。说了话、又不是撞上输出上限停下的，是 `Done`；什么都没说的，或停在 `max_tokens` 上的，是 `Limit`（§8-37）。 -/
def concluded (said : Bool) (stoppedAtMaxTokens : Bool) : Completion :=
  if !said || stoppedAtMaxTokens then .Limit else .Done

/-- 一条空回复不是「做完了」，撞上输出上限也不是。 -/
theorem done_needs_words_and_no_ceiling (said stoppedAtMaxTokens : Bool) :
    concluded said stoppedAtMaxTokens = .Done ↔ said = true ∧ stoppedAtMaxTokens = false := by
  cases said <;> cases stoppedAtMaxTokens <;> decide

/-- `run::drive` 循环里一次 `advance` 的结局，以及它的失败分哪两类。 -/
inductive Step where
  /-- 一个回合走完，还有调用：再来一回合。 -/
  | Turned
  /-- 一个回合走完，这一答没有调用，或某个安全点命中了 Cancel。 -/
  | Concluded (completion : Completion)
  /-- 可以再问的 provider 失败：先写 `watchdog_fired`，等到 `until`，再来。 -/
  | Retried
  /-- 回合中途不能再问的失败：冻成 `Cancelled`，原错误随后向上抛。 -/
  | Failed
  deriving DecidableEq, Repr

/-- run 写下的行，按 run 的事件序关心的样子：回合内的各行是一个 `Turn`，它们的次序归 `spec/Turn.lean`。 -/
inductive Line where
  | JobPinned
  | RunStarted
  | Turn
  | WatchdogFired
  | HandoffWritten
  | RunFrozen
  deriving DecidableEq, Repr

/-- `drive` 的循环：一路写到第一个结局，结局经 `freeze` 写下收尾两行。剧本里的步数用完而还没有结局时，run 仍在跑，第二个分量是 `none`。 -/
def loop : List Step → List Line × Option Completion
  | [] => ([], none)
  | .Turned :: rest => ((.Turn :: (loop rest).1), (loop rest).2)
  | .Retried :: rest => ((.WatchdogFired :: (loop rest).1), (loop rest).2)
  | .Concluded completion :: _ => ([.Turn, .HandoffWritten, .RunFrozen], some completion)
  | .Failed :: _ => ([.HandoffWritten, .RunFrozen], some .Cancelled)

/-- `run::drive`：开篇两行（`Charter::open`），然后循环。 -/
def drive (steps : List Step) : List Line × Option Completion :=
  ([.JobPinned, .RunStarted] ++ (loop steps).1, (loop steps).2)

/-- **`handoff_written`＋`run_frozen` 是唯一出口。** 一次 run 不论以什么结局结束，账本都以这两行收尾，而 `run_frozen` 只写一次。 -/
theorem freeze_is_the_only_exit :
    ∀ (steps : List Step), (loop steps).2.isSome = true →
      ∃ before, (loop steps).1 = before ++ [.HandoffWritten, .RunFrozen] ∧ .RunFrozen ∉ before
  | [], ended => by simp [loop] at ended
  | .Turned :: rest, ended => by
    obtain ⟨before, ends, once⟩ := freeze_is_the_only_exit rest (by simpa [loop] using ended)
    exact ⟨.Turn :: before, by simp [loop, ends], by simp [once]⟩
  | .Retried :: rest, ended => by
    obtain ⟨before, ends, once⟩ := freeze_is_the_only_exit rest (by simpa [loop] using ended)
    exact ⟨.WatchdogFired :: before, by simp [loop, ends], by simp [once]⟩
  | .Concluded _ :: _, _ => ⟨[.Turn], rfl, by decide⟩
  | .Failed :: _, _ => ⟨[], rfl, by decide⟩

/-- 开篇两行恒在最前：`JobPinned` 在 `run_started` 之前。 -/
theorem the_job_is_pinned_before_the_run_starts (steps : List Step) :
    (drive steps).1.take 2 = [.JobPinned, .RunStarted] := by
  simp [drive]

/-- 拒绝的稳定码：这里只有一种，`E_INVALID_ARGS`。 -/
inductive Code where
  | InvalidArgs
  deriving DecidableEq, Repr

/-- `TimeMs` 能装下的最大一刻。 -/
def TIME_MAX : Nat := 2 ^ 64 - 1

/-- `Charter::close`：`handoff_written` 记 `t`，`run_frozen` 记 `t + 1`，两行记的是同一件事；`t` 已到顶时拒绝，而不是回绕。 -/
def close (t : Nat) : Except Code (Nat × Nat) :=
  if t < TIME_MAX then .ok (t, t + 1) else .error .InvalidArgs

/-- 收尾两行相隔一毫秒，`run_frozen` 在后。 -/
theorem run_frozen_follows_its_handoff_by_one_millisecond (t handoff frozen : Nat)
    (closed : close t = .ok (handoff, frozen)) : handoff = t ∧ frozen = handoff + 1 := by
  unfold close at closed
  split at closed
  · injection closed with same
    injection same with first second
    subst first
    subst second
    exact ⟨rfl, rfl⟩
  · cases closed

/-- 时钟到顶时冻结拒绝，`run_frozen` 不会记成比 `handoff_written` 更早的一刻。 -/
theorem a_clock_at_its_ceiling_cannot_close : close TIME_MAX = .error .InvalidArgs := by
  simp [close]

end Runtime.Run

/-! D7 定规：run 的开篇与收尾归 `Charter`，harness run 不借 `RunPlan`

**决定**：开篇两行与收尾两行由 `run::charter::Charter` 写，`Run::dispatch`、`Run::freeze` 与 `run::harness::HarnessRun` 都经它；`Charter` 是从 `RunPlan` 借出的视图（`RunPlan::charter`），harness 一侧从它自己的值借出同一个形状。harness run 的行序与结局判定住 `run::harness`，不住装配层。

**理由**：harness run 与模型 run 的开篇、收尾是同一件事：同样的 JobPinned、同样的 `run_started` 载荷、同样的冻结对与 1 ms 间隔。两处各写一遍，`run_started` 多一个键时（例如 `mode`）两份会各改各的。结局判定放在本 crate，是因为 `lifecycle::concluded` 已经是「一答冻成什么」的家，空回答冻成 `Limit` 这一条两边必须同一个读法。

**被否**：①给 harness 填一份 `RunPlan`：`CallShape` 与 `FrozenPrefix` 是城没有的事实，窗口、提醒与 `transcript` 都会读它们；②装配层直接 append 这些行：run 生命周期的行就有了两个作者，JobPinned 的次序与冻结对的 `t`/`t + 1` 规则各写一份；③把 `RunPlan` 拆成 `Charter` 加模型部分：每个构造 `RunPlan` 的地方（citysim、装配层、测试）与每个读 `plan.run`／`plan.who` 的地方都要改，借出的视图给出同一条映射而不动它们。

**重开参数**：`RunPlan` 的开篇字段与 `Charter` 的字段不再一一对应（例如 `run_started` 多一个只有模型 run 才有的键），视图要带 `Option` 时，改为 `RunPlan` 持有一个 `Charter` 值。
-/
