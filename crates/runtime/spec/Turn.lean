-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::turn

规定 `turn`、`turn::boundary`、`turn::report`、`turn::wave`、`turn::wave::reorder`、`turn::ledger`、`turn::prompt`（`crates/runtime/src/` 下同名的文件）。回合的 typestate 四相、四个取消点、工具波的入账次序与回合的账本门。本文件是 `crates/runtime/Spec.lean` 的一个分部；下面每一节保留它在 runtime 规格里的标签 §8-n，别处引作 `crates/runtime/Spec.lean §8-n`。
-/

/-!
### 8-3 runtime::turn（形状 5 typestate 机）＋bench／conversation


**一个 typestate 机、一张工作台、一份会话历史，三种形状三个模块。**
- `bench`（形状 1 判定）：`ToolBench`／`BenchOutcome` 与三条必要前提次序（去重先于副作用；exec 的 discard 预报先于 Write 门；Deny 以 `tool_result` 回去而不结束回合）。它拥有的是**次序**；工具本身以 `Box<dyn Tool>` 递入，沙盒在缝上，副作用不归它。
- `conversation`（形状 2 值）：`Conversation`，以及再导出的 `Opening`（定义住 kernel，因为 `run_started` 要带它，`crates/kernel/Spec.lean` §8-82-1）。它是会话，不是 transcript（冻结后写下的逐 run 文件），也不是 `kernel::Window`（上下文大小）。一条不变量在每一个入口上成立——**连续的 user 内容并进已开的那条消息，而不另开一条**；steer、工具结果与开场任务是同一条规则的三扇门。
- 根重导出：`runtime::Opening` 在根上；`ToolBench` 只经 `runtime::bench`。

```rust
pub struct Turn<'h, S> { /* journal（run、who、t、钟、refs、redacted）、state —— 全私有；相内数据在别的相不可表示 */ }
pub struct Assembling(/* 私有 */);  pub struct Calling { /* prefix 哈希 */ }
pub struct ToolWave { /* calls */ }   pub struct Recording { /* refs */ }

pub enum Interrupt { None, Cancel, Steer { source: String, text: String } }
pub enum PhaseOutcome<Next> { Advanced(Next), Cancelled(TurnCancelled) }
// PhaseOutcome 刻意穷尽：新结局必须逼每个执行器表态，不得掉 catch-all（全库枚举皆闭）。
pub struct TurnCancelled { /* refs：含 cancel_received —— 私有，getter 取 */ }
pub struct TurnReport { /* refs、model_returned_ref、wave_len —— getter 取 */ }

impl<'h> Turn<'h, Assembling> {
    pub fn begin(run: RunId, who: String, t: TimeMs, now: &'h mut dyn FnMut() -> Result<TimeMs, AxError>)
        -> Turn<'h, Assembling>;   // t 是回合时间戳；now 只给四种等来的时刻采样
    /// Boundary 1 (组装前). Cancel here consumes before any model bytes.
    pub fn assemble<'c>(self, interrupt: Interrupt, ledger: &mut dyn Ledger, prompt: RunPrompt<'_>, …)
        -> Result<PhaseOutcome<Turn<'h, Calling>>, AxError>;      // 每 run 一条 prompt_assembled（§8-39 第 5 条）
}
impl Turn<'_, Calling> {
    /// Boundary 2 (provider 调用前).
    pub fn call(self, interrupt: Interrupt, ledger: &mut dyn Ledger, model: &mut dyn Model,
                policy: &BuildingPolicy, generating: Generating<'_, '_>)
        -> Result<PhaseOutcome<Turn<'_, ToolWave>>, AxError>;     // 产 model_called＋model_returned
}
/// 模型还在生成时，谁据它的回答行动：走哪扇门由这里一处定。
/// 生产路径（run::lifecycle）每回合都传 Speculating，没有页面在看时 deltas 为 None；
/// call_speculating 的默认实现落到 call_streaming，所以没人看的 run 也走流，ModelCall 的 streamed 为真，
/// 流式失败的重发修复对它同样适用。Unwatched 与 Watched 今天只有测试调用。
pub enum Generating<'a, 'sink> {
    Unwatched,                                            // 阻塞门 Model::call
    Watched(&'a mut (dyn FnMut(&Increment) + 'sink)),     // 流式门 call_streaming，增量给页面
    Speculating { deltas: Option<&'a mut (dyn FnMut(&Increment) + 'sink)>,
                  tools: &'a dyn ConcurrentInvoke },      // 推测门 call_speculating，见下「生成中起跑」
}
impl Turn<ToolWave> {
    /// Boundary 3 (工具执行前)；per call tool_called + tool_result，按调用序入账。
    /// 开头连续的 Effect::Read 调用同时执行，结果进重排缓冲；
    /// 第一条非只读调用等它们全部收齐后再开始，此后串行。
    /// still_going 在每条 call 之前被问一次（见 §8-28-2）。
    /// 一条入账失败即返回，排在它后面的已放行只读调用不入账：`admit` 对 Effect::Read
    /// 不写去重表、taint 与检查点，所以它们没有留下账本不知道的状态。
    pub fn execute_concurrent(self, interrupt: Interrupt, ledger: &mut dyn Ledger,
                              tools: &mut dyn ConcurrentInvoke,
                              still_going: &mut dyn FnMut(u32) -> Interrupt)
        -> Result<PhaseOutcome<Turn<Recording>>, AxError>;
}
pub trait ConcurrentInvoke {          // 在 crate 根重导出：runtime::ConcurrentInvoke／runtime::Admitted
    fn effect_of(&self, call: &ToolCall) -> Option<Effect>;   // None＝目录不认识的工具，按非只读处理
    fn ahead(&self, call: &ToolCall) -> Option<&dyn Tool> { None }  // 未放行就借出的只读工具；None＝不提前起跑
    fn admit(&mut self, call: &ToolCall, t: TimeMs) -> Admitted;          // 一段：按调用序串行放行
    fn tool(&self, ticket: &bench::Ticket) -> Result<&dyn Tool, AxError>; // 二段：借出工具，多线程调
    fn account(&mut self, call: &ToolCall, ticket: bench::Ticket,
               answered: Result<ToolOutcome, AxError>) -> Result<ToolOutcome, AxError>;  // 三段：按调用序串行记账
}
pub enum Admitted { Answered(Result<ToolOutcome, AxError>), Cleared(bench::Ticket) }
impl<F: FnMut(&ToolCall, TimeMs) -> Result<ToolOutcome, AxError>> ConcurrentInvoke for F { /* 放行即作答；不报效果，故整波串行 */ }
pub enum NextCall { Allowed, Halted }   // RunHooks::wait 的答案：等待中只有「停」可以立刻执行
impl Turn<Recording> {
    pub fn record(self, interrupt: Interrupt, ledger: &mut dyn Ledger) -> Result<PhaseOutcome<TurnReport>, AxError>;
}
```

- **相变函数携 `&mut dyn Ledger`，相内字段私有**；无返回既往相的方法；跳相／相内取消／字面量构造中间相，三者编译不过（trybuild）。
- **取消只在边界**：每相变函数首参即边界快照；命中 Cancel → 追加 cancel_received → 返回 Cancelled（回合终止，后续 handoff_written＋run_frozen 归执行器）。相内无任何中断入口＝A9 的结构化一半；另一半（事件序断言）在 citysim。**工具波内的每条 call 同样是一道边界**：一波是 N 件副作用而不是一件，于是 `execute_concurrent` 在每条 call 之前问 `still_going`，答案交给同一个 `consume_boundary`：Cancel 写下同一条 `cancel_received`，Steer 写下同一条 `steer_received`——消费中断的地方仍然只有一处。
- **四取消点**：组装前／provider 调用前／工具执行前／派生前，四点全住本模块。第四点由 `Turn<Recording>::record` 收边界快照，故 `record` 与前三相同形——收 `Interrupt`、答 `PhaseOutcome`。它买到的是别处买不到的一件事：**一个回合把活派下去之后、子 Run 起来之前，仍停得住**；`calls_made == 0` 的收尾回合尤其如此，那一刻在第四点之前根本没有下一个边界。
- **model_called 载荷**：segments 哈希（与 prompt_assembled 同源）；model_returned 载荷＝message＋calls 数。
- **前缀冻结是运行时不变量，不只是测试。** `assemble` 走 `prefix.verified_segment_hashes()?`：从 `bytes()` 重算四段哈希并与构造时记录的对拍，不等即 `E_CAS_CORRUPT` **拒绝**（不是警告），恢复语指名一条走得通的路——换一个地址派这件活（§8-4-1）；`call` 在写 `model_called` 之前对 `chat.system` 的四块做同一断言（`prefix::verified_system_hashes`），哈希不等或某块丢掉断点同拒。**两处都接在既有的每回合摘要上，不另起记录点**；离线口径同一条断言（`replay::rebuild_prefix` 从载荷与同源文档重算对拍）。
- **CallShape 的冻结由 `CallShape::verified_against(frozen)` 一处判定。** model／effort／`max_tokens` 三个上线字段任一变了即 `E_CONFIG_INVALID` 拒绝，恢复语先指「把动过的那一项改回去」，再指同一句换地址（§8-4-1）；`context_tokens` 只喂本地提醒、不上线，不参与比较。派活面的拦截点（`Command::Dispatch { effort }` → `accounting::worker::dispatching::running` → `city::write_effort`）在放行写房间 effort 之前问这一句；运行时只立判定与拒绝路径，拦在哪里归装配层。
- **只读调用并行执行，按调用序入账（确定性 5）。** 效果由工具自己声明（`kernel::Effect`），执行器不猜：一波开头连续的 `Effect::Read` 调用同时起跑，第一条在本线程跑，其余各占一个 `std::thread::scope` 线程，scope 返回前全部 join；结果按调用序进重排缓冲，再逐条经 `account` 写 `tool_called`＋`tool_result`——入账只此一处，串行段与并行段共用，所以事件的次序与载荷与串行执行一致；各行的时刻是量出来的，并行时本就不同（§8-15）。`tests/run_driver.rs` 与 `turn/tests/concurrent.rs` 在停住的时钟下对拍，逐字节相同。第一条非只读调用就是 checkpoint：它等前面的只读调用收齐才开始，此后整波串行，因为写与写、写与读之间的先后是可观察的。`still_going` 对开头那段只读调用在起跑前逐条先问，Cancel 落在第 k 条就只起跑前 k 条——正是串行波在同一处停下之前会做的那几条；各条的答案留到该条入账之前才交给 `consume_boundary`，所以 Steer 的 `steer_received` 落在串行波写它的同一位置。线程崩溃不是回合错误：该条以 `E_TOOL_UNAVAILABLE` 回给模型。
- **`ConcurrentInvoke` 是三段，不是一个闭包。** 放行（`admit`，`&mut`，按调用序）、执行（`tool` 借出 `&dyn Tool`，`&self`，各条在 scope 线程上调它的 `invoke`）、记账（`account`，`&mut`，按调用序）。一个包着 bench 的闭包表达不了这个次序：放行与记账写同一张去重表与同一份 taint，而 bench 不是 `Sync`（checkpoint 持有 git 仓库句柄），工具是（`kernel::Tool: Send + Sync`，`invoke(&self)`；有内部状态的工具把状态放在自己的锁后）。三个闭包也不行：三者要借同一个 bench，一个要 `&`、两个要 `&mut`。所以它是 trait——第二实现在缝上已经存在：闭包的全覆盖实现（放行即作答，不报效果，于是 citysim 与脚本化工具的测试走串行、字节不动），与装配层 `accounting::worker::driving::placing` 的 bench 实现（`crates/sprawling/Spec.lean` §8-31）。只读调用的门不读 taint，所以先放行后记账不改变任何一扇门的判定；`IdemKey` 的位置在放行时按调用序定下，exec 计数与 checkpoint 记录在记账时按调用序累加，所以它们与串行波逐字相同。落选的是「lane 把整个 bench 放进锁、闭包取 `Sync`」：锁把三条读排成一条队，并行只剩名字。生产路径由 `tests/run_driver.rs` 的三读测试守着：`drive` 走 `lifecycle` 到 `execute_concurrent`，三条读彼此重叠，次序与载荷与串行逐行相同。

- **一个回合的落盘屏障（runtime D24，TF1）：只读调用不在执行前等落盘，写调用的意图先落盘。** 一道屏障的价钱几乎与它带几条记录无关（`kernel::Ledger::append_all` 的文档：一条一屏障约 585 µs，五十条共用一道时每条约 13 µs），所以要省的是屏障的次数。**参照**是逐条落盘：每次追加一次 `Ledger::append`，经 `accounting::worker::relay` 一个阻塞的往返，一回合 `4 + 2 × 调用数` 道屏障（连同 run 自己的两行，`reference_turn_barriers`），且若 `tool_called` 在工具执行之后才写，写动手之后、记录落盘之前掉电，重启后的历史里没有这次写，`replay::DanglingCalls` 也找不到它——ARCHITECTURE §5 第 4 步要排除的正是这个。**决定的次序**：只读调用（`Effect::Read`）追加 `tool_called` 后不等它落盘就执行，`tool_result` 也只追加；写调用追加 `tool_called` 后等它落盘再动手（意图先落盘），它的 `tool_result` 搭下一道屏障；下一个对外可见的效果——下一次模型调用、一次写、run 的冻结——之前一道屏障把此前追加的记录一起落盘。一个回合因此付 `1 + 写调用数` 道，一波最多一条写调用时不超过 2。`EventRef` 仍只为已落盘的记录交出：只读调用的 ref 在那道屏障处才到回合手里，所以「`Ok(ref)` 即已落盘」的端口契约（`kernel::ledger`）一字不改，改的是回合在什么时候需要 ref。性质的权威是 `crates/runtime/spec/Turn/Durability.lean`：任一崩溃点，对外效果之前此前追加的每条记录都已落盘（`tf1_effect_after_durability`）；写动手时它的 `tool_called` 已在盘上（`tf1_write_intent_durable`）；记录与逐条落盘的参照次序逐条相同，盘上的总是它的前缀（`tf1_records_match_reference`、`tf1_durable_is_reference_prefix`），于是 `resume` 面对的是参照次序也会留下的历史，缺的只有没有对外效果的只读调用，重做它们不改变世界；屏障数见 `tf1_turn_barriers`、`reference_turn_barriers`。**形状**：「追加未落盘」住在回合的账本门（`turn::ledger` 的 `Journal`）里——`append_authored`／`append_redacted` 只把 draft 攒下并交回它在回合记录里的位置（`Entry`），`Journal::barrier` 用一次 `Ledger::append_all` 交出攒下的全部并在此刻收下 ref；屏障在三处：`model_called` 之后、模型调用之前（`recovery::ModelCall::record`；run 第一回合的 `prompt_assembled`、插话的 `steer_received` 与 run 自己的 `prompt_shape_compared` 都搭这一道），放行的非只读调用的 `tool_called` 之后、工具动手之前（`wave.rs` 的 `alone`），以及回合收尾（`record` 与取消的 `Journal::close`）。`model_returned` 的 `Entry` 一直留到 `record` 那道屏障之后才用 `Journal::durable` 换成 ref，所以 `TurnReport` 只带已落盘的 ref。seq 在交出时由账本按攒下的次序给，所以次序仍是调用序（relay 的那一半见 `crates/sprawling/Spec.lean` §8-42-2）。**run 自己的行走回合的账本门**：run 在相与相之间写两行（`run::lifecycle`：组装之后的 `prompt_shape_compared`，回答之后、工具波之前的 `checkpoint_committed`），它们经 `Turn::hold_run_line` 作为第三种记录（`turn::ledger::RunLine`，run 写的、带 run 的地址、不扫秘密）排进回合攒下的记录里，于是攒下的记录跨过相而 seq 仍是追加的次序。检查点本身（把写域提交成 commit）不算对外边界：在它之后、下一道屏障之前掉电，只留下一个账本没有提到的 commit，世界里谁也看不见它。所以 Rust 一回合付 `2 + 写调用数` 道（`closedTurn`、`closed_turn_barriers`，前三组性质对它原样成立），只读调用一道也不加；比模型里的 `1 + 写调用数` 多的是收尾那一道，因为 `Journal` 只活一个回合、`TurnReport` 交出的每个 ref 都须已落盘。runtime D36 决定省掉它：`Journal` 改由 `Run<Active>` 持有，攒下的记录跨过回合，`TurnReport` 的 ref 在下一道屏障处换出；性质（ref 在下一个对外效果之前到齐、崩溃点历史与今天的形状相同）与待实现的派生检查在 `Durability.lean` 末两节，Rust 仍是今天的 `closedTurn`，直到那一处实现落地。派生检查：`turn::tests::durability` 的 `tf1_turn_barriers`（计数账本）、`tf1_write_intent_is_durable_before_the_write_runs`（写动手时读盘上的记录），以及经整个 run 的 `run::lifecycle::tests::tf1_run_turn_barriers`（run 自己的行不另付屏障）。代价：页面在那道屏障之后才看到只读调用的两行，晚的是那几条读的执行时间。**落选**：一波结束才一道屏障、写也不先落盘——写动手之后掉电留下一件账本不知道的事，正是第 4 步要排除的；逐条落盘（今天的参照）——安全，但每条只读调用多付两道屏障而不换来任何可恢复性。重新打开它的参数：一次屏障的价钱与它带的记录数变得成正比，或者某个只读工具开始有对外效果（那时它的 `Effect` 就不是 `Read`）。D24 在三个平台上相同：它只决定屏障的次数与位置，屏障本身怎样落盘归 `crates/storage/spec/Jsonl/Barrier.lean`。

- **生成中起跑只读调用（`runtime::turn::speculation`，形状 2 值：按位置的缓存 `Speculated`）。** `Generating::Speculating` 让 `call` 走 `Model::call_speculating`；模型每交出一条调用，只要它排在本回答第一条非只读调用之前、`effect_of` 答 `Effect::Read`、`ahead` 借得出工具，它就在一个 `std::thread::scope` 线程上起跑，scope 在模型调用返回前 join 全部线程，于是一回合的墙钟约等于 max(工具, 生成)，而不是两者之和。结果按调用在回答中的位置缓存进 `Turn<ToolWave>`，连同起跑时的那条调用；入账时仍按调用序先 `admit`，放行（`Cleared`）且该位置缓存的调用与结算后那条逐字段相等，才用缓存结果，否则照常执行——所以 `tool_called`／`tool_result` 的载荷与顺序与串行波相同，时刻按 §8-15 的时点采，推测结果本身不是事件。回答失败（流被切断、恢复段重发）时整份缓存随那次尝试丢弃；取消落在 k 处时 k 之后的缓存随 `Turn` 丢弃；`admit` 自己作答（重放、门拒绝）时该位置的缓存丢弃。哪些调用可以提前、缓存按什么序入账，权威是 `crates/runtime/spec/Turn/Speculation.lean`：越过第一条写调用推测会让读看到写之前的世界（`speculating_past_a_write_changes_the_ledger`）。**起跑不问 `still_going`**：一个已立的取消挡不住早读，它们的结果随 `Turn` 丢弃；代价是被停的 run 仍做完这些读，模型调用失败时也要等它们 join 才返回，而它们都无副作用，所以不越过任何门。**起跑先于放行**：放行写去重表与 taint，被截断的回答得把它们撤回，而只读调用的结果在放行前算出、放行后才用，被拒的那条结果从不到达模型与账本。落选的是「推测时就 `admit`」：它要为截断与取消各写一条撤销路径。`ahead` 有默认 `None`，闭包的全覆盖实现因此不提前起跑，citysim 字节不动；bench 的实现按名借出（`ToolBench::tool_named`，与 `tool_for` 同一张表）。
-/

/-!
### 8-6 turn／prefix／handoff 的会话面（形状不变，参数长入）


typestate 四相、边界消费、事件序、私有字段三不变量不动；会话、工具与调用形状作为相变函数的入参进来。被否替代：平行第二条 call 路径——同一相两个入口即两个权威，落选。

```rust
// kernel::model（缝上 canonical 会话类型，`crates/kernel/Spec.lean` §8-24）：
// ChatRequest<'a> { system: Vec<SystemBlock>, messages: Cow<'a, [ChatMessage]>, tools: Cow<'a, [ToolDef]>, breakpoint: MessageBreakpoint }
// SystemBlock { text, cache }；ChatMessage { role, content: Vec<ContentBlock> }；Role { User, Assistant }
// ContentBlock { Text{text} | ToolUse{id,name,input:Payload} | ToolResult{tool_use_id,content,is_error} }
// ToolDef { name, description, input_schema: Payload }；ModelUsage 四整数；StopReason { EndTurn, ToolUse, MaxTokens }
// ModelRequest<'a> 携 chat: ChatRequest<'a>；ModelReturn 携 usage: Option<ModelUsage>、stop: Option<StopReason>、billed: Option<UsdMicros>

// runtime::conversation
pub enum Opening { FromJob, Inherited, WithPerson }   // 穷尽三臂，城在写 brief 时已决定；Inherited 只由 fork 选
pub struct Conversation { /* messages、sent、held —— 私有；执行器持有，逐回合推进 */ }
impl Conversation { pub fn new() -> Conversation;
    pub fn push_task_lines(&mut self, task: &str, goal: &str, opening: Opening);  // 首轮，run_started 可重建
    pub fn push_steer(&mut self, source: &str, text: &str);          // 「user」或「@ID」前缀形
    pub fn push_reminder(&mut self, reminder: &ContextReminder);    // §8-34
    pub fn push_inherited(&mut self, messages: &[ChatMessage]);     // 分支开场继承的那段（§8-2）
    pub fn push_assistant(&mut self, content: Vec<ContentBlock>);
    pub fn push_tool_results(&mut self, results: Vec<ContentBlock>); // ToolResult 块（pipeline 产出的成品文本）
    pub fn mark_sent(&mut self);                                     // §8-47
    pub fn messages(&self) -> &[ChatMessage]; }

pub struct CallShape { pub model: String, pub max_tokens: Option<Ceiling>, pub effort: Option<Effort>,
                       pub context_tokens: u64 }   // 窗口大小；只喂本地提醒、不上线（§8-3）
impl CallShape { pub fn verified_against(&self, frozen: &CallShape) -> Result<(), AxError>; }   // 冻结判定（§8-3）
                    // 三个上线字段全部来自选型点，无一项在调用处手写。model 与 max_tokens 解自
                    // 模型目录行（gateway::market::ModelEntry）；effort 解自 kernel::FrozenConfig，
                    // Run 内恒不变——改它就换缓存前缀（理由与出处在 `crates/kernel/Spec.lean` §8-22）
impl Turn<Assembling> {
    pub fn assemble<'c>(self, interrupt: Interrupt, ledger: &mut dyn Ledger, prompt: RunPrompt<'_>,
                    conversation: &'c Conversation, tools: &'c [ToolDef], shape: &CallShape)
        -> Result<PhaseOutcome<Turn<Calling<'c>>>, AxError>;   // Calling<'c> 相私持已组 ChatRequest，借用 conversation 与 tools（'c），调用返回前会话不动；prompt_assembled 载荷长入
}
pub struct RunPrompt<'r> { /* prefix: &'r FrozenPrefix, recorded: &'r mut PromptRecord */ }
impl<'r> RunPrompt<'r> { pub fn new(prefix: &'r FrozenPrefix, recorded: &'r mut PromptRecord) -> RunPrompt<'r>; }
#[derive(Default)] pub struct PromptRecord { /* written: Option<Payload> */ }   // 住 Run 的 Active 态，跨回合
// Interrupt 增 Steer { source: String, text: String }：边界消费→追加 steer_received（in-window）→照常 Advanced（不终止回合）；
// 文本回折入 Window 归执行器（它持 Window 与 Steer 原文），呼应「追加在结果末尾」。
// TurnReport 长入：model_content: Vec<ContentBlock>（助手内容）与 wave_results: Vec<ContentBlock>（ToolResult 块）——
// 执行器据此折叠 Window；两份材料是收尾边界压缩后的 exchange（8-44），离线重建同源于
// model_returned.data.content 与 tool_result 事件并在同一边界重放同一判定（C16 由 8-44 的对拍承接）。
// kernel::ToolCall 增 id 字段：tool_use↔tool_result 对号是两 Dialect 的 wire 硬性要求；
// tool_called 载荷增 id，tool_result 载荷增 tool_use_id。
```

**prefix 四段全量**（新增构建面；既有 FrozenSegment/assemble 不动）：

```rust
pub struct SourceDoc { pub addr: Address, pub bytes: Option<Vec<u8>> }   // None＝缺失或不可读（跳过入账）
    // `generation` 数的是摘要的代数（链上第一份＝1，摘要的摘要加一），`NonZeroU32`
    // 使「还没数」拼不出 0——零与未知是两件事，与 `ModelReturned` 缺席≠报零同族。
pub struct SegmentCaps { pub city: u64, pub building: u64, pub resident: u64, pub run: u64 }  // 字节上限；来源＝调用方（`startup_default` 取 STARTUP_BUDGET_TOKENS × BYTES_PER_TOKEN ÷ PREFIX_SLOTS）
pub struct PrefixPlan { pub city: Vec<SourceDoc>, pub building: Vec<SourceDoc>,
                        pub resident: Vec<SourceDoc>, pub run: Vec<SourceDoc>, pub caps: SegmentCaps }
pub struct PrefixBuild { pub prefix: FrozenPrefix, pub notes: Payload }   // notes＝逐段 sources/skips/truncations（prompt_assembled 载荷入口）
pub fn build_prefix(plan: PrefixPlan) -> Result<PrefixBuild, AxError>;
```

- **首轮不指向任何文件**：`JOB.md` 的正文已是 Run 段，所以开场行不携 `cas:b3-…` 一类的内容哈希——城里没有一个工具解析得了它，而溯源在 Ledger 里已记两遍。`Opening` 的两臂不是排版偏好：被派了一件活的会话与正在和人说话的会话要的第一句话不同，而把人那句话包成 `Task:`／`Goal:` 表单，换回来的也是一张表单。**`FromJob` 的开场行不复述任务**：它只写 `The task is in JOB.md above.` 与 `Goal: <goal>` 两行——任务正文已在 Run 段，再抄一遍，人贴的一段话每次请求就付两遍（Run 段不在缓存里，每回合全价）。Goal 仍写在这里，因为它是「什么时候停」，短，且是这一行唯一不重复的指令。**分叉重建的母亲开场用 `Inherited`**：母亲的 `JOB.md` 在她的房间里，不在分叉的 Run 段里，所以那一行写 `Task: <task>` 与 `Goal: <goal>`；若照搬 `FromJob`，分叉读到的「在上面的 JOB.md 里」指向一个它看不见的文件，母亲的任务就丢了。
- **read 的两条路，差别在于谁选的**：**路径是模型选的，故受审**——`Address::parse` 杀穿越，`is_reserved` 杀保留子树（`E_GATE_DENIED`）；**catalog 里的名字是人选的**——楼的阅览室写下它时准入就已发生，故它解到的 skill 可以住在保留空间里。两条路共用一个参数，因为对模型而言它们是同一件事（把一份东西调到眼前）；**先问 catalog** ，一个同名文件不得遮蔽楼已经准入的 skill。
- **`Catalog::expand` 答 `Expansion { Skill { addr, package }, Said { text } }` 而不是 `String`**：skill 展开成一个可打开的地址，其余展开成目录自己持有的正文；两者压成一个字符串时，调用方只能拿它去试解析成地址，而一段恰好能解析成地址的正文就会被当成文件打开。
- **正文不在 prompt 里，所以交出去而不是拒绝**：`render()` 只写每条的 disclosure，`expansion` 从未进过窗口。
- **它是 `Catalog::expand` 的调用者**：没有它，一栋楼的阅览室能报出一个 skill 的名字而永远交不出它。
- 截断：文件超段位余额即截到边界，原处留 ASCII 标记（文本与切口规则属 `runtime::elision`，§8-42），恒不静默丢尾；标记字节从段预算先扣。
- 单位换算写成代码：`SegmentCaps` 四个字段是**字节**，`STARTUP_BUDGET_TOKENS` 是**token**，`startup_default` 用 `BYTES_PER_TOKEN` 与 `PREFIX_SLOTS`（`NonZeroU64`，与 `SegmentSlot` 变体数由 `prefix::tests` 钉住）把前者换算成后者。两个换算常量住 `kernel::consts_policy`（`BYTES_PER_TOKEN` 与 `PREFIX_SLOTS`），与 `STARTUP_BUDGET_TOKENS` 同一个家；prefix.rs 只读。
- 断点只有一个作者：`prefix::BreakpointPlan`（`prefix/breakpoint.rs`，形状 1 判定，纯函数）。`BreakpointPlan::for_conversation(&[ChatMessage])` 决定一次请求实际发出的断点：前三段（city／building／resident）的段界各一个，对话非空时尾消息再一个，合计 ≤ `CACHE_BREAKPOINTS_MAX`（4）；run 段界不放，因为尾锚紧随其后已覆盖它。`FrozenPrefix::system_blocks()` 以 `BreakpointPlan::marks_edge(slot)` 标 system 块，`BreakpointPlan::message_breakpoint` 标请求（回合借用会话，不复制它，kernel D6），`prompt_payload(&plan)` 把 `plan.breakpoints()` 逐个拼成 `breakpoints` 行（段界写 slot 名，尾写 `tail`）；`verified_system_hashes` 以同一个 `marks_edge` 核对线上的块。兼容格式只负责拼写（Anthropic：被标记消息的最后一块带 `cache_control`），不决定任何断点。
```rust
pub enum Breakpoint { Edge(SegmentSlot), Tail }
pub struct BreakpointPlan { /* tail: Option<usize> */ }
impl BreakpointPlan {
    pub fn for_conversation(messages: &[ChatMessage]) -> BreakpointPlan;
    pub fn marks_edge(slot: SegmentSlot) -> bool;
    pub fn message_breakpoint(&self) -> MessageBreakpoint;   // 写进 ChatRequest.breakpoint；会话本身不动
    pub fn breakpoints(&self) -> Vec<Breakpoint>;
}
```
- A15 重建器：`replay::rebuild_prefix(data: &serde_json::Value, resolver: &dyn Fn(&Address) -> Option<Vec<u8>>) -> Result<[B3Hash; 4], AxError>`——从 prompt_assembled 载荷（逐源的键以 `kernel::event::record::PromptSource` 为准：`{addr, kept, marker, dropped, producer}`，其中 `producer` 不参与对拍，因为哈希只盖段字节、指纹不是字节的一部分）与同源文档重算逐段哈希对拍；resolver 以 Address 取文（钉版 oid 级解析随 checkpoint 接入升级，接口不变）。拼接分隔符的唯一权威住 prefix.rs（`DOC_JOIN`），截断标记的唯一权威住 `runtime::elision`（§8-42），replay 同 crate 复用不另拷。
- E_TOOL_OUTCOME_UNKNOWN 补写面：`replay::DanglingCalls` 逐条看已验证的记录，`observe(&EventRecord)`；看完 `into_calls() -> Vec<EventRecord>` 交出没有结果的那些 `tool_called`，按 seq 升序（`tool_called` 之后同一 run 里没有 `tool_result` 即悬空；同一 run 前一个未合上又来一个，前一个同样悬空）。`replay::outcome_unknown_draft(call, t) -> EventDraft` 写补记的 `tool_result`，携 E_TOOL_OUTCOME_UNKNOWN 错误体。消费者＝`resume` 的启动扫描：它把 `fold_ledger_dir` 验过的每条记录交给 `observe`，验链与检出是同一遍流式读，常驻的是一段字节，外加尚未合上的那几条调用。补写方经 `ToolCalled`／`ToolResult` 两个结构读写，读不懂即拒绝：若手挑 `id` 与 `name` 两个键、读不出就写下 `"unknown"`，一条这个 build 读不懂的调用就会被关在一个谁也答不上的 id 上。**被否：先得到 `VerifiedLedger` 再检出**——整本原始行与整本记录同时常驻，只为找几条没有结果的调用；检出只交回 `(RunId, Seq)`，调用方还要回到整本记录里逐条找那一行。

```rust
#[derive(Debug, Default)]
pub struct DanglingCalls { /* 私有：未合上的调用（按 run）、已判悬空的调用 */ }
impl DanglingCalls {
    pub fn observe(&mut self, record: &EventRecord);
    pub fn into_calls(self) -> Vec<EventRecord>;
}
pub fn outcome_unknown_draft(call: &EventRecord, t: TimeMs) -> Result<EventDraft, AxError>;
```

- 单位换算写成代码：`SegmentCaps` 四个字段是**字节**，`STARTUP_BUDGET_TOKENS` 是**token**，`startup_default` 用 `BYTES_PER_TOKEN` 与 `PREFIX_SLOTS`（`NonZeroU64`，与 `SegmentSlot` 变体数由 `prefix::tests` 钉住）把前者换算成后者。两个换算常量住 `kernel::consts_policy`（`BYTES_PER_TOKEN` 与 `PREFIX_SLOTS`），与 `STARTUP_BUDGET_TOKENS` 同一个家；prefix.rs 只读不再自定。
- 断点：`FrozenPrefix::system_blocks()` 产四块、逐块 cache=true＝断点恒 4＝`CACHE_BREAKPOINTS_MAX`，断点只落段界。
- handoff：五段＋构造点＋resume 消费；「下一步段首列用户指定动作」属生产者纪律（回合层与 spine 文件），类型不另加钩。
- 第四取消点（派生前）：`collab::delegate_tool` 是它的生产者：`SafePoint::BeforeSpawn` ＋ `Turn<Recording>::record(interrupt, ledger)`，装配层在 `Completion::Cancelled` 时清空派生台，**被取消的 Run 一件活也交不下去**。
-/

/-!
### 8-18 runtime::turn 目录化


| 文件 | 管什么 |
|---|---|
| `turn.rs` | typestate 载体 `Turn<S>` 与四个相类型，`assemble`／`call`／`record`，以及唯一的中断消费点 `consume_boundary`。`assemble` 与 `call` 带 `argument_count` 豁免，故留在原路径 |
| `turn/boundary.rs` | 执行器在相变处交出什么、相变答什么：`Interrupt`、`PhaseOutcome`、`TurnCancelled` |
| `turn/report.rs` | 一轮跑完交给 run loop 的东西与它被给定的调用形状：`TurnReport`、`CallShape` |
| `turn/recovery.rs` | 模型调用的恢复管线（§8-49） |
| `turn/prompt.rs` | 一次 run 的 prompt 材料 `RunPrompt`，`assemble` 的入参 |
| `turn/speculation.rs` | 生成中起跑只读调用的门与缓存（§8-3） |
| `turn/wave.rs` | 边界 3：`impl Turn<ToolWave>` 的工具波，只读前缀并行执行，按调用序入账 |
| `turn/wave/reorder.rs` | 开头只读段同时起跑、按调用序交回答案的重排缓冲：`all_at_once` 与 `lost_answer` |
| `turn/ledger.rs` | 本模块通往账本的唯一一道门：`Journal`、`Authored`、`Carried`，以及四种等来的时刻从哪只钟读（§8-41、§8-15） |
| `turn/tests.rs` | 纯索引 |
| `turn/tests/helpers.rs` | 三处共用的夹具：`TestLedger`、`OneShotModel`、`prefix`／`run_id`／`shape`／`advance`／`probe_call` |
| `turn/tests/phases.rs` | 四个边界跑在真账本链上 |
| `turn/tests/concurrent.rs` | 只读前缀并行：波内停下只起跑停点之前的调用，steer 落在串行波写它的位置；停住的时钟下两者账本字节与串行一致 |
| `turn/tests/window.rs` | 开场白与 steer 在窗口里留下什么 |
| `turn/tests/redaction.rs` | 工具参数与工具结果里的密钥进不了账本，其余字段完好 |
-/

/-!
### 8-41 回合带进账本的明文，必经打码那道门


**不变量：`tool_called`、`tool_result`、`model_returned` 三类事件的载荷，在写进账本之前逐串跑过 `kernel::scan`，命中的区段换成 `secret:redacted/<b3-16>` 标记。** 三类都要：工具参数与工具结果若**逐字**入账，一次 `read` 读出的别人项目的 `.env` 正文、一次 `exec` 的 stdout，就会进入只增且可导出的历史。账本不可重写，所以这类损害不可逆——这是它排在安全清单最前的理由。

```rust
// turn/ledger.rs —— 本模块通往账本的唯一一道门
enum Authored { PromptAssembled, ModelCalled, CancelReceived, SteerReceived }  // 回合自己算出的值
enum Carried  { ModelReturned, ToolCalled, ToolResult }                        // 供应方或工具交回的值
struct Journal { /* run、who、t、refs、redacted —— 全私有 */ }
impl Journal {
    fn append_authored(&mut self, ledger: &mut dyn Ledger, event: Authored, data: Payload)
        -> Result<EventRef, AxError>;
    fn append_redacted(&mut self, ledger: &mut dyn Ledger, event: Carried, data: Payload)
        -> Result<EventRef, AxError>;   // 载荷经 Payload::of 由记录结构构造，扫描在门内做
    fn append(&mut self, …) -> Result<EventRef, AxError>;   // 本模块唯一的 `Ledger::append` 调用
}
```

**四条口径：**

1. **「必经」由类型保证，不由注释约定。** `Authored` 与 `Carried` 把本模块写的七类事件切成互不相交的两半，各自只在一个 append 函数里出现；`EventDraft` 的构造收进私有的 `Journal::append`。于是「不打码就写 `tool_called`」这件事在本模块里**没有可写出来的形式**——要绕过它，得先手写一个 `EventDraft`，那是一个审阅时看得见的动作。
2. **打码器只有一个。** `Journal::append_redacted` 调 `runtime::redact::redact`，后者调 `kernel::scan`；本 crate 不存在第二个扫描器或第二套标记文法。
3. **计数进 `TurnReport::redacted()`，内容不进。** 一个数目足以让诊断行说出「打掉了 N 段」，而说不出打掉的是什么；`Journal` 用饱和加法累计，跨相随 `journal` 一起搬。
4. **窗口留住账本丢掉的。** `wave_results` 与 `assistant` 在打码之前就已从 `ToolOutcome` 与 `ModelReturn` 取出，模型因此仍看得见工具的真实输出，思考块的签名也不受影响——历史与上下文是两个汇，只有账本是永久的。

**关门测试**（`turn/tests/redaction.rs`）：一次 `edit` 调用的参数与结果各带一串 `sk-ant-` 开头的密钥，跑完整回合后账本每一行都不含那串明文，而 `id`、工具名、`path` 参数与替换点周围的散文完好，`report.redacted() >= 2`，`replay::verify_lines` 仍自证。
-/

/-!
### 8-51 一次调用在账上带出它的登记，与它被裁掉的原文在哪（`turn::wave`、`pipeline`）


**(a) 工具面交出整份登记**

```rust
pub trait ConcurrentInvoke {
    /// 这次调用那件工具的登记；None 是工具台不认识的名字（它不是只读的）。
    fn meta_of(&self, call: &ToolCall) -> Option<&ToolMeta>;
    /// 经 `call` 的一次调用换成它所指的那件；默认原样交回（§8-61）。
    fn resolve_call(&self, call: ToolCall) -> ToolCall { call }
    // ahead、admit、tool、account 不变
}
```

- `meta_of` 取代原来的 `effect_of`：工具波判只读前缀、推测门判能否提前起跑，读的都是登记里的 `effect`；`tool_called` 还要照录 `effect` 与 `render`（`crates/kernel/Spec.lean` §8-75(b)）。一个方法交出整份登记，三处读同一个答案；两个方法各交一项，就是两处各自去查同一份登记。
- 闭包工具面（citysim 与测试）答 `None`：它的调用写 `tool_called` 时两键缺席，与它今天不声明效果、波次恒串行是同一件事。

**(b) 被裁掉的结果，原文在哪**

```rust
// runtime::pipeline
pub(crate) const EXEC_ACCOUNTS: &str = "sieve";         // exec 结果里账目住的键
pub(crate) const CONNECTOR_ACCOUNTS: &str = "offload";  // 外包服务答复里账目住的键
/// 一次工具结果里第一笔离窗账目的 original；结果里没有账目时为 None。
pub fn pinned_original(result: &serde_json::Value) -> Option<Locator>;
```

- **账目随它裁掉的那个结果走。** `package_exec` 把 `package` 记下的 `ResultOffloaded` 放进结果的 `sieve` 键，`package_connector` 放进 `offload` 键（§8-27、connector 一节）；这个结果写进 `tool_result`，而 `tool_result` 带着它所答那次调用的 `tool_use_id`。所以一笔账目属于哪次调用，由它所在的那一行说出，账目自己不另记调用 id：多记一份就是同一个 id 的第二个家，而且这个结果整份进模型的字节。今天没有任何写方单独写 `result_offloaded` 行。
- **第一笔就是原文**：同一个结果先经 sieve 裁、裁后仍大再被普通搬运存一次时，账目按管线次序排——sieve 的那笔在前，它的 `original` 是命令写出的全部字节；后一笔的 `original` 是 sieve 留下的替身。读者要的是命令原本说了什么，所以取第一笔。
- 键名的权威是这两个常量；读它们的是 `pinned_original`，写它们的是两扇 `package_*` 门。

**(c) 一次 exec 以哪个码结束**

```rust
// runtime::pipeline
/// 一次 exec 结果里的 `exit_code`；结果里没有码时为 None。
pub fn exit_code_in(result: &serde_json::Map<String, serde_json::Value>) -> Option<i64>;
```

- **写方只有一处，读方也只有一处。** 键名与「有码才写码」的规则住 `tools::exec::outcome` 的 `ending`；读它的是 `exit_code_in`，`package_exec` 交给 sieve 的码与读面答给页面的 `Call.exit_code`（`crates/wire/Spec.lean` §8-76）都经它读。信号停下的、没等到的命令没有码，读出 `None`，不读成某个数。
-/

namespace Runtime.Turn

/-- 一道边界上到达的信号（`turn::Interrupt`）。Steer 的来源与正文不影响写下哪一行，所以模型不带它们。 -/
inductive Interrupt where
  | None
  | Cancel
  | Steer
  deriving DecidableEq, Repr

/-- 一个回合结束的方式（`turn::PhaseOutcome` 的两臂）。 -/
inductive Ending where
  | Advanced
  | Cancelled
  deriving DecidableEq, Repr

/-- 一个回合写进账本的行，以 `kernel::EventKind` 的名字命名；`prompt_assembled` 每个 run 只写一次（§8-39），不在这里。 -/
inductive Line where
  | PromptShapeCompared
  | ModelCalled
  | ModelReturned
  | ToolCalled (call : Nat)
  | ToolResult (call : Nat)
  | SteerReceived
  | CancelReceived
  deriving DecidableEq, Repr

/-! D5 一回合里等来的四个时刻各采各的，钟交给回合的账本门

**决定**：`model_called`、`model_returned`、`tool_called`、`tool_result` 的信封 `t` 记各自那一刻，每条工具调用采开始与答复两次。时钟在 `Turn::begin` 交给 `turn::ledger::Journal`；`Authored::ModelCalled`、`Carried::ModelReturned`、`Carried::ToolCalled`、`Carried::ToolResult` 四个变体各带一个读数，其余变体只能带回合时间戳。

**理由**：结果上的 UTC 戳（§8-10）、`view --since` 与 `--until` 的时间窗与 run 页的调用用时读的是同一个值，一行只有一个时间。只采答复不采开始，调用用时只能推算，会把准入里的检查点提交算进工具用时。钟放在 `Journal` 里，`call` 与 `execute_concurrent` 的参数表不动，采样只发生在回合自己那一道账本门后面。

**被否**：①`ToolResult` 载荷加 `returned_ms`：一行两个时间，读者要知道信哪一个；②时钟随 `Generating` 进 `call`，再与工具面、`still_going` 合成一个值进 `execute_concurrent`：同一只钟两个入口，而 `call` 的参数已按 `budgets.toml` 钉在 5；③只采答复：两条并行对拍测试可以原样保留，但用时量不出来。

**代价**：同一 run 内 `t` 不再随 `seq` 单调，次序以 `seq` 为准；并行对拍测试改用停住的时钟比字节；`golden-p0` 重生成一次。
-/

/-- 哪几种行的 `t` 是为它自己采的读数，其余的带回合时间戳（`turn::ledger` 的 `Authored`／`Carried` 变体）。 -/
def measured : Line → Bool
  | .ModelCalled => true
  | .ModelReturned => true
  | .ToolCalled _ => true
  | .ToolResult _ => true
  | .PromptShapeCompared => false
  | .SteerReceived => false
  | .CancelReceived => false

/-- `consume_boundary`：一道边界消费一个信号，写下什么、回合还走不走。Steer 记一行照常前进；Cancel 记一行，回合到此为止。 -/
def consume_boundary : Interrupt → List Line × Bool
  | .None => ([], true)
  | .Steer => ([.SteerReceived], true)
  | .Cancel => ([.CancelReceived], false)

/-- 第 `first` 条起的 `count` 条调用各自的两行，按调用序。 -/
def accounted (first : Nat) : Nat → List Line
  | 0 => []
  | count + 1 => [.ToolCalled first, .ToolResult first] ++ accounted (first + 1) count

/-- 工具波：第 `call` 条调用之前问 `still_going call`，答案交给同一个 `consume_boundary`；走下去就按调用序写它的 `tool_called` 与 `tool_result`。开头只读段并行执行时次序与载荷不变，由 `spec/Turn/Speculation.lean` 证明；这里只管边界。 -/
def wave (still_going : Nat → Interrupt) (call : Nat) : Nat → List Line × Bool
  | 0 => ([], true)
  | remaining + 1 =>
    let boundary := consume_boundary (still_going call)
    if boundary.2 then
      let rest := wave still_going (call + 1) remaining
      (boundary.1 ++ [.ToolCalled call, .ToolResult call] ++ rest.1, rest.2)
    else (boundary.1, false)

/-- `wave` 走一步，写成一条等式，好让证明一次只展开最外一层。 -/
theorem wave_succ (still_going : Nat → Interrupt) (call remaining : Nat) :
    wave still_going call (remaining + 1) =
      if (consume_boundary (still_going call)).2 then
        ((consume_boundary (still_going call)).1 ++ [.ToolCalled call, .ToolResult call] ++
            (wave still_going (call + 1) remaining).1,
          (wave still_going (call + 1) remaining).2)
      else ((consume_boundary (still_going call)).1, false) := rfl

/-- 一个回合在四个取消点上收到的信号，以及波里每条调用之前的那一问。 -/
structure Signals where
  before_assemble : Interrupt
  before_call : Interrupt
  before_wave : Interrupt
  still_going : Nat → Interrupt
  before_spawn : Interrupt

/-- 回合的四相：组装、调用、工具波、收尾。每一相先消费它的边界，命中 Cancel 就写下 `cancel_received` 并结束；否则做这一相的事。`calls` 是模型这一答发出的调用数。 -/
def turn (signals : Signals) (calls : Nat) : List Line × Ending :=
  let assemble := consume_boundary signals.before_assemble
  if !assemble.2 then (assemble.1, .Cancelled) else
  let call := consume_boundary signals.before_call
  let called := assemble.1 ++ [.PromptShapeCompared] ++ call.1
  if !call.2 then (called, .Cancelled) else
  let opening := consume_boundary signals.before_wave
  let returned := called ++ [.ModelCalled, .ModelReturned] ++ opening.1
  if !opening.2 then (returned, .Cancelled) else
  let landed := wave signals.still_going 0 calls
  let waved := returned ++ landed.1
  if !landed.2 then (waved, .Cancelled) else
  let spawn := consume_boundary signals.before_spawn
  (waved ++ spawn.1, if spawn.2 then .Advanced else .Cancelled)

/-- 没有人喊停的一波，按调用序给每条调用记下两行。 -/
theorem an_uninterrupted_wave_accounts_every_call (still_going : Nat → Interrupt)
    (quiet : ∀ call, still_going call = .None) :
    ∀ (count first : Nat), wave still_going first count = (accounted first count, true)
  | 0, _ => rfl
  | count + 1, first => by
    rw [wave_succ, an_uninterrupted_wave_accounts_every_call still_going quiet count (first + 1)]
    simp [consume_boundary, quiet, accounted]

/-- **取消落在第 k 条调用之前，账本上恰好是前 k 条调用的两行，再一条 `cancel_received`。** 串行的波在同一处停下之前做的就是这几条，后面的调用一条也不写，所以账本上没有一条没有结果的调用。 -/
theorem a_cancel_before_a_call_accounts_exactly_the_calls_before_it
    (still_going : Nat → Interrupt) :
    ∀ (before first remaining : Nat),
      (∀ j, j < before → still_going (first + j) = .None) →
      still_going (first + before) = .Cancel →
      wave still_going first (before + remaining + 1) =
        (accounted first before ++ [.CancelReceived], false)
  | 0, first, remaining, _, stopped => by
    have stopsHere : still_going first = .Cancel := by simpa using stopped
    rw [show 0 + remaining + 1 = remaining + 1 by omega, wave_succ]
    simp [accounted, consume_boundary, stopsHere]
  | before + 1, first, remaining, quiet, stopped => by
    have here : still_going first = .None := by
      simpa using quiet 0 (Nat.succ_pos before)
    have later : ∀ j, j < before → still_going (first + 1 + j) = .None := by
      intro j below
      have shifted := quiet (j + 1) (by omega)
      rwa [show first + (j + 1) = first + 1 + j by omega] at shifted
    have stoppedLater : still_going (first + 1 + before) = .Cancel := by
      rwa [show first + (before + 1) = first + 1 + before by omega] at stopped
    have rest := a_cancel_before_a_call_accounts_exactly_the_calls_before_it still_going
      before (first + 1) remaining later stoppedLater
    rw [show before + 1 + remaining + 1 = (before + remaining + 1) + 1 by omega, wave_succ, rest]
    simp [accounted, consume_boundary, here]

/-- 一道边界上的 Steer 不结束回合：只要四个取消点与波里每一问都不是 Cancel，回合就前进。 -/
theorem a_steer_never_ends_a_turn (signals : Signals) (calls : Nat)
    (assemble : signals.before_assemble ≠ .Cancel) (call : signals.before_call ≠ .Cancel)
    (opening : signals.before_wave ≠ .Cancel) (spawn : signals.before_spawn ≠ .Cancel)
    (going : ∀ k, signals.still_going k ≠ .Cancel) :
    (turn signals calls).2 = .Advanced := by
  have keeps : ∀ signal : Interrupt, signal ≠ .Cancel → (consume_boundary signal).2 = true := by
    intro signal notCancel
    cases signal with
    | None => rfl
    | Steer => rfl
    | Cancel => exact absurd rfl notCancel
  have waves : ∀ (count first : Nat), (wave signals.still_going first count).2 = true := by
    intro count
    induction count with
    | zero => intro _; rfl
    | succ count ih =>
      intro first
      rw [wave_succ]
      simp [keeps _ (going first), ih]
  simp [turn, keeps _ assemble, keeps _ call, keeps _ opening, keeps _ spawn, waves]

/-- **被取消的波以它的 `cancel_received` 结束，之后什么都不写**：回合后面的 `handoff_written` 与 `run_frozen` 归 `run` 的 `freeze`（§8-15）。 -/
theorem a_cancelled_wave_ends_with_its_cancel (still_going : Nat → Interrupt) :
    ∀ (count first : Nat), (wave still_going first count).2 = false →
      ∃ before, (wave still_going first count).1 = before ++ [.CancelReceived]
  | 0, _, ended => by simp [wave] at ended
  | count + 1, first, ended => by
    rw [wave_succ] at ended
    rw [wave_succ]
    cases signal : still_going first with
    | Cancel => exact ⟨[], by simp [consume_boundary]⟩
    | None =>
      have restEnded : (wave still_going (first + 1) count).2 = false := by
        simpa [consume_boundary, signal] using ended
      obtain ⟨before, ends⟩ :=
        a_cancelled_wave_ends_with_its_cancel still_going count (first + 1) restEnded
      exact ⟨[.ToolCalled first, .ToolResult first] ++ before, by simp [consume_boundary, ends]⟩
    | Steer =>
      have restEnded : (wave still_going (first + 1) count).2 = false := by
        simpa [consume_boundary, signal] using ended
      obtain ⟨before, ends⟩ :=
        a_cancelled_wave_ends_with_its_cancel still_going count (first + 1) restEnded
      exact ⟨[.SteerReceived, .ToolCalled first, .ToolResult first] ++ before,
        by simp [consume_boundary, ends]⟩

end Runtime.Turn
