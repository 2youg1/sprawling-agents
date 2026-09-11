# runtime-SPEC.md

> crate：`runtime`。本 SPEC 先于代码存在；实现不多不少地遵守本文。
> Stage 1 两模块（replay／fork）＋Stage 2 三模块最小形（turn／prefix／handoff，§8-3…§8-5）。
> Stage 3 增章：完备化（§8-6）＋pipeline／offload（§8-7／§8-8）＋watchdog（§8-9）＋clock／catalog／mode（§8-10…§8-12)＋sandbox 缝（§8-13）＋tools 三件（§8-14）。

## 1 需求拆解

| 卡 | 模块 | 一句话 |
|---|---|---|
| S1.10 | `replay` | 离线重演验链：EventRef 第二铸造点；A2 的执行体 |
| S1.10 | `fork` | 分叉前缀：母 Run 事件序列到节点为止的逐字节前缀；A19 的执行体 |
| S2.01 | `turn` | 回合 typestate 四相＋取消边界；形状 5；相内中断编译不过 |
| S2.02 | `prefix` | FrozenSegment 四段＋分段哈希＋易变类型隔离；形状 5＋2 |
| S2.02 | `handoff` | 五段构造点＋resume 消费 Handoff 产新 Run 种子；形状 2 |
| S3.08 | 完备化 | prefix 四段全量（封顶＋截断标注＋跨段去重＋跳过入账）＋断点 ≤4＋Steer 边界消费＋窗口组装入 Assembling 相 |
| S3.09 | `pipeline`＋`offload` | 结果信封三附件＋offload 四不变量（独占有损可还原）＋截断定序 |
| S3.10 | `clock`＋`catalog`＋`mode` | ClockStamp 纯格式化＋渐进披露三类条目＋五 mode 枚举 |
| S3.11 | `watchdog` | 处置面分级（纠正 Steer→停滞→冻结）；判据只从 kernel::stall 来 |
| S3.12 | `sandbox` | 缝（trait）＋wasmtime fuel 生产适配器＋直通/故障两替身；A10 三断言 |
| S3.13 | `tools/` 三件 | exec 三臂／edit 乐观并发／status 十二字段＋ToolBench 按 Effect 过门 |

「replay 只重演不重执行」的 S1 含义：本期重演＝验证链与重建记录序列；入窗重建器（C16/A15）随 prefix 组装（S2+）加入，两卡共用本模块的验证输出。

## 2 验收标准

- A2 演示：对 jsonl 落盘目录与内存行序列各跑一次 verify，逐事件验 prev 链与 seq 连续；任一字节被篡改即拒并报行号。
- A19 演示：从任一节点取分叉前缀，与母 Run 原始行 0..=at_seq 逐字节相同；`at_seq` 越界＝`E_INVALID_ARGS`（恒不静默截到末尾）；母序列不因分叉改变。
- 未知 kind：无 `ig:true` 即拒（方向语义：更新的写方）；带 `ig:true` 的行跳过类型化解读但链照验。
- S3.08：A4——同一 PrefixPlan 两次 build 逐字节同（golden）；A15——prompt_assembled 载荷＋同源文档经 `rebuild_prefix` 重建，四段哈希逐段相同。
- S3.09：A7——offload 往返（替代体≤原件且≤上限；循 rest_path 续读与循 CAS 取回字节一致；外部清理后自 CAS 重物化字节一致；命中既有哈希直引原 Locator）。
- S3.10：A18 零字节——granularity=Off 时打包输出与未接 clock 特性逐字节相同。
- S3.12：A10 三断言结论书——fuel 内成功／未授能力被拒／fuel 耗尽中断（真 wasmtime 上）。
- S3.13：L0×失败注入矩阵；A6 双守（Done 恒携 kind 合法证据，运行时纵深校验）；A8（到限恒 `Completion::Limit`，恒不记完成）。

## 3 假设与歧义

1. **verify 的规范复验**：v1 无升级器链，故对每行断言 `canonical_line(parse_line(raw)) == raw`（写方规范性质）。未来 v>1 经升级器读入后此断言只对原版字节成立——届时随升级器一并改约（本文更新）。
2. **fork 的 run_forked 落账**：事件写入母城 Ledger 由调用方（S2 起 runtime 回合层／citysim）执行；本期 fork 只产 EventDraft 与前缀，不持 Ledger 句柄——保持纯函数形。
3. **同一套重建器**：A15 与 A19 共用 verify 输出；本期重建器＝verified 行序列本身。

## 4 现状分析

空壳。verify 为 O(n) 全量；S1 消费面（测试/夹具/citysim）规模千行级，无性能议题；seq→偏移索引属 S3 memory::index。

## 5 权威信源

Fork 三规则；重放/分叉/幂等；at_seq 越界、未知 kind、崩溃恢复行；kernel-SPEC §8-4/§8-9；memory-SPEC §8-1。

## 6 命名统一

**跨 crate 类型住处（card-1.1–1.3 起）**：`kernel` 的门／计划／脊／事件／错误／弃置／秘密七面已切目录，`cargo public-api` 基线记其定义位簇路径（如 `error::shape::AxError`）；本 crate 经 `kernel` 顶层重导出引用，公共拼写不变，住处是 kernel 内政（card-2.1 起 `memory::checkpoint::Checkpoint` 住 `fence` 同例）。

replay、verify、VerifiedLedger、VerifiedLine、fork prefix、`at_seq`。不引入「重播/回放/复演」等同义词。

## 7 模块边界

```
replay ──▶ kernel(event/ledger/error)、memory(jsonl::read_raw_lines)
fork   ──▶ replay(VerifiedLedger)、kernel
turn   ──▶ kernel(ledger/event/error/tool/model)、prefix(FrozenPrefix)
prefix ──▶ kernel(locator::B3Hash/event::Payload/error)
handoff──▶ kernel(locator/event/error)
pipeline ──▶ offload、sieve、clock、kernel(tool)
sieve  ──▶ offload(tee)、memory(cas 读前一次原文)、kernel(tool::ExecArm/locator)
offload ──▶ memory(cas)、kernel(locator)
watchdog ──▶ kernel(stall/completion)
catalog ──▶ kernel(tool)、mode
sandbox ──▶ wasmtime（feature `wasm` 内藏；缝声明恒在）
tools/ ──▶ kernel(tool/version/discard/gate)、sandbox、memory(cas 经 pipeline)
```

**本 crate 本版不做什么（否定式三条）**：
- 不重执行任何效果——verify 恒不调工具、不出网、不写盘。
- 不生成 RunId——新 Run 身份由调用方注入（kernel 禁随机的同一纪律）。
- 不读 projection——重放的唯一输入是 Ledger 原始行（历史只有一份）。

## 8 接口先行（按模块分章）

### 8-1 runtime::replay（S1.10）

```rust
pub enum VerifiedLine {
    Known { record: EventRecord, echo: EventRef },   // echo：第二铸造点产物
    IgnoredUnknown { seq: Seq },                     // ig:true 的未知 kind
}
pub struct VerifiedLedger { /* 私有：lines: Vec<Vec<u8>>, verified: Vec<VerifiedLine> */ }
impl VerifiedLedger {
    pub fn raw_lines(&self) -> &[Vec<u8>];
    pub fn lines(&self) -> &[VerifiedLine];
    pub fn tail_seq(&self) -> Option<Seq>;
}
/// Offline chain verification (A2). Errors carry the failing line number in
/// `subject`. Refuses: v > EVENT_LOG_V (direction-aware), broken prev chain,
/// seq gaps, non-canonical bytes, unknown kind without `ig:true`.
pub fn verify_lines(lines: Vec<Vec<u8>>) -> Result<VerifiedLedger, AxError>;
/// Convenience over a jsonl directory: memory::jsonl::read_raw_lines + verify.
/// 无段目录与空账本在此同形（均得空 VerifiedLedger）——本函数的调用方均自持城根算出路径；
/// 区分二者是「从人那里拿到路径」的一层的事（§11；sprawling-SPEC §12）。
pub fn verify_ledger_dir(dir: &Path) -> Result<VerifiedLedger, AxError>;
```

流程：逐行①envelope 探查（serde_json::Value：v/seq/prev/kind/ig 键）；②v 判向（>EVENT_LOG_V 即 `E_LOG_VERSION_UNSUPPORTED`）；③链续（`chain_hash` 复算对拍 prev，首行对 GENESIS_PREV）；④seq 连续（自 FIRST 起）；⑤kind 已知→`parse_line` 全解＋规范复验＋`to_ref`；未知＋`ig:true`→记 IgnoredUnknown；未知无 ig→`E_LOG_VERSION_UNSUPPORTED`（subject=kind＋行号）。链与 seq 对一切行（含 ignored）成立。

**「没找到要验的东西」与「验过且为空」必须异形，但不在这一层异形**（issue #3）。`verify_ledger_dir` 的四个生产调用方（`fold`、`rebuild_views`、`startup_scan`、`fork`）均自持城根算出路径，而 `JsonlLedger::open` 只建目录、首次 append 才建段：**已开未写的城恰好是一个无段目录**，在此处报错会把一个合法启动打红（`fold` 早已以 `if ledger_dir.exists()` 记下这个状态）。若改成在此报错，四个调用方就各需一份同样的守卫——一条条件四份拷贝。

故判据归给**拿到人输入路径的那一层**：`sprawling replay <ledger-dir>` 先问 `memory::ledger_segments_at`，一段都没有就报 `E_PATH_NOT_FOUND`（sprawling-SPEC §12）。先例取自本仓库：`xtask guard` 在无提交时说 `no commits yet, nothing to judge`，而不说通过。**空账本本身仍然合法**：`verify_lines(vec![])` 照旧返回空 `VerifiedLedger`。

### 8-2 runtime::fork（S1.10）

```rust
/// Byte-identical fork prefix (A19): raw lines 0..=at_seq of the verified
/// mother sequence. `at_seq` past the tail is E_INVALID_ARGS, never a
/// silent clamp to the end.
pub fn prefix(mother: &VerifiedLedger, at_seq: Seq) -> Result<Vec<Vec<u8>>, AxError>;
/// The run_forked draft for the city Ledger. Caller supplies the new run
/// id and clock reading; fork itself is pure.
pub fn fork_draft(from: RunId, at_seq: Seq, new_run: RunId, t: TimeMs, who: String)
    -> Result<EventDraft, AxError>;      // data = {"from": …, "at_seq": …}
```

### 8-3 runtime::turn（S2.01；形状 5 typestate 机）＋bench／window（V3.39）

**V3.39：一个 typestate 机、一张工作台、一份会话历史，三种形状住一个文件。** 1,716 → 918（turn）＋740（bench）＋109（window）。
- `bench`（形状 1 判定）：`ToolBench`／`BenchOutcome` 与三条承重次序（去重先于副作用；exec 的 discard 预报先于 Write 门；Deny 以 `tool_result` 回去而不结束回合）。它拥有的是**次序**；工具本身以 `Box<dyn Tool>` 递入，沙盒在缝上，副作用不归它。
  **它原本坐在第一个 `mod tests` 之后**——一个文件长成这样的机制在 V3.30 已经记过：没有边界的地方，新代码落在光标所在的行。
- `window`（形状 2 值）：`Window`／`Opening`。一条不变量在每一个入口上成立——**连续的 user 内容并进已开的那条消息，而不另开一条**；steer、工具结果与开场任务是同一条规则的三扇门。
- 公开面只改定义模块，**不新增任何根重导出**（`runtime::Opening` 原就在根上；`ToolBench` 原就只能走模块路径，今天仍然）。

```rust
pub struct Turn<S> { /* run、who、t、state —— 全私有；相内数据在别的相不可表示 */ }
pub struct Assembling(/* 私有 */);  pub struct Calling { /* prefix 哈希 */ }
pub struct ToolWave { /* calls */ }   pub struct Recording { /* refs */ }

#[non_exhaustive] pub enum Interrupt { None, Cancel }    // Steer variant 随 S3 只加（消费方传值不 match，开放无痛）
pub enum PhaseOutcome<Next> { Advanced(Next), Cancelled(TurnCancelled) }
// PhaseOutcome 刻意穷尽：新结局必须逼每个执行器表态，不得掉 catch-all；
// 14.3 的 non_exhaustive 规则辖 wire 冻结枚举，不辖判定输出（verdict 枚举全库同此例）。
pub struct TurnCancelled { /* refs：含 cancel_received —— 私有，getter 取 */ }
pub struct TurnReport { /* refs、model_returned_ref、wave_len —— getter 取 */ }

impl Turn<Assembling> {
    pub fn begin(run: RunId, who: String, t: TimeMs) -> Turn<Assembling>;
    /// Boundary 1 (组装前). Cancel here consumes before any model bytes.
    pub fn assemble(self, interrupt: Interrupt, ledger: &mut dyn Ledger, prefix: &FrozenPrefix)
        -> Result<PhaseOutcome<Turn<Calling>>, AxError>;          // 产 prompt_assembled
}
impl Turn<Calling> {
    /// Boundary 2 (provider 调用前).
    pub fn call(self, interrupt: Interrupt, ledger: &mut dyn Ledger, model: &mut dyn Model,
                policy: &BuildingPolicy) -> Result<PhaseOutcome<Turn<ToolWave>>, AxError>;
                                                                  // 产 model_called＋model_returned
}
impl Turn<ToolWave> {
    /// Boundary 3 (工具执行前). One wave, serial in S2; per call
    /// tool_called + tool_result. Invoker is a plain closure — no second
    /// dispatch trait until a second consumer exists (S3 catalog).
    pub fn execute(self, interrupt: Interrupt, ledger: &mut dyn Ledger,
                   invoke: &mut dyn FnMut(&ToolCall) -> Result<ToolOutcome, AxError>)
        -> Result<PhaseOutcome<Turn<Recording>>, AxError>;
}
impl Turn<Recording> {
    pub fn record(self, interrupt: Interrupt, ledger: &mut dyn Ledger) -> Result<PhaseOutcome<TurnReport>, AxError>;
}
```

- **相变函数携 `&mut dyn Ledger`，相内字段私有**；无返回既往相的方法；跳相／相内取消／字面量构造中间相，三者编译不过（trybuild，S2.01 随卡、S2.11 入全集）。
- **取消只在边界**：每相变函数首参即边界快照；命中 Cancel → 追加 cancel_received → 返回 Cancelled（回合终止，后续 handoff_written＋run_frozen 归执行器）。相内无任何中断入口＝A9 的结构化一半；另一半（事件序断言）在 citysim。
- **四取消点**：组装前／provider 调用前／工具执行前／派生前，四点全住本模块。第四点由 `Turn<Recording>::record` 收边界快照，故 `record` 与前三相同形——收 `Interrupt`、答 `PhaseOutcome`。它买到的是别处买不到的一件事：**一个回合把活派下去之后、子 Run 起来之前，仍停得住**；`calls_made == 0` 的收尾回合尤其如此，那一刻在第四点之前根本没有下一个边界。
- **model_called 载荷**：segments 哈希（与 prompt_assembled 同源）；model_returned 载荷＝message＋calls 数。S3 接真 dialect 时只加字段。
- **工具波 S2 串行**：并行执行串行入账（确定性 5）属 S3 并发波；接口不预留并发参数，入账序＝calls 序。

### 8-4 runtime::prefix（S2.02；形状 5＋2）

```rust
pub enum SegmentSlot { City, Building, Resident, Run }   // 四段恒四，穷尽不扩
pub struct FrozenSegment { /* slot、bytes、hash —— 私有 */ }
impl FrozenSegment {
    /// The only way in: static bytes from frozen sources. Volatile types
    /// (TimeMs, usage, signals) have no conversion into this type — the
    /// absence of those impls is the isolation guarantee (15.3-4).
    pub fn new(slot: SegmentSlot, bytes: Vec<u8>) -> FrozenSegment;   // hash＝B3Hash::digest
    pub fn slot(&self) -> &SegmentSlot;  pub fn hash(&self) -> &B3Hash;  pub fn bytes(&self) -> &[u8];
}
pub struct FrozenPrefix { /* 四段 —— 私有 */ }
impl FrozenPrefix {
    /// Slot order is the type: city, building, resident, run. A mismatched
    /// slot in any position is E_INVALID_ARGS (fail-closed, no reorder).
    pub fn assemble(city: FrozenSegment, building: FrozenSegment,
                    resident: FrozenSegment, run: FrozenSegment) -> Result<FrozenPrefix, AxError>;
    pub fn segment_hashes(&self) -> [B3Hash; 4];
    pub fn prompt_payload(&self) -> Result<Payload, AxError>;   // prompt_assembled 载荷：逐段 {slot, hash, len}
}
```

- 段序即缓存经济：类型把四段位置写死，断点与各段上限属 S3 完备化（只加字段）。
- 分段哈希经 `B3Hash::digest`（kernel 唯一哈希产地）；A4（同输入同字节）由 golden 断言，A15 重建器随 S3。
- trybuild 反例：`FrozenSegment::from(TimeMs)`／把 TimeMs 传进 assemble —— 无转换路径，编译不过（ClockStamp 等类型落地后同规逐个加反例）。

### 8-5 runtime::handoff（S2.02；形状 2）

```rust
pub struct Handoff { /* must_read、overview、progress、context、next_step —— 私有 */ }
impl Handoff {
    /// Sole constructor: must-read non-empty; every
    /// entry is an already-parsed Locator by type. Five sections always
    /// present; prose quality is the probe's business (P1), not the type's.
    pub fn new(must_read: Vec<Locator>, overview: String, progress: String,
               context: String, next_step: String) -> Result<Handoff, AxError>;   // 空 must_read → E_INVALID_ARGS
    pub fn must_read(&self) -> &[Locator];  pub fn payload(&self) -> Result<Payload, AxError>;  // handoff_written 载荷
}
pub struct ResumeSeed { pub run: RunId, pub must_read: Vec<Locator> }
/// Resume consumes a Handoff and mints a new identity — never revives the
/// frozen one (元原则六). The caller supplies the new RunId (kernel 禁随机).
pub fn resume(handoff: &Handoff, new_run: RunId) -> ResumeSeed;
```

- 「下一步」段首列用户指定动作、must-read 规范类机器填：内容约束属生产者（S3 回合层／P2 spine_files），类型只强制结构。
- Run<Frozen> 无解冻：resume 不收 Run 值，只收 Handoff——「旧 Run 醒来」在签名上无法拼写。

### 8-6 S3.08 turn／prefix／handoff 完备化（形状不变，参数长入）

「只加不改」的取义：typestate 四相、边界消费、事件序、私有字段三不变量不动；相变函数的入参按 S3 语义长入（assemble 增 window/tools），消费者（citysim）同集更新。被否替代：平行第二条 call 路径——同一相两个入口即两个权威，落选。

```rust
// kernel::model 增（缝上 canonical 会话类型，kernel-SPEC §8-24 同集改）：
// ChatRequest { system: Vec<SystemBlock>, messages: Vec<ChatMessage>, tools: Vec<ToolDef> }
// SystemBlock { text, cache }；ChatMessage { role, content: Vec<ContentBlock> }；Role { User, Assistant }
// ContentBlock { Text{text} | ToolUse{id,name,input:Payload} | ToolResult{tool_use_id,content,is_error} }
// ToolDef { name, description, input_schema: Payload }；ModelUsage 四整数；StopReason { EndTurn, ToolUse, MaxTokens }
// ModelRequest 增 chat: ChatRequest；ModelReturn 增 usage: Option<ModelUsage>、stop: Option<StopReason>、billed: Option<UsdMicros>

pub struct Window { /* messages: Vec<ChatMessage> —— 私有；执行器持有，逐回合推进 */ }
impl Window { pub fn new() -> Window;
    pub fn push_steer(&mut self, source: &str, text: &str);          // 「user」或「@ID」前缀形
    pub fn push_task_lines(&mut self, task: &str, goal: &str, opening: Opening);  // 首轮，run_started 可重建
pub enum Opening { FromJob, WithPerson }   // P6.03：穷尽两臂，城在写 brief 时已决定
    pub fn push_assistant(&mut self, content: Vec<ContentBlock>);
    pub fn push_tool_results(&mut self, results: Vec<ContentBlock>); // ToolResult 块（pipeline 产出的成品文本）
    pub fn messages(&self) -> &[ChatMessage]; }

pub struct CallShape { pub model: String, pub max_tokens: u64, pub effort: Option<Effort> }
                    // P1.10：三项全部来自选型点，无一项在调用处手写。model 与 max_tokens 解自
                    // 模型目录行（gateway::market::ModelEntry）；effort 解自 kernel::FrozenConfig，
                    // Run 内恒不变——改它就换缓存前缀（理由与出处在 kernel-SPEC §8-22）
impl Turn<Assembling> {
    pub fn assemble(self, interrupt: Interrupt, ledger: &mut dyn Ledger, prefix: &FrozenPrefix,
                    window: &Window, tools: &[ToolDef], shape: &CallShape)
        -> Result<PhaseOutcome<Turn<Calling>>, AxError>;   // Calling 相私持已组 ChatRequest；prompt_assembled 载荷长入
}
// Interrupt 增 Steer { source: String, text: String }：边界消费→追加 steer_received（in-window）→照常 Advanced（不终止回合）；
// 文本回折入 Window 归执行器（它持 Window 与 Steer 原文），呼应「追加在结果末尾」。
// TurnReport 长入：model_content: Vec<ContentBlock>（助手内容）与 wave_results: Vec<ContentBlock>（ToolResult 块）——
// 执行器据此折叠 Window；离线重建同源于 model_returned.data.content 与 tool_result 事件（C16 一致）。
// kernel::ToolCall 增 id 字段（S3.08 同集）：tool_use↔tool_result 对号是两 Dialect 的 wire 硬性要求；
// tool_called 载荷增 id，tool_result 载荷增 tool_use_id。
```

**prefix 四段全量**（新增构建面；既有 FrozenSegment/assemble 不动）：

```rust
pub struct SourceDoc { pub addr: Address, pub bytes: Option<Vec<u8>> }   // None＝缺失或不可读（跳过入账）
pub struct SegmentCaps { pub city: u64, pub building: u64, pub resident: u64, pub run: u64 }  // 字节上限；来源＝调用方（S3 取 STARTUP_BUDGET_TOKENS×4 的四均缺省，住 consts 消费侧不另设常量）
pub struct PrefixPlan { pub city: Vec<SourceDoc>, pub building: Vec<SourceDoc>,
                        pub resident: Vec<SourceDoc>, pub run: Vec<SourceDoc>, pub caps: SegmentCaps }
pub struct PrefixBuild { pub prefix: FrozenPrefix, pub notes: Payload }   // notes＝逐段 sources/skips/truncations（prompt_assembled 载荷入口）
pub fn build_prefix(plan: PrefixPlan) -> Result<PrefixBuild, AxError>;
```

- **首轮不再指向任何东西（P6.03）**：`JOB.md` 的正文已是 Run 段，故 `FULL READ:` 那一行与它携的 `cas:b3-…` 一起取消——城里没有一个工具解析得了内容哈希，而溯源在 Ledger 里已记两遍。`Opening` 的两臂不是排版偏好：被派了一件活的会话与正在和人说话的会话要的第一句话不同，而把人那句话包成 `Task:`／`Goal:` 表单，换回来的也是一张表单。
- **read 的两条路，差别在于谁选的（P6.04）**：**路径是模型选的，故受审**——`Address::parse` 杀穿越，`is_reserved` 杀保留子树（`E_GATE_DENIED`）；**catalog 里的名字是人选的**——楼的阅览室写下它时准入就已发生，故它解到的 skill 可以住在保留空间里。两条路共用一个参数，因为对模型而言它们是同一件事（把一份东西调到眼前）；**先问 catalog** ，一个同名文件不得遮蔽楼已经准入的 skill。
- **`Catalog::expand` 改答 `Expansion { Skill { addr }, Said { text } }` 而不是 `String`（P6.04）**：skill 展开成一个可打开的地址，其余展开成目录自己持有的正文；两者压成一个字符串时，调用方只能拿它去试解析成地址，而一段恰好能解析成地址的正文就会被当成文件打开。这个错误真发生了，是一条红测试拿住的。
- **正文不在 prompt 里，所以交出去而不是拒绝**：`render()` 只写每条的 disclosure，`expansion` 从未进过窗口。最初那版 `read` 对 mode 答「已在你的 prompt 里，没什么可打开」，是错的。
- **它是 `Catalog::expand` 自 S3.10 以来的第一个调用者**：在它之前，一栋楼的阅览室能报出一个 skill 的名字而永远交不出它。
- 跨段去重：同 addr 两段命中只装首次（段序 city→building→resident→run）；后段记 skipped{reason:"duplicate"}。
- 截断：文件超段位余额即截到边界，原处留 ASCII 标记 `[truncated: N bytes]`（prefix 面向英文窗口），恒不静默丢尾；标记字节从段预算先扣。
- 断点：`FrozenPrefix::system_blocks()` 产四块、逐块 cache=true＝断点恒 4＝`CACHE_BREAKPOINTS_MAX`，断点只落段界。
- A15 重建器：`replay::rebuild_prefix(data: &serde_json::Value, resolver: &dyn Fn(&Address) -> Option<Vec<u8>>) -> Result<[B3Hash; 4], AxError>`——从 prompt_assembled 载荷（逐源 {addr, kept, marker, dropped}）与同源文档重算逐段哈希对拍；resolver 以 Address 取文（钉版 oid 级解析随 checkpoint 接入升级，接口不变）。截断标记与拼接规则的唯一权威住 prefix.rs（pub(crate) 常量），replay 同 crate 复用不另拷。
- E_TOOL_OUTCOME_UNKNOWN 补写面（本卡随 replay 交付）：`replay::dangling_tool_calls(&VerifiedLedger) -> Vec<(RunId, Seq)>`（tool_called 后邈无同 run 的 tool_result 即 dangling）＋`replay::outcome_unknown_draft(...) -> EventDraft`（补写的 tool_result，携 E_TOOL_OUTCOME_UNKNOWN 错误体）；消费者＝resume 路径（S4 serve；台账登记）。
- handoff：S2 形已全（五段＋构造点＋resume 消费），本卡零改动；「下一步段首列用户指定动作」属生产者纪律（S3 执行器／P2 spine_files），类型不另加钩。
- 第四取消点（派生前）：S2 推迟，理由是当期无派生生产者，提前落地＝死入口＋不可测。`card-P1.01` 的 `collab::delegate_tool` 是那个生产者，故本点随 `card-P1.02` 落地：`SafePoint::BeforeSpawn` ＋ `Turn<Recording>::record(interrupt, ledger)`，装配层在 `Completion::Cancelled` 时清空派生台，**被取消的 Run 一件活也交不下去**。

### 8-7 runtime::pipeline（S3.09；形状 1＋组装处）

```rust
pub struct PackContext<'a> {
    pub cap_bytes: u64,                       // 窗口余量推导的本次上限（调用方算；恒 ≥ 提示句预算）
    pub stamp: Option<ClockStamp>,            // clock::StampGate 的产出；None＝不携
    pub net_notice: bool,                     // gate::egress 首次公网放行信号
    pub steer: Option<(String, String)>,      // (source, text)；上一边界消费到的 Steer
    pub offload: Option<OffloadSite<'a>>,     // None＝无 CAS 可用（纯截断退路）
}
pub struct Packaged { pub content: String, pub events: Vec<Payload> }   // events＝result_offloaded 载荷（入账归调用方）
pub fn package(result: &[u8], ctx: PackContext<'_>) -> Result<Packaged, AxError>;
```

- 定序：offload 恒先于截断；`len ≤ cap` →原样；`len > cap 且 len ≥ OFFLOAD_MIN_BYTES 且有 OffloadSite` → offload；否则纯截断（尾部留 `[truncated: N bytes]`，不入 CAS）。
- 信封三附件一处组装：正文后依序追加 clock 行／net_notice 行（恒一次：正在连接互联网提醒，英文定句）／steer 行（`user:`／`@ID:` 前缀）；三行字节不计入 cap（附件与负载分账，附件有自己的封顶常数在实现内断言）。
- 内容感知压缩分派表属 P3；本模块本期只持「原样／offload／截断」三臂，接口不预留分派参数。

### 8-8 runtime::offload（S3.09；形状 1；四不变量的独占定义处）

```rust
pub struct OffloadSite<'a> { pub cas: &'a mut memory::Cas, pub environment: &'a std::path::Path }
pub struct OffloadRecord { pub substitute: Vec<u8>, pub original: Locator, pub rest_path: std::path::PathBuf,
                           pub original_len: u64 }
pub fn offload(bytes: &[u8], cap_bytes: u64, site: &mut OffloadSite<'_>) -> Result<OffloadRecord, AxError>;
pub fn rematerialize(locator: &Locator, site: &mut OffloadSite<'_>) -> Result<std::path::PathBuf, AxError>;
```

- 四不变量逐条入断言：①先存后缩（入参恒为全量字节，cas.put 先于一切裁剪）；②替代体含提示句恒 ≤ 原件且 ≤ cap（提示句字节先扣）；③只有有损才存（调用者保证 len>cap 才进来；函数内再断言，违反＝E_INVALID_ARGS）；④替代体恒携 rest_path：物化只读文件于 environment，内容＝全量原件；命中既有 CAS 对象即直引（幂等）。
- 替代体形：头部字节＋`\n[offloaded: total N bytes; rest at <rest_path>; original <locator>]`；提示句 ASCII。
- rematerialize：rest_path 被外部清理后自 CAS 重建，字节一致（A7 第三断言）。

### 8-9 runtime::watchdog（S3.11；形状 1＋处置历史持有者）

```rust
pub struct Watchdog { /* corrections: u32、provider_failures: u32 —— 私有，逐 Run 一实例 */ }
#[non_exhaustive] pub enum Disposal { Proceed, CorrectiveSteer { text: String },
                                     BackOff { until: TimeMs }, Freeze { reason: FreezeReason } }
#[non_exhaustive] pub enum FreezeReason { Stall, ProviderRefused }
impl Watchdog {
    pub fn new() -> Watchdog;
    /// Consumes kernel::stall's verdict verbatim; never re-derives it.
    pub fn on_stall(&mut self, verdict: &StallVerdict) -> Disposal;      // 首次 Stall→CorrectiveSteer；再次→Freeze{Stall}
    pub fn on_provider_failure(&mut self, failure: &AxError, not_before: TimeMs) -> Disposal;
    pub fn fired_payload(&self, disposal: &Disposal) -> Result<Payload, AxError>;   // watchdog_fired 载荷（E_LOOP_SUSPECTED 的 carrier）
}
```

- 处置必分级：纠正 Steer 文本指名重复指纹；只有终局的处置被明拒。子 Run 监控：`Completion::Limit` 的呈现住 status.children（S3 类型已备，派生消费者 P2），本模块不重复存储子态。
- **card-11.9：provider 失败按 `AxError::is_retriable` 分类，不再数次数。** 不可重试→`Freeze { ProviderRefused }`，一次即止；可重试→`BackOff { until }`，`until` 由调用层从 `AdmissionState::admit` 取得（provider 自己的 retry-after 已在那里取大者）。可重试的失败**永不自行冻住**：停它的是 `Halt`，城里唯一的刹车。
- **为什么删掉 `WATCHDOG_PROVIDER_RETRIES=2`。** 一个计数器对两种截然不同的失败给同一份预算：`E_WIRE_MISMATCH`（对端不说这个形状）重试三次就是把同一个 400 买三遍，而 429 重试三次就放弃又恰好把一个只需要等待的维护窗口当成了死亡。`retriable` 是产错处已经知道的事实（默认 false，fail-closed），拿它分类比在这里重新猜一遍强。
- **`ProviderExhausted` 改名 `ProviderRefused`。** 既然没有重试预算了，就没有东西被耗尽；冻住的原因是对端给了一个重试不能修复的答复。载荷里的 `reason` 字串同改为 `provider_refused`。
- S3.11 落地记录：fired_payload 字段＝{action: steer|back_off|freeze, text|until_ms|reason, corrections, provider_failures}；Proceed 拒绝成帐（无事不记）；纠正只发一次（corrections 计数），第二次 Stall 即冻——分级穷尽于 steer→freeze 两级，「停滞中间态」不另设（它就是 Stall verdict 本身）。`provider_failures` 留下作为**观察**（这个 Run 碰上了几次），不再是一个阀值。

### 8-10 runtime::clock（S3.10；形状 1；纯格式化不采样）

```rust
// 关切时区的权威住 kernel::config::ClockZone（[clock] zones 属三层配置）：
// FrozenConfig 增 clock_zones: Vec<ClockZone>，freeze 增梯入参；本模块只消费不定义（一个权威）。
pub struct ZoneEntry { pub id: String, pub offset_min: i32, pub local: String }   // local＝"YYYY-MM-DD HH:MM"
pub struct ClockStamp { pub utc_ms: TimeMs, pub zones: Vec<ZoneEntry> }           // utc_ms 已按桶截断
impl ClockStamp { pub fn render(&self) -> String }   // 信封与人读共用的唯一文本形："clock: utc …; <id> …;"
pub fn stamp(now: TimeMs, zones: &[ClockZone]) -> Result<ClockStamp, AxError>;   // zones > CLOCK_ZONES_MAX → E_INVALID_ARGS；UTC 行恒首；空表即只报 UTC

pub struct StampGate { /* granularity、last_bucket: Option<u64> —— 私有；last_bucket 兼任首发标记 */ }
impl StampGate {
    pub fn new(granularity: ClockStampGranularity) -> StampGate;
    /// Emission rule: Off -> never; first result of the
    /// run -> once; Timestamped -> every result; Timeless -> only when the
    /// granularity bucket changed since the last emission.
    pub fn observe(&mut self, now: TimeMs, temporal: Temporal, zones: &[ClockZone])
        -> Result<Option<ClockStamp>, AxError>;
}
```

- 历法纯整数（civil-from-days，无 chrono 依赖；界证明携 `#[expect]`）；戳内容按 granularity 桶截断（同桶同字节，Timeless 去重因此有义）。A18 零字节：Off 时 observe 恒 None。

### 8-11 runtime::catalog（S3.10；形状 6＋渲染）

```rust
pub struct CatalogEntry { pub name: String, pub disclosure: String, pub expansion: String,
                          pub hash: Option<B3Hash> }   // V3.27：架上那份文档被读到时的哈希
pub struct SkillPin { pub name: String, pub hash: B3Hash }   // V3.27
pub struct Catalog { /* tools: BTreeMap<ToolName,…>、skills: BTreeMap、mode: Option<Mode> —— 私有 */ }
impl Catalog {
    pub fn new() -> Catalog;
    pub fn admit_tool(&mut self, meta: &ToolMeta) -> Result<(), AxError>;      // disclosure 非空；重名＝E_INVALID_ARGS
    pub fn admit_skill(&mut self, entry: CatalogEntry) -> Result<(), AxError>; // 只收阅览室准入者（准入求值归 city::policy，P1；本期调用方直供）
    pub fn set_mode(&mut self, mode: Mode);                                    // 只列本 Run 所处者
    pub fn render(&self) -> String;              // Resident 段的 catalog 部分：段头一行自述＋一行一件；BTreeMap 序恒定
    pub fn tool_defs(&self) -> Vec<ToolDef>;     // ChatRequest.tools 的唯一来源
    pub fn expand(&self, name: &str) -> Option<&str>;   // 第二级披露（怎么用）
    pub fn skill_pins(&self) -> Vec<SkillPin>;   // V3.27：本 Run 拿到了哪几份，当时各是什么字节
}
```

- **`hash` 是 `Option`，而那个 `None` 不是「没算」**：目录里另有两类条目的正文由本构建自己握着（mode 的纪律、dev 那一条），它们背后没有一份能在无人看着时改掉的文档。
- **pin 从 catalog 取，不重扫一遍书架**：catalog 已经是「本 Run 能够到什么」的权威，再扫一次就是在另一个时刻对同一个问题给第二个答案。

**P6.02：`render()` 与 `set_mode()` 接线**。它们自 S3.10 写下就**一个生产调用者都没有**，只有自己的测试在调。后果不是「少了一行文字」：工具走 `ChatRequest.tools` 到得了模型，而**阅览室准入的 SKILL 与本 Run 所处的 mode 从未到达任何模型**——`city::library` 的准入判定因此是一道没有下游的门。

接法：`Catalog::render()` 追在 `identity.segment_bytes()` 之后，合成 Resident 段。**不另开第五个槽**：一个居民能够伸手取到什么，与它是谁同属一类常住事实，且两者都随 Run 冻结，故前缀在整个 Run 的寿命里仍可缓存。装配层因此把 prefix 的组装移到目录建好之后。

**实测（一次真机派活）**：Resident 段 106 B → 1,176 B，差额 **1,070 B**（八件工具加一个 mode）。同一份派活下，模型被问「你被告知了哪些能力」时逐个点名 `archive, edit, exec, goal, plan, pr, signal, status`——其中 `plan` 就是 mode 的那一条，它在本卡之前从未被任何模型看见过。

**第二级披露今天仍不可达，原因写在这里而不是留给人撞**：SKILL 的 `expansion` 是 `city::holding_address()` 给的一个地址，坐在**保留前缀 `.sprawling/` 下**，而本构建的工具台里**没有读文件的工具**（`edit` 只改不读）。故 `render()` 故意不印那个地址：叫一个模型去读它取不到的东西，比不告诉它更坏。补齐它需要一件读工具，那是一项新能力而不是本卡的缺陷修复。

### 8-12 runtime::mode（S3.10；形状 6；P6.05 增 dev 入口）

```rust
pub const DEV_ENTRY: &str = "dev";
pub fn dev_entry() -> CatalogEntry;   // 一行披露，全部细则归 expansion
```

- **一个 Run 只被告知它所在的那个 mode**，于是没有任何 Agent 知道这座城自己的代码与 SPEC 是可改的。`dev` 行补上这一句，**而且只补一句**：三个模式的定义、阅读次序（SPEC → 代码 → 旁边的测试）与「下一步去跟人要模式」全在 expansion 里，由 `read` 按需取。**大多数会话不改这座城，就只付一行的价。**

### 8-12b runtime::mode 原有面（S3.10）

```rust
#[non_exhaustive] pub enum Mode { PlanGoal, Up, Sc, Ud, Experiment }
impl Mode { pub fn as_str(&self) -> &'static str;              // "plan_goal" | "up" | "sc" | "ud" | "experiment"
            pub fn catalog_entry(&self) -> CatalogEntry }      // 含 PlanGoal 退出条件四列
```

### 8-13 runtime::sandbox（S3.12；缝清单文件，形状 3＋4）

```rust
pub struct Fuel(pub u64);
pub struct Mount { pub host: std::path::PathBuf, pub guest: String, pub writable: bool }   // preopen＝mount scope
pub struct SandboxJob { pub wasm: std::path::PathBuf, pub argv: Vec<String>, pub env: Vec<(String, String)>,
                        pub stdin: Vec<u8>, pub mounts: Vec<Mount>, pub fuel: Fuel }
pub struct SandboxOutcome { pub stdout: Vec<u8>, pub stderr: Vec<u8>, pub exit: SandboxExit }
#[non_exhaustive] pub enum SandboxExit { Success, Failure { code: u64 }, FuelExhausted, Trap { message: String } }
pub trait Sandbox { fn run(&mut self, job: &SandboxJob) -> Result<SandboxOutcome, AxError>; }

pub struct WasmtimeSandbox;            // feature = "wasm"；wasip1 直跑（先按 preview1 落地）
pub struct EchoSandbox { /* 直通替身：stdout＝stdin 回声＋可注入脚本输出 */ }
pub struct FaultSandbox { /* 故障替身：逐次弹出预置 SandboxExit／fuel 耗尽／trap */ }
#[cfg(feature = "conformance")]
pub fn assert_sandbox_conformance<S: Sandbox>(sandbox: &mut S, job: &SandboxJob);  // 良序两连调不中毒＋outcome 形合法
```

- 能力面＝wasip1 preopen 集（Mount 逐条）；无网络能力（WASI p1 天然无 socket 宿主实现——Python 臂禁网的机械保证）；fuel 上限即 Fuel（耗尽＝FuelExhausted，不是 Err：宿主无故障）。
- 未授能力被拒的观察形：guest 内 open 失败→非零退出（Failure）；宿主恒不代 guest 隐藏失败。A10 三断言在真 wasmtime 上以手写 WAT 模块定形（不依赖 CPython 工件）；CPython-WASI 集成测试以环境变量指向工件（住机器本地的忽略目录，恒不入库），缺工件即 skip——`just check` 自足。
**A10 三断言结论书**（S3.12；证据＝`crates/runtime/tests/sandbox_a10.rs`，真 wasmtime 48.0.0，手写 WAT 不依赖任何外部工件）：

| 断言 | 观测形 | 结论 |
|---|---|---|
| fuel 内成功 | `fd_write` 写 `ok\n`，Fuel(1_000_000) | `Success`，stdout 逐字节相符 |
| 未授能力被拒 | 无 preopen 时 `path_open` 失败→guest 自行 `proc_exit(7)` | `Failure{code:7}`；**授予同一目录后同一 guest 转 `Success`**——此对拍使「被拒」是能力判定而非测试损坏 |
| fuel 耗尽中断 | 无限循环，Fuel(10_000) | `FuelExhausted`，且是 `Ok` 非 `Err`（宿主无故障） |

- **无出网的机械形需精确化**（S3.12 实测修正本 SPEC 原措辞）：wasip1 **确有** `sock_send`／`sock_recv`／`sock_accept`／`sock_shutdown` 宿主实现（对非 socket fd 恒返 `ENOTSOCK`）；它没有的是**获得** socket 的途径——无 `sock_open`／`sock_connect`／`sock_bind`。故导入 `sock_connect` 的 guest 直接链接失败（`E_SANDBOX_DENIED`），而 guest 能拿到的每一个 fd 都来自 preopen（均为目录）。这才是 Python 臂禁网的准确依据。
- **失败不得以默认值擦除**（rust-hardening Gate 5）：`try_into_inner()` 取不回管道、`get_fuel()` 报不出余量、退出码超 WASI 范围——三者均属**宿主故障**，恒返 `Err`；若以 `unwrap_or_default()`／`unwrap_or(1)` 兑成「空输出」「未耗尽」，就是把猜测冒充事实。
- **代价实测**：开 `wasm` feature 后 runtime 依赖面由 71 个 crate 增至 257 个（+186）；debug 下 `libwasmtime_wasi.rlib` 单件 186 MiB。故 feature 内藏是必要的，非可选修饰。
- 待办（不属本卡）：CPython-WASI 工件接入与体积实测回填性能册——工件住机器本地的忽略目录、恒不入库，缺工件即 skip，`just check` 自足。
- 门的缺口：`just clippy` 取 `--all-targets` 而无 `--all-features`，故 feature 内藏的代码逃过零警告门；本卡已手动跑 `clippy --features wasm,conformance --all-targets` 零发现。扩到 `--all-features` 要改 justfile，而 justfile 在 guard 辖区内。

- wasmtime 钉 48（2026-08 复核：含 GHSA-2r75-cxrj-cmph（path_open TRUNCATE 绕过，修于 44.0.2/45.0.0）与 CVE-2026-58494（hard-link/rename FilePerms 绕过，修于 45.0.3/46.0.1）两处修复——「钉版恒含权限绕过修复」的判据实例）。

**P4.02 增（sandbox）**：`AbsentSandbox` —— 未带执行引擎的构建在缝上的产品实现，逐次以 `E_TOOL_UNAVAILABLE` 拒并携替代臂。它存在的理由是**缺席要是一个判词而不是一个替身**：Echo 放在这个位置会对一个从未运行的 guest 回答「成功」，而第一个察觉的人是相信了那份输出的人。

### 8-14 runtime::tools 四件（S3.13 三件＋P6.04 read；形状 4；tools.rs 为纯索引）

```rust
// tools/exec.rs —— 三臂（ExecArm 住 kernel::tool）
pub struct ExecTool { /* workdir、mounts、python_wasm: Option<PathBuf>、sandbox: Box<dyn Sandbox>、
                        shell: Option<PathBuf>、fuel —— 私有；全由装配／执行器注入 */ }
impl ExecTool { pub fn new(…) -> ExecTool; }
impl Tool for ExecTool { /* meta：name=exec、effect=Write{domain}、temporal=Timestamped、render=Terminal */ }
// Program 臂：std::process::Command（workdir 钉定、环境变量白名单——secret 恒不透传）；本期唯一真子进程产地
// Python 臂：sandbox.run(python_wasm, argv=["python","-c",code], mounts)；组件缺失→E_TOOL_UNAVAILABLE＋alternative＝Program 臂
// Shell 臂：探测缺失即拒（E_TOOL_UNAVAILABLE，不是降级）；存在则 sh -c／cmd /C

// tools/edit.rs —— base_version 乐观并发＋写域双闸＋创建臂（整修卡 R1.02）
pub struct EditTool { /* city_root、writable: WriteDomain —— 私有 */ }
impl Tool for EditTool { /* meta：name=edit、effect=Write{domain}、render=Diff、temporal=Timeless */ }
// new(city_root, addr, writable: WriteDomain)：writable＝该 Run 的写域（rules.write_domain()）。
// 每次调用先判路径后碰盘：Address::parse 杀穿越（..／绝对路径／空段），WriteDomain::admits 杀域外与
// reserved prefix（E_OUTSIDE_WRITE_DOMAIN，recovery 报可写前缀清单）。此前只有工具静态声明过门，
// 模型选的 path 未经任何判定直接落盘——那是一个真漏洞，修在权威处而非 bench 里的第二份判定。
// args：{path, base_version, old, new}；version＝内容 B3Hash 前 16 hex；check_base 拒即 E_VERSION_CONFLICT；
// old 必唯一命中（零命中／多命中＝E_INVALID_ARGS 携计数）；回显＝unified diff＋new_version（逐次 diff 即回档粒度）

// tools/read.rs —— 一个参数，两条路（P6.04）
pub struct ReadTool { /* city_root、catalog: Rc<RefCell<Catalog>> —— 私有 */ }
impl ReadTool { pub fn new(city_root: &Path, catalog: Rc<RefCell<Catalog>>) -> Result<ReadTool, AxError>; }
impl Tool for ReadTool { /* meta：name=read、effect=Read、cost=Light、render=Generic、temporal=Timeless */ }
// args：{path}。先问 catalog，再当作地址。
// 创建臂：base_version=="new"（16 hex 永拼不出，无碰撞）→ 文件必不存在（存在＝E_VERSION_CONFLICT 报真实版本），
// old 必 ""，new＝全文；父目录自动建（域内已证）。缺文件而非创建形的拒词指向创建形；
// 缺参拒词报四字段契约。理由：没有创建能力的城里，Agent 在空房间里无法开始任何工作；
// 创建住 edit 而非新工具，因为「文件变更＋乐观并发」已是本工具拥有的唯一权威，“absent”只是版本的一个取值。

// tools/status.rs —— 十三字段（P3.06 追加第十三行）
pub struct StatusSnapshot { pub who: String, pub addr: Address, pub mode: Mode, pub ctx_used: Tokens,
    pub ctx_limit: Tokens, pub budget_usd: UsdMicros, pub budget_tokens: Tokens, pub trust: String,
    pub write_domain: String, pub locks: Vec<String>, pub worktree_path: String, pub worktree_disk: ByteLen,
    pub signals_pending: u32, pub children: Vec<ChildStatus>, pub now: Option<ClockStamp>,
    pub provider_mode: ProviderMode, pub neighbours: u32 }   // neighbours 在末尾，渲染序与声明序同一
#[non_exhaustive] pub enum ProviderMode { Normal, Degraded, LocalOnly }
pub struct ChildStatus { pub room: Address, pub kind: DelegateKind }   // P1.03：重塑
pub struct StatusTool { /* snapshot＋ children: Box<dyn Fn() -> Vec<ChildStatus> + Send> ＋ backlog: Option<Backlog> */ }
impl StatusTool {
    pub fn watching(snapshot: StatusSnapshot, children: Box<dyn Fn() -> Vec<ChildStatus> + Send>) -> Result<StatusTool, AxError>;
    pub fn reporting(self, backlog: Backlog) -> StatusTool;   // §8-28-2：末行 `backlog:` 从表里现读，与 children 同一理由
}
impl Tool for StatusTool { /* meta：name=status、effect=Read、temporal=Timestamped、render=Generic；渲染序末尾追加 backlog 一行 */ }

// ToolBench 住 turn.rs（S3.13 同卡加入）：按 Effect 过门是回合层职责（Handoff 裁定 10），不另立 bench 模块。

- **`children` 为何重塑（P1.03）**：旧形状 `{run, phase, ctx_used, ctx_lock}` 预设子已在跑。真实情形是子 Run 在父嚽结之后才开，故父自己那一跑里 **子既无 run id 也无上下文读数**——四个字段里三个只能填零，而零与未知是两件事。现形状只携得出口的两件：派到哪个房间、哪一类代理。
- **`neighbours` 追加在末尾而不插入到 `signals_pending` 旁边（P3.06）**：冻结序存在的理由是字段表增长时居民的习惯仍可迁移，而一次插入会把前十二行里的一半挪位。它只报**人数**不报名单：名单长度随人口增长，而 `status` 是一份定长文本（`render_children` 已为同一条理由被压成一行）；详情归 `neighbours` 工具，city-SPEC §8-15b。
- **数的是人，不是地址**：一间没人站着的房间没有读者，把它计入会让 `neighbours: 3` 读起来像「有三个人可以说话」而实际上一个都没有。空房间仍然在工具的答案里，因为它对 delegate 与搬入是真信息。
- **`children` 是闭包而不是快照字段**：派活发生在 `status` 工具造好之后，一份开跑前拍的快照永远是空的。派生台住 `collab`，而 depmap 不允许 runtime 依赖 collab，故本模块只收一个答「现在派了哪些」的闭包，装配层把台接上去——与 `RunHooks` 四个闭包同一纪律：第二实现不存在时不引 trait。
// runtime::compaction（P3.14；形状 6 数据面＋形状 1 判定）
pub enum Content { Prose, Code, Diff, Log, Structured, Table, Unknown }   // 七类，Unknown 是其中之一
pub enum Strategy { Keep, Head, Ends, Tail, Offload }
pub fn detect(text: &str) -> Content;                       // 前几行上的前缀与计数，顺序即设计
pub fn plan(content: Content, size: ByteLen, budget: ByteLen) -> Strategy;
pub fn compact(text: &str, budget: ByteLen) -> (String, bool);
// 硬不变量：结果恒不大于输入，且在出口再验一次（真长了就退回原文）。切口落在字符边界。
// Structured 与 Unknown 恒不截断：被截断的 JSON 比缺席的 JSON 更糟；未知内容不拿猜测去丢东西。
// 机制面（把大结果移出窗口）仍归 offload——本模块只答「缩不缩、留哪一头」。

// runtime::mode 的准入面（P3.08；形状 1 判定）
pub struct Produced { pub tests_passed: Option<bool>, pub contract_moved: bool,
                      pub held_in: Option<bool>, pub held_out: Option<bool> }
pub enum Admission { Lands, Refused { because: &'static str, alternative: &'static str } }
pub fn admits(mode: Mode, produced: &Produced) -> Admission;
// `None` 不是 `Some(false)`：「没测」与「测了没过」是两件事。证据以 bool 入参而非 eval 的类型，
// 因为 eval 在本 crate 之外，而这里问的不是证据怎么来的，是够不够。UD 是唯一要双验证的模式。

// runtime::redact（P3.04；形状 1 判定）——入口只有一个：`Turn::call` 写 model_returned 之前。
// 窗口块已在此前取出，故思考块签名不受影响；历史与上下文是两个汇，只有一个是永久的。
// 替换物是 `secret:redacted/<b3-16>` 标记而非 Vault 条目：模型复述的钥匙不是城被托付保管的凭证，
// 存它等于给它一条没人要求过的命，而哈希前十六位已足以看出两处是否同一个值。
pub fn redact(payload: &Map<String, Value>) -> (Map<String, Value>, u32);
pub fn redact_text(text: &str) -> (String, u32);

pub struct ToolBench { /* tools: BTreeMap<String, Box<dyn Tool>>、domain: WriteDomain、registry: Registry、
                          taint: TaintSet、seen: BTreeSet<IdemKey>、prior_public_egress: bool —— 私有 */ }
impl ToolBench {
    pub fn new(domain: WriteDomain, registry: Registry) -> ToolBench;
    pub fn register(&mut self, tool: Box<dyn Tool>) -> Result<(), AxError>;
    /// Gate routing by declared Effect（Handoff 裁定 10：收进回合层，executor 归还薄形）：
    /// exec 先 forecast（Suspected → **强制 checkpoint 先行**，见下）；Write → domain 门；
    /// Egress → egress 门（target 由调用自报 host，不自报即 E_INVALID_ARGS）；Spend → 门已接但 P1 前无实例；
    /// Spawn → delegation 门（恒 Escalate；granted 命中即放行，未命中即 Pending）；
    /// Govern → govern 门（同形；提案正文由调用的 `text` 参数取，进 action_desc 供人过目）；
    /// Deny → 以 refusal 作 tool_result 回流（不吞掉回合）；Escalate → BenchOutcome::Pending 回流（S3 无应答面）。
    pub fn invoke(&mut self, call: &ToolCall, key: &IdemKey, ctx: &GateContext)
        -> Result<BenchOutcome, AxError>;
    /// R2.20b：上面那句「按 Effect 过门」自己的名字。`None` 即此门已开，
    /// 工具可跑；`Some` 即门已代这次调用给出答案，工具不跑。私有，公开面不变。
    fn admit(&mut self, call: &ToolCall, name: &str, effect: &Effect, ctx: &GateContext)
        -> Result<Option<BenchOutcome>, AxError>;
    /// 一扇门的判定对本 bench 意味着什么。三处 Escalate 的「人已经允过的
    /// cluster 不再问第二遍」原本各写一遍，现在只住这里。
    fn settled(&self, outcome: GateOutcome) -> Option<BenchOutcome>;
    /// 同形：两处出网判定的「首次公开出网要记下来」只住这里。
    fn crossed(&mut self, outcome: EgressOutcome) -> Option<BenchOutcome>;
    /// S3.14 长入：包信封的调用者要读 `temporal` 才知道时钟行该不该发。
    pub fn meta_of(&self, name: &str) -> Option<&ToolMeta>;
    /// card-2.2：栅栏署名随网一起交给 bench。一个没有 `Provenance` 的栅栏
    /// 写不出 `Sprawling-Run:`，而预测栅栏恰恰是在一个拿不到运行上下文的
    /// 闭包里升起的，所以它在装配时就被交下。
    pub fn with_checkpoint(self, net: CheckpointNet) -> ToolBench;
    /// 栅栏的三件东西恒同行：仓、它盖住的范围、写它的人。
    pub struct CheckpointNet { pub checkpoint: Checkpoint, pub scope: String, pub of: Provenance }
    /// P1.04：本 bench 服务的那份活。Spawn 门要铸一个人答得出的条目，
    /// 条目要有 actor（问谁）与 artifact（看什么）；两者都不在一次工具调用里。
    /// 未给即拒（fail-closed）——一个人问不到的派生就是没人批准的派生。
    pub fn for_job(self, asking: Address, job: Locator) -> ToolBench;
    /// P3.03: a cluster the person already allowed. An escalation whose
    /// cluster is granted runs instead of parking, which is what lets an
    /// answer carry the work on rather than send it back to the door it
    /// was just let through. Granted per **cluster**, because that is the
    /// unit the person was shown and answered in.
    ///
    /// The caller folds these from `approval_resolved`; the bench does
    /// not read history, because it runs inside a drive that owns the
    /// ledger and a second reader of that would be a second answer.
    pub fn grant(&mut self, cluster: ClusterKey);
}
```

- L0 三件恒列 prefix（City.md 只放这一级）；catalog 只收 L2——L0 不进 catalog（名字即文档）但 tool_defs 恒含三件（wire 面要 schema）。
- **S3.13 语义修正（红转绿抓出）**：本 SPEC 期初写的「Suspected → Discard 门」与 kernel 既有设计冲突，以 kernel 为准。理由：`DiscardRequest` 只有 `Planned`／`Unplanned` 两变体，而 `decide` 对 `Unplanned` **恒判 Deny(NoRestoration)**——把 forecast 的预判包成 Unplanned 送进门，等于让任何含 `rm ` 的 exec 调用全被拒。`kernel::discard` 的注释早已写明正确意图：「text prediction is obfuscatable by design — hits route conservatively, and the git checkpoint net (S3) is the honest backstop」。故 **Suspected 不拒而围栏**：强制 `checkpoint.wave_pre` 先行再放行，删掉的东西因而可回档；**无 checkpoint 网时才拒**（`E_TOOL_UNAVAILABLE`），因为「无保护地跑」是唯一没人选择的结局。此路由使 A14 的先行半链在 exec 臂上机械成立。
- ToolBench 持 `Option<Checkpoint>` 具体类型而非新 trait：checkpoint 只有一个实现，为尚不存在的第二实现引缝会造空抽象（AGENTS.md：trait 只在已有第二实现的缝上引入）。
- **`BenchOutcome` 去掉 `#[non_exhaustive]`（整修卡 R2.16）**：它是判定输出，而本文对 `PhaseOutcome` 写的规则已经辖到它——「14.3 的 non_exhaustive 规则辖 wire 冻结枚举，不辖判定输出」。全工作区扫一遍：`kernel` 的八个判定枚举无一标它（`budget.rs` 行内写着「Deliberately exhaustive verdict enum」），**`BenchOutcome` 是唯一的例外**，于是两个下游各背着一条永不执行的 `_ =>`——而那正是 §7「新增一种答案而不回答它就不编译」要护的东西。收口是编译红：摘掉属性即得两条 `unreachable pattern`（citysim 一条、sprawling 一条），`-D warnings` 下即错，删掉它们才绿。下游从此必须穷尽匹配四臂。
- **三条规则各回到一处（整修卡 R2.20b）**：`invoke` 曾为 246 行，是 `length` 门报出的两个对象之一。拆它时量到三处重复：① `GateOutcome::Escalate` 的「granted 命中即放行」写了**三遍**（Write／Spawn／Govern）；② `EgressOutcome` 的「首次公开出网要记下来」写了**两遍**（Connector／Egress）；③ `serde_json::to_vec` 扫描参数写了两遍。三条都是规则而不是巧合：一份人给过的允许在三个地方各有一份实现，就是三个可以各自漂走的权威。归位后：`settled` 一处、`crossed` 一处，`admit` 成为那个 `match &effect` 自己的名字。尺寸 246 → 65（`admit` 139、`settled` 11、`crossed` 13）。行为逐字不变，公开面不变。
- `BenchOutcome` 四态：`Ran{outcome, fenced}`（fenced 携围栏 oid，供波后补记）／`Refused{refusal}`（回流不终止回合）／`Pending{item}`／`Duplicate`。dedup 先于任何副作用；**key 在过门之后才记入 seen**，故被门拒的调用重试不算重放。
- status 的 result 是**按冻结序渲染的文本**而非 JSON 对象（S3.13 红转绿抓出）：`serde_json::Map` 对键排序，JSON 对象没有读者可依赖的序，「冻结序」会悄悄变成字母序。序是「模型读到的东西」的属性，故落在模型读到的地方。
- 注意本期无 Egress／Spend 工具实例（出网代理 P1）：两门路由代码落地以测试替身驱动，接线台账仍记「计划内待接」至真实例出现。

### 8-15 runtime::run（P1.01；形状 5 typestate 机；**run 事件序的唯一权威**）

```rust
pub struct Run<S> { /* plan、window、turns、last_turn_t —— 全私有 */ }
pub struct Active(/* 私有 */);   pub struct Frozen { /* completion、turns */ }

pub struct RunPlan {                 // 一个 Run 的全部常量，调用方先备齐
    pub run: RunId, pub who: String, pub addr: Address,
    pub task: String, pub goal: String, pub job: Locator,
    pub opening: Opening,                                 // 这一场是接了写下来的活，还是人在场
    pub parent: Option<RunId>,                            // P1.03：派活给它的那个 Run
    pub predecessor: Option<RunId>,                       // card-11.6：把这场活交给它的前任，深度守恒
    pub shape: CallShape,
    pub prefix: FrozenPrefix, pub policy: BuildingPolicy, pub tools: Vec<ToolDef>,
    pub skills: Vec<SkillPin>,                            // V3.27：阅览室准进了什么，当时各是什么字节
}
```

- **`budget_turns` 与 `budget` 两栏已随 card-11.7 删除**：回合上限与花销天花板都不存在了，一跑循环到它自己结束为止；停一件正在跑的事是 `Cancel`，停一片是 `Halt`。
- **`skills` 写进 `run_started` 载荷，且无条件写**（空则空数组），理由与当年 budget 两栏同一条：一个时有时无的 key 是一个读者得猜的形状，而「这栋楼一个都没准进」本身就是一件值得记下的事。进账本而不只留在进程里，是因为「它变了没有」需要一个**早一次的读取**，而进程一走就只剩账本说得出这一轮到底拿到了哪些字节。

#[non_exhaustive] pub enum SafePoint { BeforeAssemble{turn:u32}, BeforeCall{turn:u32}, BeforeWave{turn:u32}, BeforeSpawn{turn:u32} }
pub enum Advance { Turned, Concluded(Completion) }        // 穷尽；新结局逼每个调用方表态

pub struct RunHooks<'a> {            // 四个闭包，不是四个 trait：本模块只有一个消费者形式
    pub now: &'a mut dyn FnMut() -> Result<TimeMs, AxError>,        // 时间入参，本模块恒不采样
    pub interrupt: &'a mut dyn FnMut(SafePoint) -> Interrupt,       // 安全点由我定，信号由你答
    pub fence: Option<&'a mut dyn FnMut(TimeMs) -> Result<Payload, AxError>>,  // 波前 checkpoint
    pub invoke: &'a mut dyn FnMut(&ToolCall, TimeMs) -> Result<ToolOutcome, AxError>,  // 回合时间戳随行
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

- **为什么要这个模块**：「Dispatch → N 回合 → 冻结」的事件序先前只存在于 `citysim::executor`。真城再写一遍就是两个权威，而两者一旦漂开，**仿真继续绿而真城错**——仿真的全部价值恰好建立在它跑的是同一份代码上。故 citysim 改为本模块的调用方，23 剧本从此直接验证生产回路。
- **`run_started.parent`**（P1.03）：只在派生开的 Run 上出现。父子关系先前只存在于「两行相邻」这个巧合里，而相邻不是一个可查询的事实；写进载荷之后，前端折得出树，离线重放也折得出同一棵树。
- **`run_started.usd_micros` 与 `run_started.tokens`**（整修卡 R2.11）：一跑被派出去时的花销天花板。写它与写 `parent` 同理——**一个进程死后，「这跑当时允许花多少」只剩账本能回答**。先前它只活在装配层的一个局部变量里，于是一跑因待批而停下、被批准后续上的那一跑天花板归零（sprawling-SPEC §8-23 查出、§8-25 修复）。两个键是 `u64` 整数，符合确定性第六条；`fixtures/golden-p0` 随本卡重生（`GOLDEN_WRITE=1`），这是它存在的用法。
- **时间纪律**：dispatch 采两次（checkpoint、run_started），每回合一次；**自然结束与预算耗尽时 freeze 再采一次**，handoff 用它、run_frozen 用它＋1（两行同一件事，不值两次采样）；**取消时 freeze 沿用被打断那个回合的时间戳**，因为这次冻结属于那个回合而不是一件新事。三条合起来使一个计数器闭包（citysim）与一个壁钟闭包（真城）在同一驱动下各自正确。
- **结束判定**：`calls_made == 0` 即 `Completion::Done(Evidence[model_returned])`；跑满 `budget_turns` 即 `Completion::Limit`；任一安全点命中 Cancel 即 `Completion::Cancelled`。三条均经 `freeze` 出口，故 **handoff_written＋run_frozen 是唯一出口**，无第二条退路。第四点 `BeforeSpawn` 与前三点同权：命中即 `Cancelled`，那个回合的 assistant 与 tool results **不入窗**，因为窗口前推是「回合成立」的后果而不是它的一部分。
- **第四种结束：回合中途的失败（前端会话 3，实测修正）。** 上一段那句「无第二条退路」先前在代码里不成立：`drive` 的错误臂只对带 `Carrier::Event` 的码写载体事件并冻结，对 `Carrier::Loadtime` 的五个码直接 `return Err`，**那次 run 被丢掉、账本上只剩 `run_started`**。实测：ModelScope 的流式 tool_call 拼接缺陷令每次派活死于 `E_WIRE_MISMATCH`，重启后 `city_view` 仍报那次 run `frozen: false`，页面于是把每条消息都当 `steer` 发而被拒（tmpcity2 的 `99785458-…`）。**改判：两种 carrier 都经 `freeze` 出口，差别只在冻结之前写不写载体事件。** 理由：`Loadtime` 原先的理由是「账本自身就是受害者时，没有什么真实的东西可写」——这对 `CasCorrupt`／`StorageFatal`／`LogVersionUnsupported` 成立，对 `WireMismatch` 不成立：供应方把兑换格式写错与账本健否无关。而对前三个码，写不进去的后果就是 `freeze` 的 append 自己失败并把那个失败向上抩——这比预先判定「写不进去」更诚实。冻结后**原错误仍然向上抩**：账本得到判决，调用方得到诊断，两件事不互相替代。否决「把 `WireMismatch` 重分类为 `Carrier::Event(ProviderDegraded)`」：该码在握手期也用于 wire 版本不匹配（那时连 run 都不存在），一个码两种含义去改分类表，会让 `kernel::event::kind` 那条「loadtime 白名单封死在五个」的测试变成对一件无关的事作证。
- **Window 归驱动持有**：入窗内容就是回合报告的前推结果（assistant＋tool results），放在调用方手里等于把一条不变量交给每个调用方自己维护。
- **四个闭包而非四个 trait**：第二实现尚不存在，而本库的纪律是 trait 只在已有第二实现的缝上引入（同 8-3 的 invoke 闭包）。`RunHooks` 自身只是四个引用的容器，不持策略。
- **字节不变是本卡的验收判据**：同一堆剧本、同一份 `fixtures/golden-p0`，换了驱动实现而字节不动——这才能证明“提取”是提取而不是重写。

### 8-16 runtime::digest（P1.07；形状 1 判定＋形状 2 值类型）

```rust
pub struct StructureNode { pub level: u8, pub title: String, pub offset: ByteLen, pub span: ByteLen }
pub struct Digest { /* source、origin、structure、prose —— 私有 */ }
impl Digest {
    pub fn structural(source: B3Hash, origin: Option<Locator>, text: &str) -> Digest;
    pub fn with_prose(self, prose: String) -> Digest;      // 消费并返回：带 prose 的是另一个值
    pub fn is_suspect(&self) -> bool;  pub fn window_header(&self) -> String;
}
pub fn structure_of(text: &str) -> Vec<StructureNode>;      // 纯、全函数

pub struct Breaker { /* limit、consecutive */ }
pub enum BreakerVerdict { Attempt, Open { after: u32 } }
pub enum DigestOutcome { Cached(Digest), Fresh(Digest), Structural { digest: Digest, reason: AxError } }
pub fn digest_once(text, origin, breaker, cached: &mut dyn FnMut(&B3Hash) -> Result<Option<Digest>, AxError>,
                   write_prose: &mut dyn FnMut(&str) -> Result<String, AxError>) -> Result<DigestOutcome, AxError>;
```

- **结构是读出来的，prose 是写出来的**：前者机械可复现，与原文恒不冲突；后者恒带 `suspect`，**没有清除该标记的方法**——摘要不会因为被读两遍就不再是摘要。
- **`window_header` 把可疑说在读者看得见的地方**，并给出原文位置：与原文冲突时以原文为准，这条要能被执行而不只是被相信。
- **一个内容哈希一生只摘一次**：顺序即全部策略——先哈希、再问缓存、再读结构、最后才花一次模型调用；熔断打开时连那一次也省掉。
- **熔断按次数不按时间**：判定路径里放墙钟会毁掉重放，而「连续三次失败」是调用方可复现的事实。一次成功即复位：间歇性的 provider 与坏掉的 provider 是两种情况。
- **失败不升级为错误**：`Structural{digest, reason}` 仍带完整结构树——摘要失败的文档仍然是一份有标题的文档。
- **模型调用归调用方**：本模块决定问什么、信什么，`bin::assembly` 持有 provider（同 `runtime::run` 的钩子形状）。

### 8-17 runtime::diagnostics（P1.13；形状 4 薄壳＋形状 6 数据面）

设计权威是 `docs/logging.md`；本节只记接口与三处口径差异。

```rust
#[non_exhaustive] pub enum Level { Refuse, Effect, Decide, Trace, Wire }  // 全序：层底控到该级为止
impl Level { pub const DEFAULT: Level = Effect; pub const ALL: [Level; 5]; pub fn parse(&str) -> Option<Level>; }
pub struct Site<'a> { pub run: RunId, pub seq: Seq, pub module: &'a str }   // 三字段必填
pub type Sink = Box<dyn FnMut(&str) + Send>;
pub struct Diagnostics { /* floor: Option<Level>、sink —— 私有 */ }
impl Diagnostics {
    pub fn new(floor: Level, sink: Sink) -> Diagnostics;
    pub fn off() -> Diagnostics;
    pub fn floor(&self) -> Option<Level>;
    pub fn admits(&self, level: Level) -> bool;
    pub fn write(&mut self, level: Level, site: Site<'_>, message: &str);
    // 无读方法。这是本模块全部保证的形状半边
}
pub fn redact(&str) -> String;   pub const REDACTED: &str = "secret:redacted";
```

- **无读方法即全部形状保证**：「判定与恢复逻辑不读日志」不靠纪律，靠这一点——把一行读回来在类型上拼不出。推论就是收口条件：删光日志，行为、重放与总账逐字节不变。
- **行上恒无时间戳**（与 `docs/logging.md` 早期口径的差异，已回写该文）：锦点是 `seq`——两条时间线靠一个整数对齐，而采样壁钟会在一个不允许采样的库里开第二个时间源。想要时间的 sink 在装配层自己加。
- **坐标由 Ledger 自己说**：`memory::JsonlLedger::position()`（本卡新增，返回「现在写一条会落在哪」）。只给位置不给内容：一个能读记录的访问器会把判定逻辑引到它正在写的账上去。
- **双重防线**：`Sealed` 无 Debug/Display，入行在类型层就不成立（反例 `tests/ui/log_a_credential.rs`）；普通字符串里的明文由 `kernel::scan`——**同一个**扫描器，不是第二个——就地换成 `secret:redacted`。不丢整行：周围那句话通常正是读者要的。
- **不引 `tracing`**：它在此处的唯一功能是跨 `await` 携模块名的 span，而回合路径是同步的，该功能今天无消费者。理由已回写 `docs/logging.md` §7。
- **写入方三处**（§6 的三类各一）：命令被拒（`refuse`，写在 `handle` 而非调用方，因为每个调用方都要）；endpoint 附着与探测结果（`effect`）；dispatch 跑完（`effect`，作为指向 Ledger 的指针）。

## 8.5 两个设计

**第二对（S2，turn 侧）**：中断作相变入参（选中）vs 独立 `cancel()` 方法。后者表面更直观，但 cancel 方法可在任意持有点被调＝相内中断可表示，A9 退化成时序约定；选中方案把边界快照做成相变函数的形参，相内无入口，结构即断言。代价：调用方每相必须显式给 Interrupt（哪怕 None）——这个啰嗦是刎意的：它迫使执行器在每个边界问一次信号面。

**首对（S1）：fork 消费 VerifiedLedger**——分叉前必先验链，类型上把「从未验证的序列分叉」做成不可表示；分叉正确性与重放正确性因此是同一条断言。
**B（落选）：fork 直接吃原始行**（`prefix(lines: &[Vec<u8>], at_seq)`）——少一次验证成本，但打开「对损坏历史分叉」的路径，且 at_seq↔行号对应要自行重解 envelope＝第二解析权威。落选理由：验证成本 O(n) 在分叉频率下可忽略，而不变量 14（citysim 检查器）需要的正是 A 的类型保证。另 `verify_dir` 命名族落选：与 `verify_ledger_dir` 二选一，取后者（dir 一词泛滥易撞 S3 worktree 面）。

## 9 工作流程

`just replay <log>`（S1.11 接线 bin）→ `verify_ledger_dir` → 全绿报 tail_seq／行数，违规报 three-part。citysim 检查器与 A19 测试直接调 `verify_lines`／`prefix`。

## 10 实现逻辑

envelope 探查与全解共用 kernel 的解析（Value 探查仅取五键，不建第二记录类型）；行号从 1 计（人读）；错误 recovery 字段给「重放同一夹具于更新版本」或「检查介质」两句可执行建议。

## 11 边界枚举

空序列（合法：VerifiedLedger 空，tail_seq=None；fork 于其上恒越界）；**目录存在但不含任何账本段**（在本模块合法且与空账本同形；人输入路径的拒绝在 CLI）；单行创世；`at_seq=FIRST`（前缀＝仅创世行）；`at_seq=tail_seq`（前缀＝全量）；ig:true 且 kind 已知（照常全解，ig 只授未知时的跳过权）；篡改中段一字节（链断于下一行报错）；两段夹具跨段验证（memory 读面已拼平）。

## 12 错误处理（逐码答「能否定义掉」）

- `E_INVALID_ARGS`（at_seq 越界）：不可定义掉——「从已冻结 Run 最后事件之后分叉」是用户可达输入（§19.1 点名）；静默夹取是被明拒的替代。
- `E_LOG_VERSION_UNSUPPORTED`（v 判向＋未知 kind 无 ig）：不可定义掉——数据比二进制长寿。
- 链断/seq 洞/非规范字节：以 `E_CAS_CORRUPT` 报（存储完整性族；subject=行号与路径）——能否定义掉＝「介质位腐烂在设计边界外」，同 memory-SPEC §12。

## 13 依赖选型

kernel、memory（读面＋S3 增 cas 消费）；serde_json（envelope 探查）。dev：proptest、tempfile、trybuild、insta（S3 增：prefix golden）。
S3 增：`wasmtime = "48"`（feature `wasm` 内藏，钉版理由见 §8-13；wat 为 dev 依赖供 A10 模块）；`similar`？否——unified diff 自写最小形（edit 回显只需逐行对照，不引第三方 diff 库；被否理由：依赖面换一处 80 行纯函数，不值）。其余无新第三方（分段哈希经 kernel `B3Hash::digest`，不直依 blake3）。

## 14 硬编码声明

无（行号计法与 recovery 文句不构成行为常量）。

S3 增两处 pub(crate) 数据面（改须本 SPEC 同集；`WATCHDOG_PROVIDER_RETRIES=2` 已于 card-11.9 删除，理由见 §8-9）：信封附件封顶 `ENVELOPE_ATTACH_MAX_BYTES=1024`（§8-7：附件与负载分账的断言界）；net_notice／truncation／offload 提示句三定句（ASCII，住 pipeline／offload 实现内，改句＝改入窗字节＝过本 SPEC）。

## 15 影响面

citysim 链检查器（S1.11）复用 verify_lines；bin `replay` 子命令接线（S1.11）；S2 prefix 重建器将消费 VerifiedLedger——接口本期定形，只加不改。
S3：assemble 签名长入波及 citysim 执行器（同集更新）；kernel::model 增 canonical 类型波及 ScriptModel；ToolBench 收走 executor 门闭包（归还薄形）；E_TOOL_OUTCOME_UNKNOWN 补写面（replay 新增 dangling 检测）供 resume 路径消费。

## 16 测试与约束

单测：五步各拒绝分支＋ig 跳过；fork 越界；fork_draft 载荷形。proptest：对任意合法 draft 序列（经内存 Ledger 物化）verify 恒过；任意单字节翻转恒拒。A2/A19 演示测试入 crates/runtime/tests/。约束：clippy 零告警。
S3 增：A4 golden（build_prefix 重跑逐字节同）；A15（rebuild_prefix 对拍）；A7 往返四断言；A18 零字节；watchdog 分级序；A10 三断言（feature `wasm` 下真 wasmtime＋WAT）；L0×失败注入矩阵（三臂×（正常／工具错／拒收））；ToolBench 门路由（Deny 回流／Escalate 回流／dedup 先于副作用）。

## 17 模型体验

零字节：replay/fork 是离线设施；其产物（分叉 Run 的入窗历史）经 S2 prefix 组装间接入窗，本模块自身不产生任何 prefix 字节。

## 18 文档同步

ARCHITECTURE §6 runtime 表逐卡状态翻转（turn/prefix/handoff 随 S2.01/S2.02；S3 九模块逐卡）；接线台账同 PR 登记；S3 完备化的「只加不改」取义见 §8-6（三不变量不动，相变入参按语义长入，消费者同集）；kernel-SPEC §8-23/§8-24 随 S3.01/S3.13 同集改；api-baseline 随每张改公开面的卡重算。

### RunHooks 多一个：说到一半的话往哪去（V3.13）

```rust
pub struct RunHooks<'a> {
    pub now: ...,
    pub interrupt: ...,
    pub fence: ...,
    pub invoke: ...,
    pub deltas: Option<&'a mut (dyn FnMut(&str) + 'a)>,
}
```

**`None` 是承重的，不是缺省值。** 一个增量改变不了 run 的任何判断，所以「没人看」的驱动器就不向 provider 要流：`Turn::call` 在 `None` 时走 `Model::call`，字节与从前一模一样。citysim 与离线重放因此一字未改——**这是这条改动不碰确定性的全部理由**。

**它不返回 `Result`。** 增量不是判断：下游任何东西都不得据它分支，而一个能拒绝的 sink 会让一个显示细节有能力弄失败一次调用。

**写进账本的那句话只从 `ModelReturn` 来。** `model_returned` 的载荷此前怎么写，现在还怎么写——增量恒不参与拼装它。于是「页面看到的」与「账本保存的」不可能出自对同一个回复的两次读法；流被切断表现为读取错误，永不表现为一个变短的回答。

**`Turn::call` 的 `'sink` 是显式命名的。** 调用方（`drive`）持有 sink 跨越整个 run 并把它交给每一轮；生命周期省略时，重借需要收缩 trait object 自己的生命周期，而 `&mut` 不允许。这不是风格，是这个签名必须显式的原因。

### 8-18 runtime::turn 目录化（card-5.2）

**919 行一个文件 → 334（`turn.rs`）＋44＋56＋100＋8＋109＋271＋81。** 切法是「测试迁出＋簇切」，逻辑一行未改，函数签名与公开面逐字节不变。

| 文件 | 管什么 |
|---|---|
| `turn.rs` | typestate 载体 `Turn<S>` 与四个相类型，`assemble`／`call`／`record`，以及唯一的中断消费点 `consume_boundary`。`assemble` 与 `call` 带 `argument_count` 豁免，故留在原路径 |
| `turn/boundary.rs` | 执行器在相变处交出什么、相变答什么：`Interrupt`、`PhaseOutcome`、`TurnCancelled` |
| `turn/report.rs` | 一轮跑完交给 run loop 的东西与它被给定的调用形状：`TurnReport`、`CallShape` |
| `turn/wave.rs` | 边界 3：`impl Turn<ToolWave>` 的工具波，按调用序串行 |
| `turn/tests.rs` | 纯索引（`mod helpers; mod phases; mod window;`） |
| `turn/tests/helpers.rs` | 三处共用的夹具：`TestLedger`、`OneShotModel`、`prefix`／`run_id`／`shape`／`advance`／`probe_call` |
| `turn/tests/phases.rs` | 四个边界跑在真账本链上（5 个 `#[test]`） |
| `turn/tests/window.rs` | 开场白与 steer 在窗口里留下什么（3 个 `#[test]`） |

**开放的字段（均为 `pub(super)`，仅供 `turn.rs` 构造）**：`TurnCancelled.refs`；`TurnReport.refs`、`TurnReport.model_returned`、`TurnReport.calls_made`、`TurnReport.assistant`、`TurnReport.wave_results`。构造点仍只有 `consume_boundary` 与 `Turn::<Recording>::record` 两处，getter 仍是唯一读法；`wave.rs` 不需要开任何字段，因为子模块本就能看见父模块的私有项。

**api-baseline 重写了。** `Interrupt`／`PhaseOutcome`／`TurnCancelled`／`TurnReport` 的定义位置移进私有子模块后，`cargo public-api` 改印 `lib.rs` 的再导出路径（`runtime::TurnReport`），而非 `runtime::turn::TurnReport`。是同一项换了规范路径，不是公开面变化：`pub use turn::{…}` 与 `pub mod turn` 都没动，调用方一行 `use` 未改。

### 8-19 runtime::bench 目录化（card-5.2）

**740 → 254（`bench.rs`）＋215（`bench/admit.rs`）＋288（`bench/tests.rs`）。** 形状与 8-3 的 bench 行一致（形状 1 判定），三条承重次序未动。

| 文件 | 管什么 |
|---|---|
| `bench.rs` | `ToolBench` 与 `BenchOutcome` 的定义、装配面（`new`／`for_job`／`grant`／`with_checkpoint`／`register`／`taint_mut`／`meta_of`）、`invoke` 的路由次序，以及 `kernel_error_from_memory` |
| `bench/admit.rs` | 门：`admit` 按 `Effect` 分派到 Write／Connector／Egress／Spawn／Govern 各门，`settled`／`crossed` 把一次判定翻译成 `BenchOutcome`，`scanned` 为两扇朝外的门备好密钥扫描的字节 |
| `bench/tests.rs` | 去重、门、fence 与注册冲突的夹具（7 个 `#[test]`） |

**无字段开放。** 子模块本就能看见父模块的私有项，故 `ToolBench` 的字段一个都没动；唯一的可见性改动是 `fn admit` → `pub(super) fn admit`，因为它现在由父文件的 `invoke` 调用。

**api-baseline 未重写。** 搬走的都是私有项，公开面逐字节不变。

### 8-20 runtime::replay 目录化（card-5.2）

**570 → 388（`replay.rs`）＋186（`replay/tests.rs`）。** 只做测试迁出：离线验证的生产代码本就在 400 行以内，不需要簇切，逻辑与函数签名一行未改。

| 文件 | 管什么 |
|---|---|
| `replay.rs` | `VerifiedLine`／`VerifiedLedger`／`Envelope`，以及 `verify_lines`／`verify_ledger_dir`／`rebuild_prefix`／`dangling_tool_calls`／`outcome_unknown_draft` |
| `replay/tests.rs` | 离线验证拒绝什么：更高的 `v`、无 `ig:true` 的未知 kind、漂移的 prefix 源文档、悬空的 tool_called（5 个 `#[test]`） |

**无字段开放。** 子模块本就能看见父模块的私有项，`mod tests` 上原有的 `#[allow(...)]` 清单原样搬到 `mod tests;` 声明上。

**api-baseline 未重写。** 公开项的定义位置未动，规范路径不变。

### 8-21 runtime::prefix 目录化（card-5.2）

**546 → 391（`prefix.rs`）＋159（`prefix/tests.rs`）。** 只做测试迁出：四段构建与冻结前缀的生产代码本就在 400 行以内，不需要簇切，函数签名、类型与公开面一行未改。

| 文件 | 管什么 |
|---|---|
| `prefix.rs` | `SegmentSlot`／`FrozenSegment`／`SourceDoc`／`SegmentCaps`／`PrefixPlan`／`FrozenPrefix`，以及 `build_prefix`／`build_segment`／`truncation_marker`／`DOC_JOIN` 与 `system_blocks`／`segment_hashes`／`prompt_payload` |
| `prefix/tests.rs` | 冻结前缀保证什么：槽位次序、同输入同哈希、跨段去重与跳过入账、截断标记与字符边界、四个缓存断点（8 个 `#[test]`） |

**无字段开放。** 子模块本就能看见父模块的私有项，`mod tests` 上原有的 `#[allow(...)]` 清单原样搬到 `mod tests;` 声明上。

**api-baseline 未重写。** 公开项的定义位置未动，规范路径不变。

### 8-22 runtime::tools::edit 目录化（card-5.2）

**522 → 335（`tools/edit.rs`）＋191（`tools/edit/tests.rs`）。** 只做测试迁出：乐观并发、写域双闸、创建臂与最小 unified diff 的生产代码本就在 400 行以内，不需要簇切，函数签名、类型与公开面一行未改。

| 文件 | 管什么 |
|---|---|
| `tools/edit.rs` | `EditTool` 与 `version_of`／`CREATES`：`new` 的参数模式声明、`Tool::invoke` 的判定次序（工具身份→地址→写域→版本→匹配数→落盘），创建臂 `create`，以及 `unified_diff`／`common_prefix`／`common_suffix` |
| `tools/edit/tests.rs` | edit 拒绝什么、回显什么：版本相符的落盘与 diff、陈旧版本、创建臂两种冲突、写域外与非法地址、缺文件的恢复话术、零次与多次匹配、错路由（9 个 `#[test]`） |

**无字段开放。** 子模块本就能看见父模块的私有项，`mod tests` 上原有的 `#[allow(...)]` 清单原样搬到 `mod tests;` 声明上。

**api-baseline 未重写。** 公开项 `EditTool`／`version_of` 的定义位置未动，规范路径不变。

### 8-23 runtime::run 目录化（card-5.2）

**468 行一个文件 → 225（`run.rs`）＋264（`run/lifecycle.rs`）。** run.rs 无测试，故做的是簇切：把 `impl Run<Active>` 整块搬进子模块，逻辑一行未改，函数签名与公开面逐字节不变。

| 文件 | 管什么 |
|---|---|
| `run.rs` | 一个 Run 的常量与状态类型（`RunPlan`／`SafePoint`／`Advance`／`RunHooks`／`Active`／`Frozen`／`Run<S>`）、`impl Run<Frozen>` 的三个读法、载荷构造 `payload`，以及驱动循环 `drive`。`drive` 带 `argument_count` 豁免，故留在原路径 |
| `run/lifecycle.rs` | 一个活着的 Run 在账本上做的三件事：`dispatch` 的调度对（job pin＋run_started）、`advance` 的一回合（四个安全点、波前围栏、报告前推入窗），以及唯一出口 `freeze`（handoff_written＋run_frozen）；连同只有 `advance` 用得上的 `fold_steer` |

**无字段开放。** 子模块本就能看见父模块的私有项，故 `Run`／`Active`／`Frozen` 的字段与 `fn payload` 的可见性一个都没动。

**`impl Run<Active>` 保持为一整块，不按 dispatch／advance／freeze 三分。** 先试过三个文件，`cargo public-api` 立刻按 impl 块计数印出六行 `impl runtime::run::Run<runtime::run::Active>`（原为两行）——那是公开面输出的变化而不是规范路径重拼，故收回为一个子模块一个 impl 块。

**api-baseline 未重写。** 公开项的定义位置未动，规范路径不变。

### 8-24 runtime::digest 目录化（card-5.2）

**449 行一个文件 → 321（`digest.rs`）＋132（`digest/tests.rs`）。** 切法只用了「测试迁出」：非测试部分本就在 400 行以内，逻辑一行未改，函数签名与公开面逐字节不变。

| 文件 | 管什么 |
|---|---|
| `digest.rs` | 摘要管线本身：`StructureNode`／`Digest`／`Breaker`／`BreakerVerdict`／`DigestOutcome`，纯函数 `structure_of` 与 `close_deeper`，以及唯一入口 `digest_once`。`digest_once` 带 `argument_count` 豁免，故留在原路径 |
| `digest/tests.rs` | 摘要对读者的四个承诺：标题树跳过代码围栏、模型写下的散文永远 suspect、同一内容哈希一生只摘要一次、熔断器计次开合（5 个 `#[test]`，与切前相等） |

**无字段开放。** 子模块本就能看见父模块的私有项，故 `Digest` 的 `source`／`origin`／`structure`／`prose` 可见性一个都没动。

**api-baseline 未重写。** 公开项的定义位置未动，规范路径不变。

### 8-25 runtime::sandbox 目录化（card-5.2）

**422 行一个文件 → 374（`sandbox.rs`）＋52（`sandbox/tests.rs`）。** 切法只用了「测试迁出」：非测试部分本就在 400 行以内，逻辑一行未改，函数签名与公开面逐字节不变。

| 文件 | 管什么 |
|---|---|
| `sandbox.rs` | 执行边界本身：`Fuel`／`Mount`／`SandboxJob`／`SandboxOutcome`／`SandboxExit`／`Sandbox` 缝，缺席判词 `AbsentSandbox`，两个替身 `EchoSandbox`／`FaultSandbox`，feature `wasm` 下的 `engine` 子模块（`WasmtimeSandbox` 与 `classify`），以及 feature `conformance` 下的 `assert_sandbox_conformance` |
| `sandbox/tests.rs` | 两个替身对调用方的承诺：直通替身回声 stdin 并记下 job、故障替身按序发脚本且发完即止（2 个 `#[test]`，与切前相等） |

**无字段开放。** 子模块本就能看见父模块的私有项，故 `EchoSandbox.scripted` 与 `FaultSandbox.scripted` 的可见性没动。

**api-baseline 未重写。** 公开项的定义位置未动，规范路径不变。

### 8-26 runtime::tools::exec 目录化（card-5.2）

**403 行一个文件 → 277（`tools/exec.rs`）＋130（`tools/exec/tests.rs`）。** 切法只用了「测试迁出」：非测试部分本就在 400 行以内，逻辑一行未改，函数签名与公开面逐字节不变。`ExecTool::new` 的 7 参豁免键 `crates/runtime/src/tools/exec.rs::new` 因而仍指着它原来的文件。

| 文件 | 管什么 |
|---|---|
| `tools/exec.rs` | 三臂本身：`ExecTool`（`new` 与 `run_program`／`run_python`／`run_shell`）、环境白名单 `ENV_ALLOWLIST`、结果打包 `outcome`／`exceptional`、臂解析 `parse_arm`，以及 `impl Tool for ExecTool` 的路由 |
| `tools/exec/tests.rs` | 每条臂对调用方的承诺：缺件时点名替代方案而拒绝、python 臂在沙盒里跑并报自己的退出码、燃料耗尽与 trap 原样抵达、无法识别的臂只拒不猜、program 臂真起子进程且环境被洗（5 个 `#[test]`，与切前相等）|

**无字段开放。** 测试是子模块，父模块的私有字段与私有方法本就可见。

**api-baseline 未重写。** 公开项 `ExecTool`／`parse_arm` 的定义位置未动，规范路径不变。

### 8-27 runtime::sieve（card-11.4；形状 1 判定＋形状 6 数据面；**本节是压缩器的唯一权威**）

> 裁决 D31。读这一节即可实现，不需要再查别处、不需要再做任何参数选择。方法论取自 Hypabolic/Hypa 的 ADR-0002 与其 `Compression/` 实现；四处刻意背离记在 §8-27-7。

#### 8-27-1 它是什么，以及为什么不是 LLM

`sieve` 按**产生结果的命令**决定留下什么。它与 `compaction` 相邻而不重叠：`compaction` 看文本形状（Prose／Code／Diff／Log／Structured／Table／Unknown），`sieve` 看命令身份（`cargo build` 与 `git status` 的噪声形状完全不同，而两者都是 Log）。

不用模型做压缩，理由是被架构强制的而非偏好：V6 要求同一颗种子重放出逐字节相同的对话，一个会思考的压缩器会让重放不可能。它同时省掉一次调用的钱与延迟。

#### 8-27-2 位置与法则

管线原法则「offload 恒先于 truncation」扩写为：

```
tee（原文钉进 CAS ＋ 实体化 rest 文件）
  → sieve（命令感知）
    → offload / truncation（既有三臂）
```

**tee 恒在最前**：这是 `offload` 既有的 store-before-cut 不变量上提一层。因为原文一定先落盘，sieve 才敢压得比 Hypa 狠——Hypa 那条「压缩率异常高时拒绝」的护栏存在，是因为它不能保证全文带内可恢复；本城可以。

#### 8-27-3 作用范围

**只作用于 `exec` 结果。** 命令感知的东西对没有命令的工具无可感知：`read` 结果、模型输出、MCP 结果各自走既有的 `compaction`／`redact` 路径，本节不动它们。

#### 8-27-4 七条不变量

1. 输出恒不长于输入。
2. 输入非空时输出非空。
3. 任何裁剪之前，原文已钉入 CAS。
4. 每个被筛过的结果都携带 rest 文件路径与 CAS locator。**压缩是强制的；可恢复也是强制的**——没有落 tee 的压缩不许发生。
5. 切口落在字符边界上。
6. **这条路上不用正则表达式**（沿用 `compaction` 模块头的既有裁决；判「重要」的四类模式全部手写线性扫描，见 §8-27-6）。
7. 账本记的是**模型看到的字节**＋原文 locator。重放复现模型看到的东西，不是命令打印的东西。

#### 8-27-5 阶段顺序与参数（全部已定，实现者不再选择）

| # | 阶段 | 参数 | 取值 |
|---|---|---|---|
| 0 | 地板 | `SIEVE_FLOOR` | **2 KiB**。以下原样通过，连 tee 都不做（此时 tee 的一次 CAS 写加一个实体文件，比省下的字节贵） |
| 1 | 去 ANSI | — | 恒开 |
| 2 | 空行折叠 | 连续空行 | **≥3 折为 1** |
| 3 | 模板去重 | 同模板出现次数 | **≥4 折成一行＋计数**；≤3 全留 |
| 4 | 跨调用差分 | 同 `(arm, path, args)` 本 run 内跑过且原文在 CAS | 只出新增／变化行，未变部分**一行**带过 |
| 5 | 过滤表 | 见 §8-27-6 | 命中即用，否则走通用路径 |
| 6 | 长行截断 | 单行长度 | **> 2 KiB 截断并标记** |
| 7 | 截断 | `MAX_TOTAL_LINES` | **240** |
| | | `HEAD` / `TAIL` | **40 / 40** |
| | | 中段保留上限 | **60 条，按优先级排序取前 60** |

每一级只在**不变长**时被接受；**被跳过的级要记进账**（Hypa 是静默跳过的，坏过滤器因而事后不可诊断）。

#### 8-27-6 重要行的优先级与过滤表

优先级序（同级按原始行序，稳定且确定）：

```
error  >  panicked / fatal / failure / failed  >  assertion  >  warning  >  note / help
```

判「重要」的四类模式，手写线性扫描：关键词子串加词边界；`路径:行:列` 形状；`大写字母2-4 ＋ 数字3-5` 的诊断编号；`test result:` / `exit code` 这类结果行。

**原样保留、任何阶段不得触碰**：URL、设备码形状的字符串、`secret:realm/name` 引用、带行列的文件路径、退出码。

过滤表住 `<city>/.sprawling/FILTERS.toml`，楼级可覆盖 `<building>/.sprawling/FILTERS.toml`，走既有三层阶梯且**整值解析而非逐字段合并**（直接沿用 kernel-SPEC §8-22 P4.02 给 `[sandbox]` 定的口径①，一条规则一个权威）。**信任问题不存在**：过滤表住保留区，没有任何写域够得着它。

形状（谓词只有 prefix / contains / suffix，无正则）：

```toml
[[filter]]
id          = "cargo"
command     = "cargo"
subcommands = ["build", "check", "test", "clippy", "nextest"]
strip_ansi  = true
drop_prefix = ["   Compiling ", "    Checking ", "    Finished ", "   Updating ", "    Blocking "]
keep_contains = ["error", "warning:", "panicked at", "test result:", "-->"]
head = 20
tail = 40
on_empty = "cargo {sub}: clean, exit {code}"
```

**内建编译进去的 reducer 恰三个**：`cargo`（含 rustc 诊断分组）、`git`、`generic`。其余一律走表——ADR-0002 自己把「命令专属 reducer 是持续维护负担」写在负面后果里，Hypa 列了十个，那张清单会长成泥潭。

页脚（**不报 token，不引分词器**——o200k 不是所接模型的分词器；真实 token 由 provider 在账上给）：

```
[sieve: 1,240 → 78 lines, 31.4 KiB → 2.1 KiB, filter=cargo, rest at ./.rest/rest-a91f.dat]
```

#### 8-27-7 四处刻意背离 Hypa

1. **重要行排序而非全留。** Hypa 的 `TruncationStage` 把中段所有命中 `ImportantLineClassifier` 的行全部保留；其 `\b[45]\d{2}\b` 会把 `compiled 437 files` 判成重要行，而 cargo 输出里几乎每个依赖都命中一次 `warning`。一次三百条 warning 的构建因此压了等于没压。本实现取前 60 条。
2. **模板级去重而非相邻去重。** Hypa 的 `DeduplicateStage` 只折叠连续相同的行；构建日志是交错的（`Compiling a` / `Compiling b`），一行都压不掉。归一化数字、哈希与路径后按模板分组，是 cargo 场景下最大的单项收益。
3. **跨调用差分。** Hypa 每次调用独立压缩，因为它的压缩路径没有会话模型。本城有账本与 CAS：开发循环里 `cargo check` 跑十遍，九遍与上一遍逐字节 90% 相同，只出差分能在压缩之上再降一个数量级——而且给模型的是**更好的信息**（「这个错是新出现的」本来要它读两遍才能得出）。
4. **无正则。** Hypa 的分类器整个是正则；本城 `compaction` 的模块头已裁决这条路上不用模式引擎。手写扫描约 40 行，更快且无回溯风险。

不抄的三样：它的十个 compiled reducer 清单（维护跑步机）、`Microsoft.ML.Tokenizers`（依赖加谎言）、SQLite 与 `hypa trust`（账本＋CAS＋保留区规则已经更强）。

#### 8-27-8 验收

每张内建过滤器一组 golden；一条性质「输出 ≤ 输入」；一条性质「输入非空则输出非空」；一个 citysim 场景——同一颗种子、同一张过滤表，重放出逐字节相同的窗口。

#### 8-27-9 接口与文件切分（card-11.4 实现记）

> §8-27-1…8 定的参数与不变量一个不改；本小节只把它们落成签名，并记下三处那几节没写明、实现时按下面读法取的口径。

```rust
// runtime::sieve — 入口（形状 1）
pub struct CommandKey { arm: String, program: String, args: Vec<String> }   // Ord：BTreeMap 键，跨调用差分按它分组
impl CommandKey {
    pub fn of(arm: &ExecArm) -> CommandKey;   // Program→(path,args)；Shell→按空白切 text，首词为 program；Python→("python",[code])
    pub fn command(&self) -> String;          // program 的文件名去目录、去 .exe、小写：过滤表 `command` 按它匹配
    pub fn subcommand(&self) -> Option<&str>; // 首个不以 `-` 开头的参数：过滤表 `subcommands` 与 `{sub}` 按它取
}
pub struct SieveInput<'a> { pub key: &'a CommandKey, pub exit_code: Option<i64>, pub text: &'a str }
pub enum Sieved {
    Passed { text: String, reason: PassReason },   // 地板以下／全部阶段被拒：原文一字不动
    Cut(SieveRecord),
}
pub enum PassReason { BelowFloor, NothingShrank }
pub struct SieveRecord { pub text: String, pub original: Locator, pub rest_path: PathBuf, pub filter: String,
                         pub lines_in: u64, pub lines_out: u64, pub bytes_in: u64, pub bytes_out: u64,
                         pub stages: Vec<StageReport> }
impl SieveRecord { pub fn payload(&self) -> Result<Payload, AxError>; }   // result_offloaded 载荷：原文 locator＋替代体长度＋逐级账
pub struct StageReport { pub stage: Stage, pub outcome: StageOutcome }
pub enum Stage { StripAnsi, FoldBlank, DedupTemplate, DiffPrevious, Filter, CutLongLine, Truncate }
pub enum StageOutcome { Applied { bytes_before: u64, bytes_after: u64 }, Noop, Rejected { grew_to: u64 }, Unavailable { reason: String } }
pub fn sieve(input: SieveInput<'_>, table: &FilterTable, site: &mut OffloadSite<'_>, history: &mut SieveHistory)
    -> Result<Sieved, AxError>;
// tee 走 offload::tee（pub(crate)；offload() 自身也改经它，store-before-cut 只有一处）；history 无论 Cut／Passed 都记本次原文

// runtime::sieve::filter — 过滤表（形状 6）
pub struct Filter { id, command, subcommands: Vec<String>, strip_ansi: bool,
                    drop_prefix / drop_contains / drop_suffix / keep_prefix / keep_contains / keep_suffix: Vec<String>,
                    group_until_blank: bool,                                    // 命中 keep 的行把其后到空行为止的行一起带上（rustc 诊断分组）
                    head: Option<u64>, tail: Option<u64>, on_empty: Option<String> }   // 全部字段 serde default；只有 id 与 command 必填；未知字段拒
pub struct FilterTable { filters: Vec<Filter> }
impl FilterTable {
    pub fn builtin() -> FilterTable;                            // 恰三张：cargo、git、generic
    pub fn parse(toml_text: &str) -> Result<FilterTable, AxError>;   // `[[filter]]` 数组；E_INVALID_ARGS 拒坏表
    pub fn resolve(city: Option<&str>, building: Option<&str>) -> Result<FilterTable, AxError>;  // 口径①整值覆盖：楼＞城＞内建
    pub fn lookup(&self, key: &CommandKey) -> &Filter;         // 表内命中＞generic；表内顺序即优先序
}

// runtime::sieve::diff — 跨调用差分（形状 1）
pub struct SieveHistory(BTreeMap<CommandKey, Locator>);        // 本 run 内每个键最近一次的原文 locator；调用方持有

// runtime::pipeline — 管线接入
pub struct SieveRequest<'a> { pub key: CommandKey, pub exit_code: Option<i64>, pub table: &'a FilterTable, pub history: &'a mut SieveHistory }
pub struct PackContext<'a> { /* 既有五字段 */ pub sieve: Option<SieveRequest<'a>> }
// package：sieve 为 Some 时 `result` 是命令输出文本而非 JSON；tee 复用 ctx.offload；无 OffloadSite 即无 tee 即不压（不变量 4），记 Unavailable
```

**文件切分**（一文件一模块，全部 ≤ 400 行）：

| 文件 | 形状 | 持有 |
|---|---|---|
| `sieve.rs` | 1 判定 | 阶段定序、逐级「不变长才接受」的裁决（`Draft`：行与账同行）、页脚 |
| `sieve/key.rs` | 2 值 | `CommandKey`：arm／program／args，Ord |
| `sieve/record.rs` | 2 值 | `Sieved`／`SieveRecord`／`Stage`／`StageOutcome`／`StageReport` 与 `result_offloaded` 载荷 |
| `sieve/filter.rs` | 6 数据 | `Filter`／`FilterTable`：TOML 形、三张内建、三层整值覆盖、命中规则 |
| `sieve/scan.rs` | 1 判定 | 重要行五级优先级的四类手写扫描；受保护片段（URL、设备码、`secret:` 引用、`路径:行:列`、退出码行）的判定 |
| `sieve/stages.rs` | 1 判定 | 去 ANSI、空行折叠、模板去重、长行截断、head/tail/中段截断 |
| `sieve/diff.rs` | 1 判定 | `SieveHistory` 与跨调用差分 |
| `sieve/tests.rs` | — | 三张内建过滤器的 golden、两条 proptest 性质、阶段账 |

**三处读法**（§8-27 未写明处，按此实现；改口径先改这里）：

1. **受保护片段的「不得触碰」**读作：含受保护片段的行不进模板去重、不被长行截断——这两级会**改写**一行；在第 7 级截断里它算最低一级重要行（排在 note/help 之后），与其他重要行一起按序取前 60。把它读成「恒不丢」会让一份三百条 URL 的清单压不动，与不变量 1 的目的相悖。跨调用差分**不豁免**它：把上一次逐字相同的行计入「未变 N 行」不改写任何一行，那行在 rest 文件里原样在，模型上一次也已读过；豁免它的第一版让每个 rustc 诊断块被 `-->` 行切成折不动的短段，差分在它为之而存在的 cargo 场景上恒为 Noop。空行同理计入。
   第 3 级模板去重另豁免过滤表 keep 命中的行及其分组（否则 `  |` 这样的诊断沟槽行跨块折叠，`10 |     let x0 = 1; [×6 similar lines]` 说的是并不相同的六行）。
2. **过滤表的 `head`/`tail`** 是第 7 级 HEAD/TAIL 的逐表覆盖，不是第二次截断；`keep_*` 命中的行进第 7 级中段候选，优先级与 §8-27-6 的 warning 同级。这样截断只有一处权威。
3. **generic 的 `on_empty`** 缺省为 `"(no output kept), exit {code}"`；`{code}` 在无退出码（sandbox trap／fuel 耗尽）时写 `none`。这是不变量 2 在通用路径上的执行体。

**三个 reducer 与表的关系**：cargo 与 git 是两个以代码构造的 `Filter` 值，rustc 诊断分组是 cargo 那张的 `keep_contains` 命中行向后扩到空行为止（同一诊断块整体进中段候选）；generic 是空谓词的 `Filter`。表内条目与内建同形，故楼级表可以整张换掉 cargo 的裁法而不动代码。

**citysim 侧**：`Scenario` 增 `sieve: Option<SieveWorld>`（CAS＋environment＋过滤表＋本 run 的 history）；有它时执行器把名为 `exec` 的工具结果经 `package_exec` 走带 `SieveRequest` 的 `package`，模型看到 `{content, exit_code, sieve:[载荷]}`。`citysim/tests/sieve.rs`：同一目录同一表跑两遍账本逐字节相同；第二次同命令只出新错误与 `[unchanged: N lines…]`。

**已知未接**：生产路径 `bin::assembly::driving` 今天不经 `pipeline::package`（工具结果原样交模型）；本卡把 sieve 接进 `package` 与 citysim 执行器，生产接线随 backlog（§8-28）落表时一并走 `package`。

### 8-28 runtime::backlog（card-11.2；形状 4 适配器＋形状 6 数据面）

> 裁决 D26。它同时补上 D25 的洞。

**问题**：`turn.rs` 首段写明中断只在相位边界被消费，而 `Command::output()` 阻塞在系统调用里、不在边界上，所以一条挂死的命令 `halt` 停不住。删掉花费预算与回合上限（card 11.7）之后，这是全仓唯一一处无界失效。

**设计**：一张表，成员是后台 exec 子进程与 `delegate` 子 run。

- **`exec` 恒经此表**，不设 `background` 参数——两条路径就是两个权威，而有洞的那条永远是没人想起的那条。
- 先阻塞等一个短窗口（**10 s**），短命令因而感觉上仍是同步的；超时则返回一个句柄并继续在后台跑，结果落**下一次工具结果的尾部**。这正是 `docs/City.md` 已经写给 agent 的那句话（「Do not wait for a long task. Start it, continue with other work, and read the result when it arrives at the end of a later tool result.」），今天对 exec 不成立。
- `halt {scope}` 遍历该表并终止其成员；工作线程不再进入阻塞系统调用，halt 因而真的停得住。
- `status` 报告表中属于本 run 的成员：几条在跑、各自跑了多久。

**红测试**：一条不会结束的命令起后，`halt` 使其进程终止且 run 回到边界；十秒内结束的命令不产生句柄；后台结果确实出现在下一次工具结果的尾部；一次调用的 verdict 恒只答它等到的那扇窗——窗口外落定的命令不回写本次结果，收割是下一次调用的事。

#### 8-28-1 接口与三处已定的实现选择（card-11.2 落地时补记）

```rust
pub struct BacklogId(u64);                      // Display；一次 serve 内唯一
pub enum Started {                              // 短窗口的穷尽结果，恒不是 bool
    Settled { exit_code: i64, stdout: String, stderr: String },
    Backgrounded { id: BacklogId, what: String },
}
pub struct Standing { pub id: BacklogId, pub scope: Address, pub what: String }
pub struct Finished { pub id: BacklogId, pub what: String, pub exit_code: i64,
                      pub stdout: String, pub stderr: String }

#[derive(Clone, Default)]
pub struct Backlog(/* Arc<Mutex<Table>>，表内是 BTreeMap */);
impl Backlog {
    pub fn run(&self, scope: &Address, what: String, command: Command) -> Result<Started, AxError>;
    pub fn halt(&self, scope: Option<&Address>) -> Result<usize, AxError>;   // None＝整城
    pub fn harvest(&self) -> Result<Vec<Finished>, AxError>;
    pub fn standing(&self, scope: &Address) -> Result<Vec<Standing>, AxError>;
}
```

三处实现选择，各有理由：

1. **短窗口靠固定次数的轮询走完，不采样时钟。** 10 s ＝ 500 次 × 20 ms，两个数都是常量。理由是本仓那条「时间是入参，唯一采样点是 `bin::assembly`」——一个为了等十秒而调 `Instant::now()` 的模块会把那条规则打穿，而计数不需要时钟。
2. **子进程的输出写文件，不走管道。** 管道缓冲区填满会让后台子进程停在写系统调用上，于是「后台」变成「挂死」——那正是本卡要修的那个洞的另一种写法。文件住 `std::env::temp_dir()` 下按 `BacklogId` 命名的一层目录，收割时读完即删。
3. **表是一份共享句柄（`Clone` 的 `Arc<Mutex<_>>`）。** 装配层持一份，每个 `ExecTool` 持一份克隆，于是 `halt` 够得着 `exec` 起的东西而不必让 `halt` 认识 `exec`。表内是 `BTreeMap`，遍历序恒定。

**本卡落地范围（诚实记账）**：成员目前只有后台 `exec` 子进程。`delegate` 子 run 入表是同一张表的第二类成员，接口已按此形状留好（`Standing`／`Finished` 不提进程），但本卡不接线；`status` 报告本 run 那部分同理待接。**→ 第二类成员与 `status` 那一半由 §8-28-2 接上。**

#### 8-28-2 第二类成员：委派下去的 run（card-11.2 补全；形状不变）

**问题**：§8-28 写明成员有两类，而 §8-28-1 只接了第一类。于是今天 `halt {scope}` 遍历表时看不见任何 run——一轮委派下去的活在它的 scope 被停摆之后照样跑到冻结。

**设计**：表内成员分两种身体，一张表、一个 `halt`：

```rust
pub enum BacklogKind { Command, Run }              // Standing 携带；恒不是 bool
pub struct Standing { pub id: BacklogId, pub scope: Address, pub what: String, pub kind: BacklogKind }
impl Backlog {
    /// 一轮委派下去的 run 入表。装配层在 drive 之前调，只对 `Depth::Delegated` 的派活调。
    pub fn enrol_run(&self, scope: &Address, what: String) -> Result<BacklogId, AxError>;
    /// `halt` 是否已经到过这名成员。run 的中断钩子在每个安全点问它，真即 `Interrupt::Cancel`。
    pub fn stopping(&self, id: BacklogId) -> Result<bool, AxError>;
    /// run 冻结后离表。不离表的成员会让 `standing` 报一轮已经结束的活。
    pub fn leave(&self, id: BacklogId) -> Result<(), AxError>;
}
```

- **一个 run 成员没有进程可杀**：`halt` 对它做的是把身体记成 `Stopping`，而 run 在下一个安全点问 `stopping` 并以 `Interrupt::Cancel` 走 `runtime::turn` 既有的取消路——取消因此仍只在相位边界被消费，§8-28 首段那条法则不动。
- **只有委派下去的 run 入表**。根 run 由 `Cancel` 结束，那是另一个动词（glossary「Halt」行）；把根 run 也入表会让 `halt` 与 `Cancel` 变成同一件事。
- **`harvest` 与短窗口都跳过 run 成员**：它们收的是进程的退出码，run 的结局在账本上。
- **`status` 的那一半**：第十四行 `backlog:` 追加在冻结序末尾（追加规则同 `neighbours`），报 `standing(addr)` 里属于本 run 地址的成员——`bg-3 cargo build (command)` 逐条分号相连，没有则 `none`。**不报跑了多久**：本表不采样时钟（§8-28-1 第 1 条），一个为了报时长而采样的 status 会是第二个采样点。
- **接线在 `bin::assembly::dispatching::running::dispatch_in`**：`at.parent.is_some()` 时先 `enrol_run`，drive 结束后 `leave`；`Driving` 携 `member: Option<BacklogId>`，中断钩子在人的打断与 steer 之前先问 `stopping`。今天的派活是同步的，一条 `halt` 命令在子 run 跑完前到不了记账线程；驾驶池（sprawling-SPEC §8-42）落地后 `halt` 在记账线程上被处理而子 run 在池上跑，这条路才在产品里真正走通——本节先把表接对，红测试用 `attach_interrupts` 在子 run 的安全点上调 `halt`。

**红测试**：一轮委派下去的 run 起后，其 scope 被 `halt`，子 run 以 `cancelled` 冻结且没有再叫过模型；`status` 的第十四行报本 run 起的后台命令。

### 8-29 runtime::tools::read 区间读（card-11.3）

> 裁决 D30。

`ReadTool` 增 `offset`（0 基行号，缺省 0）与 `limit`（缺省与上限**均为 512 行**）。被截断时结果携 `total_lines` 与 `next_offset`，所以「我拿到的是不是全部」不需要猜。`bytes` 字段的语义不变，仍是本次返回文本的长度。

理由：今天是 `read_to_string` 整读，而 `kernel-SPEC.md` 1483 行、`ARCHITECTURE.md` 112 KB——一次读要么吃掉整个窗口，要么被管线从中间剪掉，而剪掉的往往正是要改的那一段。512 是选定值，不再讨论。

#### 8-29-1 参数与结果（实现照此，不再选择）

```rust
// args：{path, offset?: u64, limit?: u64}
// offset 缺省 0；limit 缺省 512，大于 512 者**夹到 512** 而非拒绝——
// 模型多要一点不该赔掉一个回合，它拿到的截断字段会把真相说清楚。
// offset／limit 非整数或为负＝E_INVALID_ARGS；limit==0 同。
const LINE_CAP: u64 = 512;
```

切行按 `split_inclusive('\n')`：每行连它自己的换行符一起数、一起还，所以 `offset=0, limit>=total` 的返回与整读**逐字节相同**，`bytes` 字段的旧语义因而不动。

| 情形 | `text` | `total_lines` | `next_offset` |
|---|---|---|---|
| `offset + 返回行数 < total_lines`（截断） | 该区间 | 有 | 有，＝`offset + 返回行数` |
| 读到文件末尾 | 该区间 | 无 | 无 |
| `offset >= total_lines`（越过末尾） | 空串 | 有 | 无——后面没有东西了，给一个 `next_offset` 就是请模型原地打转 |

目录路径与 catalog 条目走同一条切行路：一个条目短到永不触顶，而两条路就是两个权威。

#### 8-29-2 保留区判定移出（同卡）

`resolve` 里「`Address::parse` 后判 `is_reserved`」这一段移进 `runtime::tools::chosen_path`（§8-30-1），`read` 与新的 `search` 同调它。理由是本卡的硬约束：模型选的路径能不能到保留区，全城只允许有一个答案与一组测试。

### 8-30 runtime::tools::search（card-11.3；形状 1 判定＋形状 4 适配器）

> 裁决 D30 的后半。

**问题**：十三件工具里没有一件能找东西。找一个符号只有两条路——写 Python（要可选的 CPython-WASI 构件，很多机器上根本没有），或走 shell（Windows 上是 `findstr`，而 shell 本身是楼级配置可以关掉的）。card 11.5 要给旧对话一个地址，而**没有检索的地址比没有地址更糟**：模型被告知那里有东西，却够不着。

#### 8-30-1 runtime::tools::chosen_path（形状 1；模型选路的唯一判定处）

```rust
// 无 I/O、无时钟、无全局状态；入一个字符串，出一个地址或一个三段式拒绝。
pub(crate) fn admit(asked: &str, action: &'static str) -> Result<Address, AxError>;
// 解析失败＝E_INVALID_ARGS；Address::is_reserved()＝E_GATE_DENIED。
```

它是 `read` 原有那段判定的搬家，不是它的第二份。`search` 遍历时对每一个候选文件同样只问 `Address::is_reserved()`——kernel 的那个原语——所以「什么是保留区」自始至终一个权威。

#### 8-30-2 参数与结果

```rust
// args：{text, path?, context?}
// text：子串，必填且非空。**不是正则**。
// path：城相对前缀，缺省＝整城。走 chosen_path::admit。
// context：每侧上下文行数，缺省 0，上限 4（更大者夹到 4）。
const MATCH_CAP: usize = 64;      // 命中上限；到顶即停走，结果自陈 truncated
const FILE_BYTE_CAP: u64 = 1 << 20; // 单文件上限 1 MiB，越界跳过
```

结果：`{matches: [{path, line, text}], count, truncated, unreadable}`。`line` 是 **0 基**，与 `read` 的 `offset` 同一套编号，所以「搜到再读那一段」是把一个数字原样递过去。`unreadable` 是遍历中打不开的目录与文件数：**找不到与看不了是两个答案**，把后者吐成前者就是把一次失败抹掉。按策略跳过的（二进制、超大、保留区）不计入它。

**不用正则表达式**，沿用 `compaction` 模块头的既有裁决并原样引用它的理由：模式引擎会把回溯放在模型和它的下一个回合之间。子串扫描是线性的，且一个模型写错的正则不会变成一次挂死。

#### 8-30-3 遍历跳过什么，以及为什么

| 跳过 | 理由 | 权威 |
|---|---|---|
| 保留区子树 | 一跑不读治理自己的东西 | `Address::is_reserved`（kernel） |
| `.git` 目录 | 它是对象库不是文本，扫它只产出乱码命中 | 本节 |
| 非 UTF-8 文件 | 二进制里没有可读的行 | 本节 |
| 大于 1 MiB 的文件 | 把一个大对象读进内存找子串是一次停顿 | 本节 |

#### 8-30-4 红测试

超过 512 行的文件返回恰 512 行、并给出真实 `total_lines` 与可续的 `next_offset`；`search` 找到子串并带上下文；`search` 对保留区前缀以 `E_GATE_DENIED` 拒绝；两者共用的 `chosen_path::admit` 有且只有一组测试。

#### 8-30-5 同集改

ARCHITECTURE.md §12 runtime 表 23→27 行（`chosen_path`、`read::tests`、`search`、`search::tests`）；`docs/glossary.md` §5 增 **search** 一行；装配层 `lay_out_workbench` 的准入清单由十三件变十四件，`search` 排在 `read` 之后——次序是缓存面的一部分，只在末尾追加。

### 8-31 runtime::tools::exec 的环境声明（card-11.1）

> 权威在 kernel-SPEC §8-22 的「11.1 增」段与 city-SPEC §8-4 的「11.1 增」段；本节只说 exec 这一侧怎么用它，以及构造面因此怎么变。

**今天的事实**：`ENV_ALLOWLIST: [&str; 4] = ["PATH", "LANG", "LC_ALL", "TZ"]` 加 `env_clear()`。于是城里的 resident 跑不动 `cargo build`——MSVC 链接器读不到 `%ProgramFiles(x86)%`，退回裸 `link.exe`，撞上 PATH 上 Git 那个 coreutils `link`。本版本承诺的闭环断在第四段。

**改法**：`ENV_ALLOWLIST` 原样留着，它是**每一栋楼无条件继承的地板**；楼在 `[sandbox] env_passthrough` 里逐名声明的是**地板之上加的那几个**。两份清单合并后按名取值，取不到的名字不进（一个没设过的变量不该变成空串——空串与未设置在 Windows 上是两件事）。

**构造面**：`ExecTool::new` 原有七个参数，`argument_count` 的历史册子里记着它。本卡不把它加到第八个，而是把总在一起走的那几个值起个名字：

```rust
pub struct ExecSetup {                 // 形状 2 值类型
    pub workdir: PathBuf,
    pub mounts: Vec<Mount>,
    pub python_wasm: Option<PathBuf>,
    pub shell: Option<PathBuf>,
    pub fuel: Fuel,
    pub env_passthrough: Vec<EnvVarName>,
    pub domain: Address,
}
pub fn new(setup: ExecSetup, sandbox: Box<dyn Sandbox>, backlog: Backlog) -> Result<ExecTool, AxError>;
```

三个参数，于是 `crates/runtime/src/tools/exec.rs::new` 那一行历史豁免不再被用到。它留在册子里不动：那份册子是 gate 机械面，删行与改被判代码同集会撞上 `guard`，而豁免只被查存在性、留着不放行任何东西。

**账本上留什么**：配置写入时不记事件——`CONFIG.toml` 是「一跑受什么治理」的权威，再记一条同事实的事件就是第二个权威（这条已由 `RunWorker::configure_building` 的注释裁决过，本卡遵守）。账本记的是**一跑拿它做了什么**：`exec` 的结果载荷增 `env` 字段，列出这次真正递给子进程的名字，按名排序。名字不是值——值恒不入账本。

**验收**：声明了名字的楼里，`exec` 的子进程恰好看到那几个（加地板四个）；没声明的楼里只看到地板四个；凭据形状的名字在配置解析处被拒。以及一次真实演示：声明了名字的楼里 `exec` 跑得动 `cargo build`。

### 8-32 runtime::transcript（card-11.5；形状 2 值类型）

> 裁决 D32。旧对话得到一个地址，于是 §8-30 的 `search` 从「方便」变成「承重」——一个不能检索的地址比没有地址更糟。

**它是什么**：一跑冻结时，把**模型实际看到的消息**——工具调用与其结果、sieve 产出的压缩形、指回被搁置部分的 rest 指针——写成 `<room>/<run-id>.jsonl`，一行一条 `ChatMessage`（`kernel::model::wire` 的 serde 形，原样，所以 `search` 找到的行号就是消息序号）。frozen prefix 不在其中：它是每次请求都相同的那一半，账本的 `prompt_assembled` 已经持有它的哈希。

**它不是账本，这是一条裁决**：账本住 `.sprawling/`，`read` 对那里的每一条路径都拒绝；而且账本是**全城一条链**——把它交给一个 resident 就是把一栋 confidential 楼的事件也交出去。transcript 不加密：同楼的 resident 可以读它，confidential 楼靠它已有的隔离保护自己。

**三步定序，不可颠倒**：①凭据扫描（`redact::redact` 逐消息走一遍，与 `model_returned` 入账本同一把扫描器）；②钉入 CAS（`memory::Cas::put`，内容寻址，同一份 transcript 写两次是一次）；③实体化（写到房间，只读位）。先扫后钉：一个钉进 CAS 的密钥永远删不掉。

```rust
pub struct Transcript { run: RunId, lines: Vec<String>, redacted: u32 }   // 私有字段，一处构造
impl Transcript {
    pub fn of(run: RunId, window: &Window) -> Result<Transcript, AxError>;  // 逐消息序列化、扫描
    pub fn address(room: &Address, run: RunId) -> Result<Address, AxError>; // `<room>/<run-id>.jsonl`，纯函数：路径在跑之前就已知
    pub fn materialise(&self, cas: &mut Cas, city_root: &Path, room: &Address) -> Result<TranscriptRecord, AxError>;
    pub fn lines(&self) -> &[String];  pub fn redacted(&self) -> u32;
}
pub struct TranscriptRecord { pub address: Address, pub original: Locator, pub redacted: u32 }
impl Run<Frozen> { pub fn transcript(&self) -> Result<Transcript, AxError>; pub fn plan(&self) -> &RunPlan; }
```

`Frozen` 状态因此保留 `Window`：冻结后唯一还需要它的读者就是这一处。地址是纯函数，所以 `freeze_plan` 在 drive 之前就能把它写进 Handoff 的 `context` 段——`handoff_written` 载荷携一行 `transcript at <room>/<run-id>.jsonl`，一条测试钉住这一行的存在。实体化发生在 drive 归来之后的 `conclude`（装配层持 CAS），失败记 diagnostics 而不让一次已冻结的跑变成 `Err`：冻结已在账本上，transcript 是它的副本。

**接线**：`RunPlan::predecessor` 为 `Some` 时，run segment 增一行 `Predecessor transcript: <address>`——继任者从 prefix 就知道去哪里 `search`。

### 8-33 runtime::tools::succeed 与 succession（card-11.6；形状 4 适配器）

> 裁决 D33。三件事一起落地，缺一件就是一个洞。

**动词**：`succeed {reason}`。一跑请求由继任者接替自己：**同地址、同深度、同工具表**，无需人在环内。它答的是「继任者将在你冻结后于 `<addr>` 启动；先把 `Handoff.md` 写在你的房间里」，而不是一个结果——与 `delegate` 同理，工具不能从一个 run 的工具台里驱动另一个 run。桌面 `SuccessionDesk` 至多持一份请求（第二次调用覆盖第一次，理由随之更新）；装配层在 `conclude` 里读它，**被取消的跑不接替**（与 delegate 的第四安全点同一条规则）。

**深度守恒**：succession 与 `delegate` 是两个动词。`delegate` 让深度加一；succession 不加。`Assignment::depth()` 从 `parent` 推出，继任者**继承前任的 `parent`**（而不是以前任为 parent），所以深度按构造守恒，工具表因而与前任逐名相同——`delegate` 在内。红测试：继任者的工具表与前任逐名相等。

**账本**：`Assignment`／`RunPlan` 增 `predecessor: Option<RunId>`，写进 `run_started` 的 `predecessor` 键；`Provenance` 增同一指针（memory-SPEC §8-17：第六条 trailer `Sprawling-Predecessor`，仅在有前任时出现；`model_fields` 同时写 `predecessor`）。`bin::views` 从 `run_started` 折出 `predecessors: BTreeMap<RunId, RunId>`，`Query::Commit` 的答 `CommitAnswer` 增 `lineage: Vec<RunId>`——本跑在前，逐级向前到第一任（WIRE_V 14→15，channels-SPEC §8-18）。红测试：三次接替后 lineage 有四个 run。

**Handoff 从楼搬到房间**：`city::handoff(city_root, room)`／`handoff_path(city_root, room)` 读写 `<city>/<room>/Handoff.md`；模板在 `city::open_room` 打开房间时铺下，楼级 `lay_out` 不再铺它。理由是 card 3.5 的同楼并发：一栋楼一份 Handoff，两个房间同时冻结就是两份内容抢一个文件。

**质量防线（不可选）**：`eval::probe` 的 handoff 探针在每次 succession 真的跑。`eval::handoff_probe()` 给出固定四问（版本 1）；装配层 `bin::assembly::probing` 在 `conclude` 读到接替请求时，用前任的 adapter 对前任的 transcript 问一遍（before），在继任者 `freeze_plan` 之后、第一回合之前，用继任者的 prefix 问一遍（after），`eval::compare` 后记一条 `eval_run`（`probe`／`version`／`kept`／`lost`／两份答案）。探针答案不是判定，`lost` 报的是位置，人自己去读两份答案——这正是 eval-SPEC §8-2 定的口径。每次 succession 两次模型调用，这是这道防线的价钱，写在明处。

### 8-34 runtime::reminder（card-11.8；形状 1 判定）

> 裁决 D34。

**两道阈值**，以 provider 报回的 `input_tokens`（事实）对模型的 `context_tokens`（`CallShape::context_tokens`，来自 endpoint 簿）计算；**恒不用窗口字节数**（那是估计）。

| 阈值 | 说的话 |
|---|---|
| 25% | 只报用量：`[context] 25% of the window used (N of M input tokens).` |
| 65% | 报用量，并说明剩余预算仍够写 handoff 并 `succeed`，过了这一点就不够了 |

**每道阈值一跑恰响一次**。状态是穷尽枚举 `Sounded { Nothing, Quarter, TwoThirds }` 而不是两个布尔；一跳越过两道（0→70%）时只响高的那一道，低的一并作废——两句话叠在一起是噪声。

```rust
pub struct ContextGauge { window: Tokens, sounded: Sounded }
impl ContextGauge { pub fn new(window: Tokens) -> ContextGauge; pub fn observe(&mut self, used: Tokens) -> Option<ContextReminder>; }
pub enum ContextReminder { Usage { used: Tokens, window: Tokens }, HandoverWindow { used: Tokens, window: Tokens } }
impl ContextReminder { pub fn render(&self) -> String; }
```

`window == 0`（簿上没写）恒不响：没有分母就没有百分比，与 `UnplannedProgress` 同一条理。整数算术：`used * 100 / window` 用 checked 乘法。

**接线**：`TurnReport` 增 `usage: Option<ModelUsage>`；`Run<Active>` 持 `ContextGauge`，每回合以 `usage.input_tokens` 观察，响则以 `Window::push_reminder` 落在该回合工具结果之后——与 steer 同一扇门，所以它「落在下一次工具结果的尾部」。`pipeline::PackContext` 同时增 `reminder: Option<ContextReminder>` 作第四个附件，句子只在 `ContextReminder::render` 一处定义。

### 8-29 回合不再有上限（card-11.7）

`RunPlan` 去掉 `budget_turns` 与 `budget`，`drive` 的 `while turns < budget` 变成 `loop`。一次跑的结束只有三种来路：一回合作出结论、一次带 carrier 事件的失败（写进历史后冻结为 cancelled）、或一个安全点送到的中断。

- **理由是刹车只留一个**：没有人能在一件事跑之前给它定价，而一个替人说停的数字，停的时刻恰好是人最不希望它停的那一刻。要停一片就 `Halt`——card-11.2 之后它真的会终止那片里的后台成员；要停一条就 `Cancel`。
- **`Completion::Limit` 保留**：它是账本词汇，旧历史里读得回去；本 crate 不再产出它。
- **`StatusTool` 十三字段变十二**：`budget_usd`／`budget_tokens` 删除，那两个数报的是上限而不是花销。留在原地的是 `ctx`——已用 token 对着这次跑拿到的窗口，那是**报告花了多少**而不是**事前不许花**。
- **citysim**：`ScenarioSpec.budget_turns` 随之删除；原先靠上限收尾的那条 scenario 改为「跑满它自己要的每一回合再作出结论」（十六波之后是空的那一波）。

### 8-36 写域的两道闸各问一个问题（前端会话 4，2026-09-11；kernel-SPEC §8-46 末段）

- **`bench::admit`** 对 `Effect::Write { domain: area }` 改调 `kernel::reach(&self.domain, area, &self.taint)`：工具声明的是一块区域，门口只问这块区域够不够得到。
- **`tools::edit::invoke`** 解析出 `target` 后改调 `kernel::domain(&self.writable, &target, &TaintSet::empty())`：`Allow` 继续，`Deny { refusal }` 原样作 `Err`（三段式因此由 kernel 一处产出，工具不再自拼 `Outside` 的话术），`Escalate` 在写域门上不可能出现——`GateOutcome` 刻意穷尽，这一臂如实答一条 `E_INVALID_ARGS` 说明该不变量，而不是 `unreachable!`。空 `TaintSet`：taint 是 bench 的事实，工具这一层没有它，拒词因此少一句「派生自 N 个外部来源」——那句话仍由门口那道 `reach` 说。
- **红→绿**：`tools/edit/tests.rs` 新增「Documents 域的工具创建 `<city>/hall/note.md` 成功、创建 `<city>/hall/note.rs` 被拒（`E_OUTSIDE_WRITE_DOMAIN`，主语是文件）」；`bench/tests.rs` 新增「Documents 域、声明区域为 `hall/mayor` 的 `Write` 效果在门口放行」——改前后者红在 `NotMarkdown`。

### 8-35 去重答的是第一次的结果，而不是一句「你已经问过了」（card-F4.3；形状 1 判定）

**旧行为**：`ToolBench::invoke` 认得重复的 `IdemKey`，回 `BenchOutcome::Duplicate`，而两个调用方（`bin::assembly::driving::lane`、`citysim::executor`）把它翻成 `E_INVALID_ARGS`。于是「同一次调用做两遍」的正确答案——**第一次的结果**——被换成了一个错误：重试的模型学到的是「这件事失败了」，而它其实成功了。这是幂等只做了一半：副作用被挡住，答案没有被记住。

**改法是把记住的东西从键变成键与答**：

| 之前 | 之后 |
|---|---|
| `seen: BTreeSet<IdemKey>` | `seen: BTreeMap<IdemKey, ToolOutcome>` |
| `BenchOutcome::Duplicate` | `BenchOutcome::Duplicate { outcome: ToolOutcome }` |
| 调用方回 `E_INVALID_ARGS` | 调用方回第一次的 `ToolOutcome`，`fenced` 为空 |

- **写入点不动**：键仍在过门之后、工具运行之前记下（被门拒的重试不算重放），答在工具返回后补齐。故一次「记了键但工具报错」的调用不会留下一个假答案——那条路径根本不入表。
- **`Duplicate` 仍是一个独立变体而不是并进 `Ran`**：`fenced` 对重放恒为空，而 `Ran` 的调用方要按 `fenced` 决定波后清扫；把两者合并会让「这一波要不要扫」多出一个恒空的分支。
- **代价写在明处**：一次运行期间每个成功调用的结果都留在内存里。这与 `seen` 本来就要活到运行结束是同一条寿命，多出来的是 payload 的字节；一次运行的工具调用数以百计而非以百万计。
