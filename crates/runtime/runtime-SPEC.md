# runtime-SPEC.md

> crate：`runtime`。本 SPEC 先于代码存在；实现不多不少地遵守本文。

## 1 需求分解

| 模块 | 一句话 |
|---|---|
| `replay` | 离线重演验链：EventRef 第二铸造点；A2 的执行体 |
| `fork` | 分叉前缀：母 Run 事件序列到节点为止的逐字节前缀；A19 的执行体 |
| `turn` | 回合 typestate 四相＋取消边界；形状 5；相内中断编译不过 |
| `prefix` | FrozenSegment 四段＋分段哈希＋易变类型隔离；形状 5＋2 |
| `handoff` | 五段构造点＋resume 消费 Handoff 产新 Run 种子；形状 2 |
| 完备化 | prefix 四段全量（封顶＋截断标注＋跨段去重＋跳过入账）＋断点 ≤4＋Steer 边界消费＋窗口组装入 Assembling 相 |
| `pipeline`＋`offload` | 结果信封三附件＋offload 四不变量（独占有损可还原）＋截断定序 |
| `clock`＋`catalog`＋`mode` | ISO UTC 的唯一拼法＋ClockStamp 与它的发放规则＋ClockReading（§8-10、§8-53）＋渐进披露三类条目＋两个 mode 的目录行与合并时的准入（§8-54） |
| `watchdog` | 处置面分级（纠正 Steer→停滞→冻结）；依据只从 kernel::stall 来 |
| `sandbox` | 缝（trait）＋wasmtime fuel 生产适配器＋直通/故障两替身；A10 三断言 |
| `tools/` | exec 三臂／edit 乐观并发／read 区间读／search／status／succeed，与模型选路的唯一判定 `chosen_path`，以及经它按字节读的 `bound_reader`（§8-14、§8-29–§8-33、§8-59） |
| `bench`＋`conversation` | ToolBench 按 Effect 过门（§8-14、§8-46）；会话历史的唯一持有者（§8-3、§8-47） |
| `run` | Dispatch → N 回合 → 冻结的事件序唯一权威（§8-15、§8-45） |
| `digest`＋`diagnostics` | 冻结时的结构化摘要（§8-16）；给人读的诊断行（§8-17、§8-38） |
| `sieve`＋`compaction`＋`elision` | 工具输出的确定性压缩（§8-27）；按内容分类的缩短判定（§8-7）与回合边界的压缩（§8-44）；「这里被剪过」的唯一标记（§8-42） |
| `backlog` | 后台命令与委派 run 的全城一张表，halt 由此停得住（§8-28） |
| `reminder`＋`redact`＋`transcript` | 上下文提醒（§8-34）；进账本前的打码（§8-41）；逐 run 的对话文件（§8-32） |

「replay 只重演不重执行」的含义：重演＝验证链与重建记录序列；入窗重建器（C16/A15）随 prefix 组装加入，与分叉共用本模块的验证输出。

## 2 验收标准

- A2 演示：对 jsonl 落盘目录与内存行序列各跑一次 verify，逐事件验 prev 链与 seq 连续；任一字节被篡改即拒并报行号。
- A19 演示：从任一节点取分叉前缀，与母 Run 原始行 0..=at_seq 逐字节相同；`at_seq` 越界＝`E_INVALID_ARGS`（恒不静默截到末尾）；母序列不因分叉改变。
- 未知 kind：无 `ig:true` 即拒（方向语义：更新的写方）；带 `ig:true` 的行跳过类型化解读但链照验。
- A4——同一 PrefixPlan 两次 build 逐字节同（golden）；A15——prompt_assembled 载荷＋同源文档经 `rebuild_prefix` 重建，四段哈希逐段相同。
- A7——offload 往返（替代体≤原件且≤上限；循 rest_path 续读与循 CAS 取回字节一致；外部清理后自 CAS 重物化字节一致；命中既有哈希直引原 Locator）。
- A18 零字节——granularity=Off 时打包输出与未接 clock 特性逐字节相同。
- A10 三断言结论书——fuel 内成功／未授能力被拒／fuel 耗尽中断（真 wasmtime 上）。
- L0×失败注入矩阵；A6 双守（Done 恒携 kind 合法证据，运行时纵深校验）；A8（到限恒 `Completion::Limit`，恒不记完成）。

## 3 假设与歧义

1. **verify 的规范复验**：v1 无升级器链，故对每行断言 `canonical_line(parse_line(raw)) == raw`（写方规范性质）。未来 v>1 经升级器读入后此断言只对原版字节成立——届时随升级器一并改约（本文更新）。
2. **fork 的 run_forked 落账**：事件写入母城 Ledger 由调用方（runtime 回合层／citysim）执行；fork 只产 EventDraft 与前缀，不持 Ledger 句柄——保持纯函数形。
3. **同一套重建器**：A15 与 A19 共用 verify 输出；重建器＝verified 行序列本身。
5. **命令结果的戳是答复时刻**：回合在调用工具面的 `account` 之前读答复时刻（§8-15、§12.8），所以工具面打戳时 `ClockReading` 里最新的就是它，戳的秒数等于这条调用 `tool_result` 的 `t`；`turn::tests::concurrent` 的 `a_stamp_the_face_reads_is_the_moment_its_answer_records` 在串行与开头只读段两条路上钉住这一点，`accounting::worker::driving::tests::sieving` 的 `a_served_command_result_ends_with_the_second_its_call_answered` 在一次真实派活上比同一个 `t`。
4. **前缀续期未接线**：`prefix::warmth` 的 `Warmed` 与记账已在（§8-4-2），而 run 结束后按 `next_due` 醒来发续期的那条循环还没有；接上它要先定续期的 usage 记成哪一种事件。
6. **`contract_kept` 的证据今天没人量。** 城读不出一次翻新有没有动到可观察的契约，装配层把 `Produced.contract_moved` 恒填 `false`，所以选了 `contract_kept` 的 run 在合并时恒放行（§8-54）。要让这一要求真的拒，得有一个读得出契约的量具（例如 run 前后同一组对外测试的结果对照）把它填进 `Produced`；判定它的证据是一次动了对外行为、测试仍绿的翻新在 citysim 里被放行。
7. **`Create` 管不到楼的 MCP 工具。** 一个 MCP server 是楼自己声明的外部进程，它写不写文件、写在哪里，城看不见（§8-55 只覆盖城自己的写路径：edit、exec 与链接）。候选是 `Create` 下不挂载声明了写效果的连接器，或只挂载声明只读的；判定它的证据是一个会写文件的连接器在 `Create` 的 run 里改动了已有文件。

## 4 现状分析

verify 为 O(n) 全量；消费面（测试/夹具/citysim）规模千行级，无性能议题；seq→偏移索引归 storage::index。

## 5 权威信源

Fork 三规则；重放/分叉/幂等；at_seq 越界、未知 kind、崩溃恢复行；kernel-SPEC §8-4/§8-9；storage-SPEC §8-1。

## 6 命名统一

**跨 crate 类型住处**：`kernel` 的门／计划／脊／事件／错误／弃置／秘密七面已切目录，`cargo public-api` 基线记其定义位簇路径（如 `error::shape::AxError`）；本 crate 经 `kernel` 顶层重导出引用，公共拼写不变，住处是 kernel 内政（`storage::checkpoint::Checkpoint` 住 `checkpoint` 同例）。

replay、verify、VerifiedLedger、VerifiedLine、fork prefix、`at_seq`。不引入「重播/回放/复演」等同义词。

## 7 模块边界

```
replay ──▶ kernel(event/ledger/error)、storage(jsonl::read_raw_lines)
fork   ──▶ replay(VerifiedLedger)、kernel
turn   ──▶ kernel(ledger/event/error/tool/model)、prefix(FrozenPrefix)
turn/recovery ──▶ turn(ledger 的 Journal)、kernel(model/error)   // 模型调用恢复管线（§8-49）
prefix ──▶ kernel(locator::B3Hash/event::Payload/error)
handoff──▶ kernel(locator/event/error)
pipeline ──▶ offload、sieve、clock、kernel(tool)
sieve  ──▶ offload(tee)、storage(cas 读前一次原文)、kernel(tool::ExecArm/locator)
offload ──▶ storage(cas)、kernel(locator)
watchdog ──▶ kernel(stall/completion)
catalog ──▶ kernel(tool)、mode
sandbox ──▶ wasmtime（feature `wasm` 内藏；缝声明恒在）
tools/ ──▶ kernel(tool/version/discard/gate)、sandbox、storage(cas 经 pipeline)
```

上图只画各模块的主要依赖；crate 之间的依赖以 ARCHITECTURE 的 `depmap` 块为准，文件清单以它的模块图为准。

**replay／fork 不做什么（否定式三条；本 crate 的其余模块，如 `tools::exec` 与 `backlog`，执行真效果）**：
- 不重执行任何效果——verify 恒不调工具、不出网、不写盘。
- 不生成 RunId——新 Run 身份由调用方注入（kernel 禁随机的同一纪律）。
- 不读 projection——重放的唯一输入是 Ledger 原始行（历史只有一份）。

## 8 接口先行（按模块分章）

### 8-1 runtime::replay

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
/// 逐行判定不住在这里：每一行经 `storage::LineCheck::advance`，拒词经 `LineFault::into_ax`
/// ——与 `JsonlLedger::open` 的尾段扫描同一份检查（storage-SPEC §8-1）。
pub fn verify_lines(lines: Vec<Vec<u8>>) -> Result<VerifiedLedger, AxError>;
/// Convenience over a jsonl directory: storage::jsonl::read_raw_lines + verify.
/// 给要原始行的读者：分叉（`fork::prefix` 吃 `VerifiedLedger`）、citysim 检查器与测试。只要结论的 `sprawling replay`
/// 走 `storage::audit_chain`，要折叠的走 `fold_ledger_dir`：两者都一次只持一段字节。
/// 无段目录与空账本在此同形（均得空 VerifiedLedger）——本函数的调用方均自持城根算出路径；
/// 区分二者是「从人那里拿到路径」的一层的事（§11；sprawling-SPEC §12）。
pub fn verify_ledger_dir(dir: &Path) -> Result<VerifiedLedger, AxError>;
/// 流式折叠：经 `storage::LedgerIndex::folding` 一次一段地读，每行过同一个 `LineCheck`，已知记录借给 `each`
/// 后即丢；ignorable 行只入链不入折。每行只读一次、只解析一次，顺序即账本序，结果确定。
/// 常驻的是一段字节与一条记录，而不是 `VerifiedLedger` 的全部原始行与全部记录——
/// 启动折叠（`fold_city`、`Standing::fold`、`Views::rebuild`）与 `resume` 的启动扫描只要折的结果，不要行本身，走这一面；
/// 分叉要原始行，走 `verify_ledger_dir`。
/// 失败即停：第 k 行验不过时，前 k-1 条已经给过 `each`，此时返回的 Err 说明整份折叠作废，
/// 调用方丢弃它折出的一切（与 `verify_ledger_dir` 同一拒词、同一行号）。无段目录同上，折叠为空。
/// 读史走 `storage::LedgerIndex::folding`，返回的索引与折叠出自同一遍字节：索引覆盖的恰是 `each` 见过的那段历史，
/// 折叠之后别人追加的行不在其中，留给 `refresh`。已知行把自己的 seq 与 run 交给索引，ignorable 行由索引自己定位。
pub fn fold_ledger_dir(dir: &Path, each: impl FnMut(&EventRecord) -> Result<(), AxError>) -> Result<LedgerIndex, AxError>;
```

流程：逐行①envelope 探查（serde_json::Value：v/seq/prev/kind/ig 键）；②v 判向（>EVENT_LOG_V 即 `E_LOG_VERSION_UNSUPPORTED`）；③链续（`chain_hash` 复算对拍 prev，首行对 GENESIS_PREV）；④seq 连续（自 FIRST 起）；⑤kind 已知→`parse_line` 全解＋规范复验＋`to_ref`；未知＋`ig:true`→记 IgnoredUnknown；未知无 ig→`E_LOG_VERSION_UNSUPPORTED`（subject=kind＋行号）。链与 seq 对一切行（含 ignored）成立。

**「没找到要验的东西」与「验过且为空」必须异形，但不在这一层异形**（issue #3）。`fold_ledger_dir` 的四个生产调用方（`fold_city`，经 `snapshot::start` 起步的 `Standing::fold` 与 `Views::rebuild`，`startup_scan`）均自持城根算出路径，而 `JsonlLedger::open` 只建目录、首次 append 才建段：**已开未写的城恰好是一个无段目录**，在此处报错会把一个合法启动当成错误（`Standing::fold` 早已以 `if ledger_dir.exists()` 记下这个状态）。若改成在此报错，四个调用方就各需一份同样的守卫——一条条件四份拷贝。

故依据归给**拿到人输入路径的那一层**：`sprawling replay <ledger-dir>` 先问 `storage::ledger_segments_at`，一段都没有就报 `E_PATH_NOT_FOUND`（sprawling-SPEC §12）。先例取自本仓库：`xtask guard` 在无提交时说 `no commits yet, nothing to judge`，而不说通过。**空账本本身仍然合法**：`verify_lines(vec![])` 照旧返回空 `VerifiedLedger`。

### 8-2 runtime::fork

```rust
/// Byte-identical fork prefix (A19): raw lines 0..=at_seq of the verified
/// mother sequence. `at_seq` past the tail is E_INVALID_ARGS, never a
/// silent clamp to the end.
pub fn prefix(mother: &VerifiedLedger, at_seq: Seq) -> Result<Vec<Vec<u8>>, AxError>;
/// The run_forked draft for the city Ledger. Caller supplies the new run
/// id, the room it lands in, and the clock reading; fork itself is pure.
pub fn fork_draft(origin: Origin, new_run: RunId, addr: Address, t: TimeMs, who: String)
    -> Result<EventDraft, AxError>;      // data = {"from": …, "at_seq": …}
/// What a new session inherits: the mother's conversation, rebuilt from
/// her own records, cut at the last line a conversation can be cut at,
/// read through the ledger's resident index: the named line for its run,
/// then that run's own lines, and nothing else read.
pub fn inherited_indexed(index: &storage::LedgerIndex, dir: &Path, at_seq: Seq)
    -> Result<Inherited, AxError>;
pub struct Inherited { pub messages: Vec<ChatMessage>, pub at: Seq }
```

**`inherited_indexed` 是「分叉」这个词真正的执行体，而它是重建而不是复制。** 一个 run 的 conversation 由持有它的循环逐回合折起来，进程一停就没有了；折它的那些记录都在账本上。这个函数走母 run 自己的线——开场任务读 `run_started`、助手消息读 `model_returned`、工具结果读 `tool_result`、人中途说的话读 `steer_received`——并把它们**折过流动循环折过的那同一个 `Conversation` 类型**，所以一条分支拿到的次序就是母亲发出去的次序，而不是对同一批记录的第二次读法。**账本已经过一次脱敏**，分支继承的是母亲真正发出去的那份文本。

**切点只有一处权威，而且它往回退。** 一次工具波是好几行，只有它的末尾是能切的地方：`tool_called` 已写、`tool_result` 未写的中间，是一条「assistant 消息的工具调用没有答案」的半个回合，任何 provider 都拒。所以折叠（`fold_run`）维护一个「开着的一波」，命中中途就退回上一次安全点，并在 `Inherited::at` 里如实回报它**实际用到**的那一行——调用方把这一行写进 `run_forked`，于是页面显示的分叉点与模型真正拿到的那一段是同一个事实。

**上下文提醒不重建，也重建不了**：它是这座城在跟模型说这一跑自己的预算，没有属于它自己的记录，而一条分支带着自己的量表开始。这条差异写在函数自己的文档里，因为它是一处诚实的不完整，不是漏掉的一步。**回合边界的压缩会重建**：母亲窗口里每回合的 exchange 是收尾边界压缩后的字节，`fold_run` 在同一边界对同一材料重放同一判定（8-44），分支拿到的是母亲真正发出去的那一份，而不是账本里更全的那一份。

**写账本的那一方走索引（`fork::indexed`）。** 持有账本的 worker 为一次分支重建只需要母 run 自己的那几行：`inherited_indexed` 用 `LedgerIndex::line_at` 读 `at_seq` 那一行定出母 run，再按 `run_seqs_before` 只读这条 run 的行，每一行先过 `storage::read_line`——它与验链门逐行所用的 `LineCheck::advance` 共用同一条分类规则：已知 kind 解析成记录，带 `ig` 的新 kind 跳过，其余（撕裂、不规范、无 `ig` 的未知 kind）以 `LineFault` 的码拒绝；切点与消息都由 `fold_run` 一处判定。它不验链：worker 是这本账唯一的写者，打开时已经过尾部恢复；而 `verify_ledger_dir` 为这一问把整本历史读进内存、逐行验链再解析（94 MB 的账本上是 +67 MiB 的瞬时内存）。拒绝写成人能照做的话：`at_seq` 不在索引里是 `outside`，恢复语给出母序列止于哪个 seq，母 run 在这之前没有 `run_started` 是同一条 `E_INVALID_ARGS`。
**母亲自己是分支时，先重建她开场时继承的那段。** 流动循环里母亲的窗口以 `RunPlan::inherited` 开头，那段对话不在她自己的线上；她的 `run_forked`（`run` 是她、`from`／`at_seq` 指向祖母的切点）才是它的出处。所以 `inherited` 先找属主 run 的 `run_forked`，按其 `at_seq` 递归重建祖母的对话，经 `push_inherited` 放在最前，再折母亲自己的线——与 `Run::begin` 同一顺序。递归只往账本更早处走（`at_seq` 必须早于那条 `run_forked`，否则拒），所以一定终止。


**`addr` 落在 `run_forked` 的那一行上**：新 run 的 id 说的是「谁继续谁」，而地址说的是**哪个房间的这次继承已经用掉了**——`accounting::worker::folds::session` 只用这两个字段回答「这个房间的当前一段是否还欠一段对话」。

### 8-3 runtime::turn（形状 5 typestate 机）＋bench／conversation

**一个 typestate 机、一张工作台、一份会话历史，三种形状三个模块。**
- `bench`（形状 1 判定）：`ToolBench`／`BenchOutcome` 与三条必要前提次序（去重先于副作用；exec 的 discard 预报先于 Write 门；Deny 以 `tool_result` 回去而不结束回合）。它拥有的是**次序**；工具本身以 `Box<dyn Tool>` 递入，沙盒在缝上，副作用不归它。
- `conversation`（形状 2 值）：`Conversation`，以及再导出的 `Opening`（定义住 kernel，因为 `run_started` 要带它，kernel-SPEC §8-82-1）。它是会话，不是 transcript（冻结后写下的逐 run 文件），也不是 `kernel::Window`（上下文大小）。一条不变量在每一个入口上成立——**连续的 user 内容并进已开的那条消息，而不另开一条**；steer、工具结果与开场任务是同一条规则的三扇门。
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
- **`ConcurrentInvoke` 是三段，不是一个闭包。** 放行（`admit`，`&mut`，按调用序）、执行（`tool` 借出 `&dyn Tool`，`&self`，各条在 scope 线程上调它的 `invoke`）、记账（`account`，`&mut`，按调用序）。一个包着 bench 的闭包表达不了这个次序：放行与记账写同一张去重表与同一份 taint，而 bench 不是 `Sync`（checkpoint 持有 git 仓库句柄），工具是（`kernel::Tool: Send + Sync`，`invoke(&self)`；有内部状态的工具把状态放在自己的锁后）。三个闭包也不行：三者要借同一个 bench，一个要 `&`、两个要 `&mut`。所以它是 trait——第二实现在缝上已经存在：闭包的全覆盖实现（放行即作答，不报效果，于是 citysim 与脚本化工具的测试走串行、字节不动），与装配层 `accounting::worker::driving::placing` 的 bench 实现（sprawling-SPEC §8-31）。只读调用的门不读 taint，所以先放行后记账不改变任何一扇门的判定；`IdemKey` 的位置在放行时按调用序定下，exec 计数与 checkpoint 记录在记账时按调用序累加，所以它们与串行波逐字相同。落选的是「lane 把整个 bench 放进锁、闭包取 `Sync`」：锁把三条读排成一条队，并行只剩名字。生产路径由 `tests/run_driver.rs` 的三读测试守着：`drive` 走 `lifecycle` 到 `execute_concurrent`，三条读彼此重叠，次序与载荷与串行逐行相同。

- **生成中起跑只读调用（`runtime::turn::speculation`，形状 2 值：按位置的缓存 `Speculated`）。** `Generating::Speculating` 让 `call` 走 `Model::call_speculating`；模型每交出一条调用，只要它排在本回答第一条非只读调用之前、`effect_of` 答 `Effect::Read`、`ahead` 借得出工具，它就在一个 `std::thread::scope` 线程上起跑，scope 在模型调用返回前 join 全部线程，于是一回合的墙钟约等于 max(工具, 生成)，而不是两者之和。结果按调用在回答中的位置缓存进 `Turn<ToolWave>`，连同起跑时的那条调用；入账时仍按调用序先 `admit`，放行（`Cleared`）且该位置缓存的调用与结算后那条逐字段相等，才用缓存结果，否则照常执行——所以 `tool_called`／`tool_result` 的载荷与顺序与串行波相同，时刻按 §8-15 的时点采，推测结果本身不是事件。回答失败（流被切断、恢复段重发）时整份缓存随那次尝试丢弃；取消落在 k 处时 k 之后的缓存随 `Turn` 丢弃；`admit` 自己作答（重放、门拒绝）时该位置的缓存丢弃。哪些调用可以提前、缓存按什么序入账，权威是 `crates/runtime/spec/Turn/Speculation.lean`：越过第一条写调用推测会让读看到写之前的世界（`speculating_past_a_write_changes_the_ledger`）。**起跑不问 `still_going`**：一个已立的取消挡不住早读，它们的结果随 `Turn` 丢弃；代价是被停的 run 仍做完这些读，模型调用失败时也要等它们 join 才返回，而它们都无副作用，所以不越过任何门。**起跑先于放行**：放行写去重表与 taint，被截断的回答得把它们撤回，而只读调用的结果在放行前算出、放行后才用，被拒的那条结果从不到达模型与账本。落选的是「推测时就 `admit`」：它要为截断与取消各写一条撤销路径。`ahead` 有默认 `None`，闭包的全覆盖实现因此不提前起跑，citysim 字节不动；bench 的实现按名借出（`ToolBench::tool_named`，与 `tool_for` 同一张表）。

### 8-4 runtime::prefix（形状 5＋2）

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
    /// 从 `bytes()` 重算四段哈希并与构造时记录的对拍；不等即拒绝。
    pub fn verified_segment_hashes(&self) -> Result<[B3Hash; 4], AxError>;
    pub fn prompt_payload(&self, plan: &BreakpointPlan) -> Result<Payload, AxError>;   // prompt_assembled 载荷
}
/// 同一断言作用在请求携带的四块上（`turn::call`）：四块、逐块 cache=true、逐块哈希对拍。
pub fn verified_system_hashes(system: &[SystemBlock], frozen: &[B3Hash; 4]) -> Result<[B3Hash; 4], AxError>;
// 载荷的键由 `kernel::event::record::PromptAssembled` 一处拼写，写读两端各经 `Payload::of` 与
// `Payload::read` 一扇门；本模块只提供值：`SegmentSlot::as_str` 给出 slot 名与 breakpoints 行，
// `SegmentSource::row` 给出 `PromptSource`，`build_segment` 的落选行给出 `PromptSkip`（reason 是
// 闭集 `SkipReason { Duplicate, Unreadable, NotUtf8, NoBudget }`）。
```

- 段序即缓存经济：类型把四段位置写死，断点与各段上限见 §8-6。
- **`segment_hashes()` 返回构造时缓存值，`verified_segment_hashes()` 才是权威。** 一个事实（这段字节的哈希）只有一个权威：`FrozenSegment::assembled` 在构造时算一次，每次派活前从将发的字节重算一次并与它比对。两者不一致意味着模型将读到的字节不是运行冻结的那一份，而那正是「同一事件序列逐字节重放」所依赖的东西，故拒绝而非警告。
- 分段哈希经 `B3Hash::digest`（kernel 唯一哈希产地）；A4（同输入同字节）由 golden 断言，A15 重建器随 S3。
- trybuild 反例：`FrozenSegment::from(TimeMs)`／把 TimeMs 传进 assemble —— 无转换路径，编译不过（ClockStamp 等类型落地后同规逐个加反例）。

### 8-4-1 一句恢复语只许指向线上真有的动词（`prefix::segment::ANOTHER_ADDRESS`）

```rust
pub(crate) const ANOTHER_ADDRESS: &str =
    "send this task to another address, which opens a session of its own";
```

- **一个事实一个家**：「一个被冻住的会话怎么出去」只拼一遍，`prefix::segment::prefix_drifted` 与 `turn::report::shape_moved` 两条拒绝各自接上它们自己的解释；两份拼写会各自漂。
- **拒绝不得指名一个不存在的动词**：恢复语只指向每个客户端都有的那件事——把任务发到另一个地址。**一句指向不存在动词的恢复语，比没有恢复语更坏**：人按它去找，找不到，然后以为是自己没找到。
- **这句话随动词走**：线上有 `Command::OpenSession`；恢复语要改指它时，改的是这一个常量，而不是去两处各改一遍。

### 8-4-2 runtime::prefix::warmth：每个前缀最近一次请求与它的续期（形状 1 判定）

```rust
/// 一个 session 的保温账：按四段哈希记每个前缀最近一次真实请求与它的 `kernel::keep_warm::CacheUse`。
pub struct Warmth { /* setting、lead_ms、kept: BTreeMap<[B3Hash; 4], Kept> —— 私有 */ }
impl Warmth {
    /// lead_ms 是调用方对所连 provider 实测的往返时长（kernel-SPEC §8-74）。
    pub fn new(setting: KeepWarm, lead_ms: u64) -> Warmth;
    /// 一条真实请求在 at_ms 发出。Off 时什么也不记。
    pub fn sent(&mut self, request: &ModelRequest, at_ms: u64);
    /// 最早一条续期的发出时刻；定时器睡到这一刻。None＝没有要续的，定时器不必醒。
    pub fn next_due(&self) -> Option<u64>;
    /// 把 now_ms 已到期的续期经 model 发出，返回每条续期的回执（调用方照常记 usage）。
    /// 过期不再续的前缀在这里被丢掉。模型失败原样返回，未发的前缀留待下一次。
    pub fn renew_due(&mut self, model: &mut dyn Model, now_ms: u64) -> Result<Vec<ModelReturn>, AxError>;
}
```

- **Off 时不记、不发**：`sent` 在 `KeepWarm::Off` 下不克隆请求，所以默认配置下每个 session 不为保温多占一字节，也不会有续期可发。默认配置下城不为保温多发一条请求，本模块的测试钉住这一点。
- 续期请求是该前缀最近一次真实请求原样重发，只把 `max_tokens` 压到 1：缓存按前缀字节命中，与输出上限无关，所以 1 个输出 token 是续期能付的最低价。
- 一个前缀只留最近一条请求：同一前缀后来的请求覆盖前一条，内存随前缀数而非请求数增长。
- 何时续、续几次全由 `kernel::keep_warm::renewal_due` 判定，本模块不另写第二个 TTL。
- 真实请求进账的门是 `Warmed`：它拥有一次 run 调用的 adapter，本身也是 `kernel::Model`，所以 turn loop 照旧只见 `&mut dyn Model`，每一次成功的调用（阻塞门或流式门）都在返回后按发出时刻记进 `Warmth`。失败的调用不记：provider 没有读到的前缀没有可续的缓存。

```rust
/// 拥有 adapter 的保温门；clock 取自 bin::assembly 的唯一采样点。
pub struct Warmed<C> { /* model: Box<dyn Model + Send>、warmth: Warmth、clock: C —— 私有 */ }
impl<C: FnMut() -> Result<TimeMs, AxError>> Warmed<C> {
    pub fn new(model: Box<dyn Model + Send>, setting: KeepWarm, clock: C) -> Warmed<C>;
    pub fn next_due(&self) -> Option<u64>;
    /// 经自己拥有的 adapter 发出 now_ms 已到期的续期。
    pub fn renew_due(&mut self, now_ms: u64) -> Result<Vec<ModelReturn>, AxError>;
}
impl<C: FnMut() -> Result<TimeMs, AxError>> Model for Warmed<C> { /* call、call_streaming */ }
```

- `lead_ms` 不是常量：`Warmed` 把每次经它发出的调用（真实请求与续期）的往返时长记为下一次续期的提前量，所以提前量跟着所连的 provider 与链路走，不按某一类机器调校。流式调用的往返含生成时长，只会让续期提前，不会让它晚于缓存过期。
- 未接线（§3）：run 结束后把 `Warmed` 留在 worker 上、在唯一 spawn 点的 attend 循环里按 `next_due` 醒来发续期、把续期的 usage 记成事件。

### 8-5 runtime::handoff（形状 2）

```rust
#[derive(Serialize)]   // 五个字段名即 handoff_written 的五个键；不派生 Deserialize
pub struct Handoff { /* must_read、overview、progress、context、next_step —— 私有 */ }
impl Handoff {
    /// Sole constructor: must-read non-empty; every
    /// entry is an already-parsed Locator by type. Five sections always
    /// present; prose quality is the handoff probe's business, not the type's.
    pub fn new(must_read: Vec<Locator>, overview: String, progress: String,
               context: String, next_step: String) -> Result<Handoff, AxError>;   // 空 must_read → E_INVALID_ARGS
    pub fn must_read(&self) -> &[Locator];  pub fn payload(&self) -> Result<Payload, AxError>;  // handoff_written 载荷
}
pub struct ResumeSeed { pub run: RunId, pub must_read: Vec<Locator> }
/// Resume consumes a Handoff and mints a new identity — never revives the
/// frozen one (元原则六). The caller supplies the new RunId (kernel 禁随机).
pub fn resume(handoff: &Handoff, new_run: RunId) -> ResumeSeed;
```

- **载荷即这五个字段**：`payload` 是 `Payload::of(self)`，键名由字段名给出，不另手写一遍。只派生 `Serialize`：`new` 是持有一个 `Handoff` 的唯一路径，能从一行账本反造一个的读方会绕过它对空 must-read 的拒绝。
- 「下一步」段首列用户指定动作、must-read 规范类机器填：内容约束属生产者（回合层与 spine 文件），类型只强制结构。
- Run<Frozen> 无解冻：resume 不收 Run 值，只收 Handoff——「旧 Run 醒来」在签名上无法拼写。

### 8-6 turn／prefix／handoff 的会话面（形状不变，参数长入）

typestate 四相、边界消费、事件序、私有字段三不变量不动；会话、工具与调用形状作为相变函数的入参进来。被否替代：平行第二条 call 路径——同一相两个入口即两个权威，落选。

```rust
// kernel::model（缝上 canonical 会话类型，kernel-SPEC §8-24）：
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
                    // Run 内恒不变——改它就换缓存前缀（理由与出处在 kernel-SPEC §8-22）
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
- 断点只有一个作者：`prefix::BreakpointPlan`（`prefix/breakpoint.rs`，形状 1 判定，纯函数）。`BreakpointPlan::for_conversation(&[ChatMessage])` 决定一次请求实际发出的断点：前三段（city／building／resident）的段界各一个，对话非空时尾消息再一个，合计 ≤ `CACHE_BREAKPOINTS_MAX`（4）；run 段界不放，因为尾锚紧随其后已覆盖它。`FrozenPrefix::system_blocks()` 以 `BreakpointPlan::marks_edge(slot)` 标 system 块，`BreakpointPlan::message_breakpoint` 标请求（回合借用会话，不复制它，kernel-SPEC §12.6），`prompt_payload(&plan)` 把 `plan.breakpoints()` 逐个拼成 `breakpoints` 行（段界写 slot 名，尾写 `tail`）；`verified_system_hashes` 以同一个 `marks_edge` 核对线上的块。兼容格式只负责拼写（Anthropic：被标记消息的最后一块带 `cache_control`），不决定任何断点。
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

### 8-7 runtime::pipeline（形状 1＋组装处）

```rust
pub struct PackContext<'a> {
    pub cap_bytes: u64,                       // 窗口余量推导的本次上限（调用方算；恒 ≥ 提示句预算）
    pub stamp: Option<ClockStamp>,            // clock::StampGate 的产出；None＝不携
    pub net_notice: bool,                     // gate::egress 首次公网放行信号
    pub steer: Option<(String, String)>,      // (source, text)；上一边界消费到的 Steer
    pub reminder: Option<ContextReminder>,    // 上下文提醒（§8-34）
    pub offload: Option<OffloadSite<'a>>,     // None＝无 CAS 可用（纯截断退路）
    pub sieve: Option<SieveRequest<'a>>,      // exec 结果才带；无站点即无 tee 即不压
    pub adviser: Option<Consultation>,        // 窗口顾问先跑，判断作参数从此入
}
pub struct Packaged { pub content: String, pub events: Vec<Payload> }   // events＝result_offloaded 载荷（入账归调用方）
pub fn package(result: &[u8], ctx: PackContext<'_>) -> Result<Packaged, AxError>;

// pipeline::adviser —— 顾问端口（形状 3 port＋1 判定）；三型在 pipeline 上重导出
pub use kernel::event::record::AdviserAsk;   // 问法与答案的词汇归 kernel，本模块只翻译与校验
pub struct Ask { pub kind: AdviserAsk, pub subject: String, pub options: Vec<String>,
                 pub material: Option<String> }
impl Ask { pub fn noul(subject: impl Into<String>, material: impl Into<String>) -> Ask;
           pub fn score(subject: impl Into<String>, material: impl Into<String>) -> Ask;
           pub fn choice(subject: impl Into<String>, options: Vec<String>) -> Ask; }
pub enum Consultation {
    Answered { ask: AdviserAsk, subject: String, answer: AdviserAnswer, elapsed_ms: u64 },
    FellBack { ask: AdviserAsk, subject: String, reason: AdviserFailure },
}
impl Consultation { pub fn answer(&self) -> Option<&AdviserAnswer>;
                    pub fn payloads(&self) -> Result<Vec<Payload>, AxError>; }   // adviser_asked＋答或回落
pub struct Adviser { /* answer —— 私有 */ }
impl Adviser { pub fn none() -> Adviser;
               pub fn with(answer: impl FnMut(&Ask, &Conversation) -> Result<AdviserAnswer, AdviserFailure>
                                 + Send + 'static) -> Adviser;
               pub fn consult(&mut self, ask: Ask, conversation: &Conversation, elapsed_ms: u64) -> Consultation; }
```

- 定序：**判定由 `compaction::plan` 一处给出**，答 `Shrink { Keep, Cut(Strategy), MustOffload }`。`Keep` →原样；`MustOffload`（结构化、未知内容、以及非 UTF-8 字节）与「`Cut` 且 `len ≥ OFFLOAD_MIN_BYTES`」→ 有 `OffloadSite` 就 offload；`Cut` 而无站点或不够大 → `compaction::shorten` 按已定的 `Strategy` 裁。**`MustOffload` 而无站点是一次带恢复语的 `Err`，不是私自的字节切**：截半的结构化数据看上去仍可解析，那正是它比缺席更糟的理由；pipeline 若自己切字节，就当场推翻了 compaction 的定规——一条规则两个家。标记由 `elision` 产出且只出现一次。
- **`Shrink::Cut(Strategy::Sections)` 不交给 offload。** 长文档的节标题骨架是该类存在的理由，而 offload 的替代体是文件头＋指向全文的指针，恰好把骨架丢掉；故 `Markup` 一律走 `shorten`，其余 `Cut` 仍按 `len ≥ OFFLOAD_MIN_BYTES` 入 store。
- **非 UTF-8 的静默回落已登记**：`Err(_) => Content::Unknown` 丢掉 `Utf8Error` 的原因，把「二进制」折成「未知」；`Unknown` 的整块离窗使它的行为安全，但名字不对。修法需要一个新内容类与它的 plan／shorten 臂，不止一行，故只登记不改。
- **顾问端口住 `runtime::pipeline::adviser`（形状 3 port＋1 判定），先于 `package` 跑。** 顾问装不进同步的 sieve 链（`Draft::step` 是 `impl FnOnce(&[String]) -> Vec<String>`，无 Result 无 async，而 sieve→package→package_exec 整链同步）；做法是顾问在链外先办，判断作 `PackContext.adviser` 喂进来。三条问法（`Noul` 是非＋概率／`Score` 有序打分／`Choice` 选一，后者仅用于开 session 选模型）与答案／回落载荷形状归 `kernel::event::record`，本模块只翻译与校验。
- **顾问是窗口的顾问，不是前缀的顾问。** 端口签名只拿得到 `&Conversation`（与一个已拼好的 `Ask`）：拿不到 `FrozenConfig`，也拿不到 `FrozenPrefix`。逐轮换模型／effort 是前缀失效，不是窗口调整，故在本端口里写不出来；要挡它们得在 `CallShape` 上挡（§8-3）。
- **顾问的影响只有两条臂，下界是「今天的每个数字」。** `Score { score_bp }` 按 basis points 缩放 cap 给能裁的内容类用；若缩放会把「本会整块保留」的 Structured／Unknown 变成 `MustOffload`，则沿城市自己的 plan 与预算（把一条密度分变成一次拒绝，正是 `Keep` 在防的那件事）。`Noul { keep: false, .. }` 只在「有 `OffloadSite` 且结果大于 cap」时把它移出窗口：offload 只存必须裁的东西，更小的结果没有放得下的去处。`Choice` 不进 `package`（只在开 session 选模型时用）。
- **失败策略：无回答即无调整，且写进账本。** 未装顾问（`Adviser::none`）、端点不可用／超时、答非所问（问 `Noul` 答 `Score`、选择不在选项内、概率越界）一律回落，`Consultation::payloads()` 产出 `adviser_asked` 加 `adviser_answered`／`adviser_fell_back` 两条载荷随 `Packaged::events` 出去。**`answer()` 返回 `None` 时上面每一个数字在原地不动，所以最坏情况恰好等于今天的行为。**
- **顾问端点走既有 `AttachEndpoint` 登记路径，不造第二套 provider 表。** 配置不新增 TOML 段（`ConfigLayer` 只有 effort／sandbox／mcp 且四处 `deny_unknown_fields`）；`gateway::adviser::AdviserClient` 收路由已产出的 `Chosen`，用 `adapter_for` 同一支笔、同一份凭证兑付与 deadline。`Choice` 问法在开 session 选模型时用，前缀成形之前。
- 信封三附件一处组装：正文后依序追加 clock 行／net_notice 行（恒一次：正在连接互联网提醒，英文定句）／steer 行（`user:`／`@ID:` 前缀）；三行字节不计入 cap（附件与负载分账，附件有自己的封顶常数在实现内断言）。
- 按内容分类的缩短判定住 `compaction`（八类内容、四种策略），sieve 住 §8-27；本模块只按它们的答案走「原样／offload／截断」三臂。

### 8-8 runtime::offload（形状 1；四不变量的独占定义处）

```rust
pub const REST_DIR: &str = ".rest";
pub struct OffloadSite<'a> { pub cas: &'a mut storage::Cas, pub city_root: &'a std::path::Path,
                             pub room: &'a kernel::Address, pub origin: storage::BlockOrigin }   // origin：这块字节替哪个 run、哪栋楼写下（storage-SPEC）
pub struct OffloadRecord { pub substitute: Vec<u8>, pub original: Locator, pub rest_path: String,
                           pub original_len: u64 }
pub fn offload(bytes: &[u8], cap_bytes: u64, site: &mut OffloadSite<'_>) -> Result<OffloadRecord, AxError>;
pub fn rematerialize(locator: &Locator, site: &mut OffloadSite<'_>) -> Result<String, AxError>;
```

- `rest_path` 是 rest 文件的城市地址 `<房间地址>/.rest/rest-<hash 尾 16 位>.dat`：以城市根为基、正斜杠、不含 `.` 段，每台机器上逐字节相同。模型的读工具先用 `Address::parse` 收下模型给的路径、再从城市根解析，所以这个地址它能直接读；站点因此携带房间的 `Address` 而不是一条房间 `Path`，物理位置由 `city_root` 与房间地址拼出，地址只有这一种拼法。`./` 起头的房间相对地址会被 `Address::parse` 以 `E_INVALID_ARGS` 拒掉；OS 绝对路径既会把一台机器的家目录写进账本，又让同一颗种子在两台机器上重放出不同的窗口。`REST_DIR` 是这个目录名的唯一定义处，目录由物化时建出。

- 四不变量逐条入断言：①先存后缩（入参恒为全量字节，cas.put 先于一切裁剪）；②替代体含提示句恒 ≤ 原件且 ≤ cap（提示句字节先扣）；③只有有损才存（调用者保证 len>cap 才进来；函数内再断言，违反＝E_INVALID_ARGS）；④替代体恒携 rest_path：物化只读文件于房间的 `REST_DIR`，内容＝全量原件；命中既有 CAS 对象即直引（幂等）。
- 替代体形：头部字节＋`\n[offloaded: total N bytes; rest at <rest_path>; original <locator>]`；提示句 ASCII。
- rematerialize：rest_path 被外部清理后自 CAS 重建，字节一致（A7 第三断言）。

### 8-9 runtime::watchdog（形状 1＋处置历史持有者）

```rust
pub struct Watchdog { /* corrections: u32、provider_failures: u32、streak: u32、retries: Retries、jitter_seed: u64 —— 私有，逐 Run 一实例 */ }
// retries 的类型是 kernel::Retries（§8-43）
pub enum Disposal { Proceed, CorrectiveSteer { text: String },
                                     BackOff { until: TimeMs, code: AxCode, subject: String },
                                     Freeze { reason: FreezeReason } }
// fired_payload 写的是 kernel::event::record::WatchdogFired（kernel-SPEC §8-4），本 crate 不另声明其形状
pub enum FreezeReason { Stall, ProviderRefused }
impl Watchdog {
    pub fn new(retries: Retries, run: RunId) -> Watchdog;               // run 的 16 字节折成抖动种子
    /// Consumes kernel::stall's verdict verbatim; never re-derives it.
    pub fn on_stall(&mut self, verdict: &StallVerdict) -> Disposal;      // 首次 Stall→CorrectiveSteer；再次→Freeze{Stall}
    pub fn on_provider_failure(&mut self, failure: &AxError, now: TimeMs) -> Disposal;   // BackOff.until = now + max(退避, retry_after_ms)
    pub fn on_provider_answered(&mut self);                              // provider 答了：连续失败的计数归零
    pub fn fired_payload(&self, disposal: &Disposal) -> Result<Payload, AxError>;   // watchdog_fired 载荷（E_LOOP_SUSPECTED 的 carrier）
}
```

- 处置必分级：纠正 Steer 文本指名重复指纹；只有终局的处置被明拒。子 Run 监控：`Completion::Limit` 的呈现住 status.children，本模块不重复存储子态。
- **provider 失败按 `AxError::retry` 的三态分类。** `No`→`Freeze { ProviderRefused }`，一次即止；`Yes` 与 `Unknown`→`BackOff { until }`（一次模型调用的效果只是一份城里从未收到的回答，效果是否落在对端只关乎计费，不关乎城里的状态，故「不知道」照样再问），`until = now + 退避`。**退避表只住 `Watchdog`**：自 provider 上次作答以来连续第 n 次失败的基准是 `500 ms × 2^(n-1)`，基准封顶 60 s（第 8 次起恒为 60 s）；实际等待是基准加一段抖动，抖动落在 `[0, 基准/2]`，所以第 n 次等待落在 `[基准, 1.5 × 基准]`，最长 90 s。起点 500 ms 与封顶 60 s 都是对端的尺度（一次过载的恢复时间），与所在机器的快慢无关，故不从机器的测量推导。落选的是「由调用层给 `until`」：调用层手里只有当前时刻，而 `gateway` 里并没有持 retry-after 的准入状态，结果是 `until` 恒等于「现在」，重试不隔一刻。**连续的计数在 provider 作答时归零**（`on_provider_answered`，`drive` 在每个走完的回合后调用）：一个已经恢复的 provider 不欠下一次故障一分钟的首等，而对重试的上限是对「同一个调用再问一次」的上限，不是对整个 Run 一生碰上几次故障的上限。`provider_failures` 仍是这个 Run 的总数，只作观察。**对端说了等多久时，等的是两者中较长的那个**：`until = now + max(退避表, AxError::retry_after_ms)`。对端的 `retry-after` 是它对自己何时恢复的陈述，早于它再问只会再收一次 429；退避表仍是下限，因为一个说「1 秒后」的对端连续失败时，连续计数照样该拉长间隔。对端给的等待不设上限：它可以很长，而停下一个在等的 Run 的是 `Halt`，`watchdog_fired` 里的 `until_ms` 让人看见它在等什么。**抖动以 `RunId` 为种子，同一个 Run 永远得到同一串等待。** 同一时刻被同一次故障打断的多个 Run 若按同一张表等待，会在同一毫秒一齐再问，把刚恢复的对端再压垮一次；抖动把它们错开。种子是 `RunId` 的 16 字节经 FNV-1a 折叠，与连续计数一起过一遍 splitmix64 的收尾混合，再对 `基准/2 + 1` 取余；不取随机源，因为一个 Run 的历史要能逐字节重放（citysim 与离线重放读的是同一张表），而 uuid v7 的末 74 位本身就是随机的，已经把同一毫秒启动的 Run 分开。抖动只往上加：退避表仍是下限，`retry-after` 取两者较长的规则不变。落选的是往下抖（`[基准/2, 基准]`）：那样第一次等待可以短于 500 ms，而表说的是「至少等这么久」。
- **可重试的失败只对着人设的那个上限冻住**（`Retries`）。`UntilHalted` 下停它的是 `Halt`，城里唯一的刹车；`AtMost(n)` 下停它的是人在端点表单上填的那个数。**这两格是穷尽而不是一个带哨兵值的计数**：「一直试到有人喊停」与「试四次」是两种意图，一个数字拼不出前者。填进表单却没有任何东西去读的数字，比根本不给这个字段更糟，所以 `request_max_retries` 的读者就是 `Watchdog`，而它的调用方是下一条的 `drive`。
- **`runtime::run::drive` 是那个调用方**：一次可重试的失败写一条 `watchdog_fired` 再重来，于是历史里第二条 `model_called` 就是人读到的那次重试，而不是一次无声的重复。节奏归 `Watchdog` 的退避表：`drive` 先把 `Watchdog` 给的 `until` 写进 `watchdog_fired`，再交给 `RunHooks::wait` 等到那一刻，于是历史许诺的「不早于 `until_ms`」与下一条 `model_called` 一致。**等待是 `Halt` 够得着一个没有回合在飞的 run 的地方**：`wait` 答 `Halted` 时 run 以 `Cancelled` 冻住，不再发下一次调用。落选的是「等完再问 `interrupt`」：退避长到一分钟，一个晚一分钟才生效的刹车不是刹车。装配层的 `wait` 以 50 ms 为片睡到 `until`，每片问一次是否停下；等待中到达的 steer 留到下一个安全点，不在等待里被吞掉。计数时钟（citysim、离线重放）答 `Allowed` 且不等，因为它重放的东西不在真实时间里等待。
- **为什么按 `AxError::retry` 分类而不设固定次数。** 一个计数器对两种截然不同的失败给同一份预算：`E_WIRE_MISMATCH`（对端不说这个形状）重试三次就是把同一个 400 买三遍，而 429 重试三次就放弃又恰好把一个只需要等待的维护窗口当成了死亡。能否再试是产错处已经知道的事实（`Retry`，fail-closed），拿它分类比在这里重新猜一遍强。
- **冻结原因叫 `ProviderRefused`**（载荷 `reason` 为 `provider_refused`）：没有重试预算，就没有东西被耗尽；冻住的原因是对端给了一个重试不能修复的答复。
- fired_payload 的形状由 `kernel::event::record::WatchdogFired`（kernel-SPEC §8-4）独家拼出：`drive` 若对同一个 kind、同一个 `back_off` 词另写一份 {action, code, subject}，一份历史里就有两种 `watchdog_fired`。退避的原因随 `Disposal::BackOff` 一同旅行——说自己退避却不说退避什么的一行，没人能据以行动。字段＝{action: steer|back_off|freeze, text|(until_ms,code,subject)|reason, corrections, provider_failures}；Proceed 拒绝成帐（无事不记）；纠正只发一次（corrections 计数），第二次 Stall 即冻——分级穷尽于 steer→freeze 两级，「停滞中间态」不另设（它就是 Stall verdict 本身）。`provider_failures` 留下作为**观察**（这个 Run 碰上了几次），不是一个阀值。

### 8-10 runtime::clock（形状 1；纯格式化不采样）

```rust
// 关切时区的权威住 kernel::config::ClockZone（[clock] zones 属三层配置，city 仍拒写它，city-SPEC §8-31）：
// FrozenConfig 携 clock_zones: Vec<ClockZone>；本模块只消费不定义（一个权威）。
pub fn iso(at: TimeMs) -> String;   // ISO 8601，UTC，到秒："2026-05-14T09:31:07Z"；一刻给人或模型读时的唯一拼法
pub fn parse_iso(raw: &str) -> Result<TimeMs, AxError>;   // iso 的逆：只收 iso 写出的那一种形状；其余 → E_INVALID_ARGS
pub struct ZoneEntry { pub id: String, pub offset_min: i32, pub local: String }   // local＝同一刻带偏移："2026-05-14T18:31:07+09:00"
pub struct ClockStamp { pub utc_ms: TimeMs, pub zones: Vec<ZoneEntry> }           // utc_ms＝读数本身，不按桶截；zones 只含配置的时区
impl ClockStamp { pub fn render(&self) -> String }   // 信封的时钟行："clock: 2026-05-14T09:31:07Z;"，其后每个时区 " <id> <local>;"
pub fn stamp(now: TimeMs, zones: &[ClockZone]) -> Result<ClockStamp, AxError>;   // zones > CLOCK_ZONES_MAX → E_INVALID_ARGS；空表即只报 UTC

pub struct StampGate { /* granularity、zones、last_bucket: Option<u64> —— 私有；last_bucket 兼任首发标记 */ }
impl StampGate {
    pub fn new(granularity: ClockStampGranularity, zones: Vec<ClockZone>) -> StampGate;   // 两值都取自这一跑的 FrozenConfig
    /// Emission rule: Off -> never; first result of the
    /// run -> once; Timestamped -> every result; Timeless -> only when the
    /// granularity bucket changed since the last emission.
    pub fn observe(&mut self, now: TimeMs, temporal: Temporal) -> Result<Option<ClockStamp>, AxError>;
}
```

- 历法纯整数（civil-from-days，无 chrono 依赖）：全程在 `i128` 上算，`u64` 毫秒加 `i32` 分钟偏移落不出它的界，所以格式化不会失败，`iso` 与 `render` 不带 `Result`；界证明携 `#[expect]`。**精度与频率分开**：戳一律到秒，粒度只决定 `Timeless` 工具多久带一次戳——同桶里第二条 `Timeless` 结果不带戳，`Timestamped` 每条都带。A18 零字节：Off 时 observe 恒 None。
- `iso` 是本 crate 里一刻的唯一文字形：时钟行、`status` 的 `now:` 行都经它；`parse_iso` 把同一种文字读回一刻（`view --since/--until`，sprawling-SPEC 8-137），同一模块、同一精度。两者互逆：`parse_iso(&iso(t))` 是 `t` 去掉毫秒，`iso(parse_iso(s)?)` 是 `s`。`parse_iso` 只收 `YYYY-MM-DDTHH:MM:SSZ` 这 20 个字节：时区偏移、秒的小数、只有日期、小写的 `t`/`z`、历法里没有的那一天（2 月 30 日、非闰年的 2 月 29 日）、`24:00:00` 与闰秒 `:60`、1970 年之前，一律 `E_INVALID_ARGS`（action `read a UTC moment`，subject 是原文，recovery 给出正确写法）。日子在不在历法里，由把算出的日数经 `civil_from_days` 再算回来、比对年月日判定，历法只有那一份算法。
- 时区行仍在：`FrozenConfig.clock_zones` 在真城里恒空（城配置拒 `[clock] zones`），剧本仍可冻结出非空表，所以格式化保留，偏移写成 `+HH:MM`／`-HH:MM`。

### 8-53 runtime::clock::ClockReading：一跑的驱动最近读到的那一刻（形状 2 值类型）

```rust
#[derive(Debug, Clone, Default)]
pub struct ClockReading(/* Arc<Mutex<Option<TimeMs>>> —— 私有 */);
impl ClockReading {
    pub fn keep(&self, at: TimeMs);          // 驱动的时钟钩子每读一次就记一次
    pub fn latest(&self) -> Option<TimeMs>;  // 最近记下的读数；还没读过为 None
}
// tools/status.rs
impl StatusTool { pub fn clocked(self, clock: ClockReading) -> StatusTool; }   // `now:` 行从它现读
```

- **为什么要它**：`RunHooks::now` 是一跑里唯一的采样点，工具面不采样只收读数（§8-15 时间纪律），而 `ConcurrentInvoke` 与 `Tool::invoke` 的签名不带读数。装配层把 `now` 包一层，读到的每个值先 `keep` 再交给驱动；要报时的工具面（命令结果的戳、`status` 的 `now:`）读 `latest`。这样工具面报出的时刻恒是账本某一行的 `t`，不是第二个钟给出的另一个值。
- **读到的是哪一刻**：回合在调用工具面的 `account` 之前读这条调用的答复时刻（§8-15），所以工具面打包、打戳时读到的就是它，串行的调用与开头只读段的调用都一样；戳的秒数因此恒等于这条调用 `tool_result` 的 `t`。被准入直接答复的调用不经工具面的 `account`，不打戳。
- **`status` 的 `now:` 行**：`now: 2026-05-14T09:31:07Z`，没有读数时是 `now: not stamped`。它不看粒度：`now` 是 `status` 自己报的一栏，不是信封附件，关掉戳不该让模型问不出时间。`StatusSnapshot` 不再有 `now` 字段——快照在派发时冻结，冻结下来的时刻整跑都不动，而这一栏要的正是调用那一刻。
- 与 `ContextReading` 同形同理：一跑一个、克隆共享、写者一个。用 `Mutex<Option<TimeMs>>` 而不是原子整数：「还没读过」是一个状态，不该拿某个整数冒充；锁里只放一个 `Copy` 值、整值替换，所以中毒不留半写的值，读写都取锁里的值照常走。

### 8-57 runtime::clock::UtcSpan：按 UTC 选一段时间（形状 2 值类型）

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UtcSpan { /* since: Option<TimeMs>, until: Option<TimeMs> —— 私有 */ }
impl UtcSpan {
    /// until 不晚于 since → E_INVALID_ARGS（action `select a span of time`）；缺一端即那一端不设界。
    pub fn new(since: Option<TimeMs>, until: Option<TimeMs>) -> Result<UtcSpan, AxError>;
    pub fn since(&self) -> Option<TimeMs>;
    pub fn until(&self) -> Option<TimeMs>;
    /// 半开区间 [since, until)：since 那一刻在内，until 那一刻不在。
    pub fn contains(&self, at: TimeMs) -> bool;
}
```

- **读者**：`sprawling view --since/--until`（sprawling-SPEC 8-137）；playback 的时间筛选接进来时读同一个值，不另写一份区间规则。`Default` 是两端都不设界，`contains` 恒真。
- **逐个时刻判断，不假定有序**：`contains` 只看给它的那一刻。账本的 `t` 不随 `seq` 单调（§12.5：并行只读段的开始时刻可以早于前一条的答复；墙钟也会回拨），所以按时间选行的读者每一行都问一次，不在第一条越过 `until` 的行处停下，也不二分。
- **矛盾的区间在构造时拒绝**：`until <= since` 的区间里没有任何一刻，按它选行只会安静地答出一段空历史；拒绝时 subject 写出两端的 `iso`。合法而恰好什么都没选中的区间照常答空。

### 8-58 分叉照母 run 开篇的写法重建第一条消息（`runtime::fork`、`runtime::run::charter`，形状 1 判定）

```rust
pub use kernel::event::record::Opening;           // runtime::conversation 与根上各一处再导出（kernel-SPEC §8-82-1）
pub struct Charter<'a> { /* …既有字段… */ pub opening: Option<Opening> }
// RunPlan::charter 填 Some(self.opening)；harness 的 charter 填 None
```

- **开篇的写法进账本。** `Charter::open` 把 `opening` 照录进 `run_started.opening`，与 `policy`、`naming` 同一处写。
- **`fold_run` 读它，按下表重建母 run 的第一条消息**：

| `run_started.opening` | 重建成 | 与母 run 发出的字节 |
|---|---|---|
| `WithPerson` | `WithPerson`：人的原话 | 相同 |
| `Inherited` | `Inherited`：`Task: …\nGoal: …` | 相同 |
| `FromJob` | `Inherited` | 第一条起不同 |
| 缺席（加键之前的行） | `job` 在场为 `Inherited`，否则 `WithPerson` | 加键之前的读法，不变 |

- **只有 `FromJob` 改写，理由写在 `fold_run` 旁。** 那一句说「任务在上面的 JOB.md 里」，指的是母 run 前缀 run 段里的那份文本；分支的前缀带的是它自己的 brief，不是母 run 的 JOB.md，照抄那一句就是让分支去读一份它从没拿到的文件。改写的代价是 provider 的前缀缓存从第一条消息起不命中，这一种开篇的分支每次都付；换成把母 run 的 JOB.md 抄进分支的 run 段，run 段就与母 run 的不同，缓存在 system 那一段已经不命中，付的一样多，还在分支里多了一份没人派给它的任务（§12.14）。
- **为什么不用 `goal` 是否为空来猜。** 「有目标才写 job 文件」是 `city::write_brief` 的规则；分叉里按 `goal` 猜写法，就是同一条规则的第二个权威，哪天 brief 的规则改了，分叉会悄悄猜错，而只有缓存命中率会说出来。
- 验收：`fork::request_tests` 的 `a_branch_first_request_opens_with_the_bytes_of_the_mothers_last`（母 run 与分支都经 `drive` 真跑；分支经 `inherited_indexed` 从账本重建；分支第一个请求的消息序列以母 run 最后一个请求的消息序列开头，逐条序列化字节相同）。真实组装出来的前缀经 gateway 按兼容格式渲染后的整份请求，在 accounting 一侧比（sprawling-SPEC 8-141）。

### 8-11 runtime::catalog（形状 6＋渲染）

```rust
pub struct CatalogEntry { pub name: String, pub disclosure: String, pub expansion: String,
                          pub hash: Option<B3Hash>,    // 架上那份文档被读到时的哈希
                          pub package: Option<String> } // 包目录，由 city 的扫描给出；单文档为 None
pub struct SkillPin { pub name: String, pub hash: B3Hash }
pub struct Catalog { /* tools: BTreeMap<ToolName,…>、skills: BTreeMap、mode: Option<Mode> —— 私有 */ }
impl Catalog {
    pub fn new() -> Catalog;
    pub fn admit_tool(&mut self, meta: &ToolMeta) -> Result<(), AxError>;      // disclosure 非空；重名＝E_INVALID_ARGS
    pub fn admit_skill(&mut self, entry: CatalogEntry) -> Result<(), AxError>; // 只收阅览室准入者（装配层按楼的 city::policy 规则求值后直供）；expansion 是城内地址
    pub fn admit_carried_skill(&mut self, entry: CatalogEntry) -> Result<(), AxError>; // 城外书架上的一件：expansion 是扫描读到的那份文档正文（§8-29-6）
    pub fn set_mode(&mut self, mode: Mode);                                    // 只列本 Run 所处者
    pub fn render(&self) -> String;              // Resident 段的 catalog 部分：段头一行自述＋一行一件；BTreeMap 序恒定
    pub fn tool_defs(&self) -> Vec<ToolDef>;     // ChatRequest.tools 的唯一来源
    pub fn expand(&self, name: &str) -> Option<Expansion>;   // 第二级披露（怎么用），§8-6
    pub fn skill_pins(&self) -> Vec<SkillPin>;   // 本 Run 拿到了哪几份，当时各是什么字节
}
```

- **`hash` 是 `Option`，而那个 `None` 不是「没算」**：目录里另有两类条目的正文由本构建自己握着（mode 的纪律、dev 那一条），它们背后没有一份能在无人看着时改掉的文档。
- **pin 从 catalog 取，不重扫一遍书架**：catalog 已经是「本 Run 能够到什么」的权威，再扫一次就是在另一个时刻对同一个问题给第二个答案。
- **一件 skill 怎么交给 run，由它进 catalog 的那扇门定**：`admit_skill` 收城内书架上的一件，`expand` 答 `Expansion::Skill`，`read` 到那个地址去开；`admit_carried_skill` 收城外书架上的一件，`expand` 答 `Expansion::Said`，正文就是 catalog 手里那份。两扇门而不是一个布尔参数：`expansion` 这一格在两扇门后是两种东西（地址与正文），门名把这件事说在调用处。两扇门共用同一套卫生检查（名字与一行披露非空、不重名），重名跨两扇门同样拒。

**`render()` 与 `set_mode()` 的生产调用者是装配层的 prefix 组装**。工具走 `ChatRequest.tools` 到达模型；没有 `render()`，**阅览室准入的 SKILL 与本 Run 所处的 mode 就到不了任何模型**，`city::library` 的准入判定就是一道没有下游的门。

接法：`Catalog::render()` 追在 `identity.segment_bytes()` 之后，合成 Resident 段。**不另开第五个槽**：一个居民能够伸手取到什么，与它是谁同属一类常住事实，且两者都随 Run 冻结，故前缀在整个 Run 的寿命里仍可缓存。装配层因此把 prefix 的组装移到目录建好之后。

**第二级披露经 `read`**：SKILL 的 `expansion` 是 `city::holding_address()` 给的一个地址，坐在**保留前缀 `.sprawling/` 下**。`render()` 不印那个地址；模型按名字调 `read`，`read` 先查 catalog（§8-29），所以它不必知道、也读不到那个保留前缀下的路径。

### 8-12 runtime::mode（形状 6；dev 入口）

```rust
pub const DEV_ENTRY: &str = "dev";
pub fn dev_entry() -> CatalogEntry;   // 一行披露，全部细则归 expansion
```

- **一个 Run 只被告知它所在的那个 mode**，于是没有任何 Agent 知道这座城自己的代码与 SPEC 是可改的。`dev` 行补上这一句，**而且只补一句**：三种准入证据要求与两种落地策略的意思、阅读次序（SPEC → 代码 → 旁边的测试）与「下一步去跟人要它们」全在 expansion 里，由 `read` 按需取。**大多数会话不改这座城，就只付一行的价。**

### 8-12b runtime::mode 原有面

```rust
pub fn catalog_entry(mode: kernel::Mode) -> CatalogEntry;     // chat 与 work 两行
```

`Chat` 的目录行只有一句：专心同人交谈，就对方说的话作答。除这一行提示之外它什么也不做，它存在的理由是让一句闲话不被当成一件要做的活。`Work` 的目录行说：朝人给的目标干活，任务要一份计划时先用 `plan` 工具写进 `Roadmap.md`，目标达成时报告。

哪些 mode 存在、各自拼成什么词，只由 `kernel::Mode` 回答（线、账本都读它）；本模块只持每个 mode 在目录里怎么介绍、以及一次 run 的产出准不准合并（§8-54）。runtime 不再有自己的 `Mode`：两份同成员的枚举要靠装配层一个恒等的 `match` 维系，新增一个 mode 时那是第二处必须同步改的地方。

### 8-54 合并时的准入，按运行策略判（`runtime::mode::admits`，形状 1 判定）

```rust
pub struct Produced { pub tests_passed: Option<bool>, pub contract_moved: bool,
                      pub held_in: Option<bool>, pub held_out: Option<bool> }
pub enum Admission { Lands, Refused { because: &'static str, alternative: &'static str } }
pub fn admits(policy: &kernel::RunPolicy, produced: &Produced) -> Admission;
```

- **判定序**：先看落地策略——`Experiment` 恒 `Refused`（试验的产出不合并，学到的写进 `Memo.md`，换一次常规落地的派活再做）；`Ordinary` 再看准入证据要求：`Standing` 恒 `Lands`（楼自己的规矩已经在别处判过，本函数不加检查）；`Tested` 要 `tests_passed == Some(true)`，`Some(false)` 与 `None` 各有自己的拒词；`ContractKept` 在 `contract_moved` 时拒；`DoubleValidated` 要 held-in 与 held-out 两半都是 `Some(true)`，缺一半与任一半为 `Some(false)` 各有拒词。
- **mode 不参与准入**：交谈与干活产出的东西走同一道合并，要不要证据由证据要求一个值回答（kernel-SPEC §12.12）。
- **唯一的调用方是合并那一刻**：`accounting::worker::reviewing` 在 `PrEffect::Merged` 写 `pr_merged` 之前问它，`Refused` 写 `pr_rejected`，理由是 `because; alternative` 两句（sprawling-SPEC 8-133）。评审说「另一位居民看过」，准入说「这次派活要的证据在」，两个问题两道门。
- **`ContractKept` 今天以城看不见的方式成立**：城读不出一个契约动没动，`Produced.contract_moved` 由装配层恒填 `false`，所以这一要求只在 run 自己报出契约动了的那一天才会拒。这一点照旧写在 §3 而不是假装已经量过。
- 验收：`mode` 测试 `a_work_run_without_the_evidence_it_chose_does_not_land`（`work`＋`tested`、没跑测试 → `Refused`），以及每种要求、每种落地各自的拒与放。

### 8-55 「只新建」在每一条写路径上判（`runtime::tools::edit`、`runtime::tools::exec`，形状 4 适配器）

```rust
impl EditTool {
    pub fn new(city_root: &Path, domain: Address, writable: kernel::WriteDomain,
               limit: kernel::WriteLimit) -> Result<EditTool, AxError>;
}
pub struct ExecSetup { /* …既有字段… */ pub limit: kernel::WriteLimit }
```

- **edit 的两条臂各走 storage 的一种落盘。** 改写一个已有文件：先过写域（§8-36），再问 `kernel::gate::replacing(limit, &target)`，`Create` 下在读文件之前就拒，盘面不动；放行后经 `storage::WriteTarget::replace`（暂存文件再 `rename`，取被替换文件的权限，storage-SPEC §8-32）。新建（`base_version: "new"`）：经 `storage::WriteTarget::create`，名字由文件系统的「仅当不存在才建」原子地占下，已有文件（包括别的调用刚建成的）答 `E_VERSION_CONFLICT`，说出它此刻的版本。新建在两种限制下都这样走，所以竞争的两次新建只成一次，不论限制是什么。
- **edit 的 disclosure 说出限制**：`Create` 下工具描述多一句「this run creates new files and changes none」，模型在动手前就读到，而不是在第一次被拒时才知道。
- **exec 在 `Create` 下只在副本里跑。** `where: host` 的 program 与 shell 直接落在人的树上，没有任何东西能让已有文件只读，所以在起进程之前就由 `gate::replacing` 拒，主语是工作目录；`sandbox` 放置照常：写入落在同步来的副本上，副本里的东西不回到树上，所以已有文件不变、命令也新建不了任何东西。python 臂的挂载恒只读，不受影响。确认不了隔离的工具不开放写入（kernel-SPEC §8-78），exec 的描述在 `Create` 下多一句说明 host 不可用。
- **链接**：写目标与它到城根之间的每一级若是链接（符号链接、junction、硬链接），`WriteTarget::within` 字面拒（storage-SPEC 8-25），两种限制下一样，所以经链接改旧文件这条路在 `Create` 下同样不通。
- 验收：集成测试 `crates/runtime/tests/create_limit.rs` 的 `an_existing_file_is_unchanged_under_create_by_edit_exec_and_link`：`Create` 下对一个已有文件的 edit 改写、host 上一条改它的 shell 命令、在指向它的链接名上新建，三者都拒，文件字节不变；storage 的 `two_racing_creates_admit_one`。

### 8-56 一次 run 记下它冻下的身份版本（`runtime::run`、`runtime::run::charter`，形状 2 值类型）

```rust
pub struct RunPlan { /* …既有字段… */ pub naming: Option<B3Hash> }
```

- **照录，不读。** 身份由 accounting 在冻结前缀时按 session 取定（accounting-SPEC §8-15、city-SPEC §8-33），名字已经在 city 段与 resident 段的字节里；本 crate 只把那一版的摘要从 `RunPlan.naming` 抄进 `run_started.naming`（kernel-SPEC §8-79），由 `Charter::open` 与运行策略同一处写。`None` 是这座城的这次 run 没有冻任何身份（测试替身、早于身份入账的构造方）。
- 被否：让 runtime 读两份治理文档自己算摘要——run 开始的那一刻读到的未必是前缀冻下的那一版，账上的摘要会与请求里的名字不符。

### 8-13 runtime::sandbox（缝清单文件，形状 3＋4）

```rust
pub struct Fuel(pub u64);
pub struct Mount { pub host: std::path::PathBuf, pub guest: String, pub writable: bool }   // preopen＝mount scope
pub struct SandboxJob { pub wasm: std::path::PathBuf, pub argv: Vec<String>, pub env: Vec<(String, String)>,
                        pub stdin: Vec<u8>, pub mounts: Vec<Mount>, pub fuel: Fuel }
pub struct SandboxOutcome { pub stdout: Vec<u8>, pub stderr: Vec<u8>, pub exit: SandboxExit }
pub enum SandboxExit { Success, Failure { code: u64 }, FuelExhausted, Trap { message: String } }
pub trait Sandbox { fn run(&mut self, job: &SandboxJob) -> Result<SandboxOutcome, AxError>; }

pub struct WasmtimeSandbox;            // feature = "wasm"；wasip1 直跑（先按 preview1 落地）
pub struct EchoSandbox { /* 直通替身：stdout＝stdin 回声＋可注入脚本输出 */ }
pub struct FaultSandbox { /* 故障替身：逐次弹出预置 SandboxExit／fuel 耗尽／trap */ }
#[cfg(feature = "conformance")]
pub fn assert_sandbox_conformance<S: Sandbox>(sandbox: &mut S, job: &SandboxJob);  // 良序两连调不中毒＋outcome 形合法
```

- 能力面＝wasip1 preopen 集（Mount 逐条）；无网络能力（WASI p1 天然无 socket 宿主实现——Python 臂禁网的机械保证）；fuel 上限即 Fuel（耗尽＝FuelExhausted，不是 Err：宿主无故障）。
- 未授能力被拒的观察形：guest 内 open 失败→非零退出（Failure）；宿主恒不代 guest 隐藏失败。A10 三断言在真 wasmtime 上以手写 WAT 模块定形（不依赖 CPython 工件）；CPython-WASI 集成测试以环境变量指向工件（住机器本地的忽略目录，恒不入库），缺工件即 skip——`just check` 自足。
**A10 三断言结论书**（证据＝`crates/runtime/tests/sandbox_a10.rs`，真 wasmtime（版本由 `Cargo.lock` 钉），手写 WAT 不依赖任何外部工件）：

| 断言 | 观测形 | 结论 |
|---|---|---|
| fuel 内成功 | `fd_write` 写 `ok\n`，Fuel(1_000_000) | `Success`，stdout 逐字节相符 |
| 未授能力被拒 | 无 preopen 时 `path_open` 失败→guest 自行 `proc_exit(7)` | `Failure{code:7}`；**授予同一目录后同一 guest 转 `Success`**——此对拍使「被拒」是能力判定而非测试损坏 |
| fuel 耗尽中断 | 无限循环，Fuel(10_000) | `FuelExhausted`，且是 `Ok` 非 `Err`（宿主无故障） |

- **无出网的机械形需精确化**：wasip1 **确有** `sock_send`／`sock_recv`／`sock_accept`／`sock_shutdown` 宿主实现（对非 socket fd 恒返 `ENOTSOCK`）；它没有的是**获得** socket 的途径——无 `sock_open`／`sock_connect`／`sock_bind`。故导入 `sock_connect` 的 guest 直接链接失败（`E_SANDBOX_DENIED`），而 guest 能拿到的每一个 fd 都来自 preopen（均为目录）。这才是 Python 臂禁网的准确依据。
- **失败不得以默认值擦除**（rust-hardening Gate 5）：`try_into_inner()` 取不回管道、`get_fuel()` 报不出余量、退出码超 WASI 范围——三者均属**宿主故障**，恒返 `Err`；若以 `unwrap_or_default()`／`unwrap_or(1)` 兑成「空输出」「未耗尽」，就是把猜测冒充事实。
- **feature 内藏是必要的**：开 `wasm` feature 后 runtime 的依赖面增加一百多个 crate，debug 构建的 wasmtime-wasi 单件以百 MiB 计。`just clippy` 带 `--all-features`，所以内藏的代码同样过零警告门。

- wasmtime 与 wasmtime-wasi 钉在 48 线，下限 48.0.3。48 是上游的 LTS（逢 12 的倍数的版本支持 24 个月，其余只支持 2 个月），而本 crate 只用 wasip1 与燃料计量，49 以后带来的东西用不上。48.0.3 清掉 RUSTSEC-2026-0314／0315／0316 三条公告；这条线还含 GHSA-2r75-cxrj-cmph（path_open TRUNCATE 绕过，修于 44.0.2／45.0.0）与 CVE-2026-58494（hard-link/rename FilePerms 绕过，修于 45.0.3／46.0.1）两处修复——「钉版恒含权限绕过修复」的依据实例。下限写在清单里而不只在锁里，所以修复是一条声明出来的要求，一次 `cargo update` 退不回去。**重开参数**：上游发出下一个 LTS（60），或 48 线停止维护。

**sandbox 增**：`AbsentSandbox` —— 未带执行引擎的构建在缝上的产品实现，逐次以 `E_TOOL_UNAVAILABLE` 拒并携替代臂。它存在的理由是**缺席要是一个判词而不是一个替身**：Echo 放在这个位置会对一个从未运行的 guest 回答「成功」，而第一个察觉的人是相信了那份输出的人。

### 8-13-2 runtime::tools::exec::confinement（宿主进程沙箱：按平台穷尽枚举＋每臂保证清单；形状 3＋2）

上面的 wasip1 面给的是 **guest** 的隔离；exec 的 program／shell 两臂跑的是**宿主进程**，它的隔离只能向平台买，而没有哪个平台把它全卖。故本模块的产物不是开关而是一份分类法：每一臂逐轴说出自己保什么、不保什么，跑在哪一臂上对人与 Agent 都可见（工具 disclosure＋doctor 回报），不是假定。

| 臂 | 保 | 不保 |
|---|---|---|
| `LinuxNamespaces { wrapper }` | 文件系统、网络、进程树、用户 | CPU／内存上限 |
| `WindowsJobObject` | 文件系统（工作目录为副本）、进程树、CPU／内存上限 | **网络**（作业对象不隔离网络）、用户 |
| `CopiedTree` | 文件系统（写入只落副本、源树只读） | 网络、进程树、用户、CPU／内存上限 |
| `Unavailable { missing }` | —— | 一切；`missing` 指名缺的是什么 |

```rust
pub enum Guarantee { Filesystem, Network, ProcessTree, User, Resources }
pub enum Kept { Yes, No }
pub struct Assurances { pub filesystem: Kept, pub network: Kept, pub process_tree: Kept, pub user: Kept, pub resources: Kept }
pub enum Missing { ScratchDirectory }
pub enum Confinement { LinuxNamespaces { wrapper: PathBuf }, WindowsJobObject, CopiedTree, Unavailable { missing: Missing } }
pub struct Offerings { pub namespace_tool: Option<PathBuf>, pub scratch: Option<PathBuf> }
impl Guarantee { pub const ALL: [Guarantee; 5]; pub fn phrase(self) -> &'static str; pub fn unkept(self) -> &'static str; }
impl Missing { pub fn phrase(self) -> &'static str; pub fn recovery(self) -> &'static str; }
impl Assurances { pub fn of(self, axis: Guarantee) -> Kept; }
impl Offerings { pub fn this_machine() -> Offerings; }   // 唯一的采样点；两个 detect 都只读它
impl Confinement { pub fn detect() -> Confinement; pub fn choose(&Offerings) -> Confinement;
                   pub fn assurances(&self) -> Assurances; pub fn name(&self) -> &'static str;
                   pub fn statement(&self) -> String; }
pub struct Confined { /* arm＋scratch＋idle（留给下一条命令的副本）＋outstanding（后台命令的副本） */ }
impl Confined { pub fn detect() -> Confined; pub fn with_arm(Confinement, Option<PathBuf>) -> Confined;
                pub fn arm(&self) -> &Confinement; pub fn statement(&self) -> String;
                pub fn place(&mut self, Command, &Path) -> Result<(Command, Placed), AxError>;
                pub fn settled(&mut self, Placed); pub fn handed(&mut self, BacklogId, Placed);
                pub fn reaped(&mut self, &[Finished]); }
impl Drop for Confined { /* 删留着的副本与未报结命令的副本 */ }
pub struct Placed { /* copy、work —— 私有 */ }
impl Placed { pub fn work(&self) -> storage::FileWork; }   // 这次同步的文件操作计数
pub enum Placement { Sandbox, Host }   // 调用参数 `where`；缺省 Sandbox
pub fn parse_placement(&Map<String, Value>) -> Result<Placement, AxError>;
```

- **保证清单在类型上**：`Assurances` 逐轴五字段，每一臂的 `assurances()` 必须写满五轴，故新增一轴即四臂同时编译红——任何一臂都不会留下一个没人问过它的旧答案。`statement()` 由 `Assurances` 与 `Guarantee::phrase()`／`unkept()` 派生而非另写一段话，句子与类型因此不可能分家。
- **选择是纯函数**：`choose` 取 `Offerings`——有 wrapper 即 `LinuxNamespaces`，否则 `CopiedTree`；scratch 根不可用即 `Unavailable { missing: ScratchDirectory }`。采样只有 `Offerings::this_machine()` 一处（`PATH`＋`std::env::temp_dir()`），两个 `detect()` 都只读它；测试用 `Offerings` 陈述一台机器而不是借一台。
- **`WindowsJobObject` 本构建不构造，且拒而不降级**：`CreateJobObject` 是 workspace `unsafe_code = forbid` 禁止的 FFI，唯一放开它的是桌面 server 的 FFI 缝（`crates/desktop/Spec.lean` D14），而那里只放 Zig 叶子的调用。以 `CopiedTree` 冒充它会对着一个开着网络的盒子回答「网络已关」，正是本模块存在的理由的镜像，故 `place()` 对它返 `E_SANDBOX_DENIED` 并给「改用 copied tree 且让命令离开网络」的 recovery。Windows 上 `detect()` 因此答 `CopiedTree`，其清单逐字写出网络未隔离——这一句就是 Agent 必须看见的那一句。
- **副本按工具一份，每条命令之前同步成工作目录此刻的样子，有界。** `place()` 取这个 `Confined` 留着的副本（没有，或留着的那份抄的是别的目录，就在 scratch 根下新建一个），把它同步成 `workdir` 此刻的样子，命令在副本里跑；`settled()` 在等待结束时把副本留给下一条命令（已留着一份时删掉这份）；交给 backlog 的后台命令由 `handed(id, …)` 记名，其成员报结时 `reaped(&[Finished])` 同样把副本留下或删掉；`Drop` 删掉留着的与未报结的副本。同步的规则：源侧跟随链接（指向树外的链接带进来的是内容，而不是通向人那棵树的入口）；副本侧不跟随链接，因为副本里的东西是上一条命令写的。副本里的一项与源不同类（命令把文件换成了链接、把目录换成了文件），或同名文件内容不同，就先删掉这一项再从源复制，落成一个新的目录项——命令可能在副本里造了指向别处的硬链接，就地改写会写穿到那一头；同名同类同长的文件逐字节比较，相同就不动；副本里源没有的项删掉。于是每条命令开始时，副本与工作目录逐文件相同，上一条命令写下的东西不会留给下一条。逐字节比较而不比 mtime 与长度：同一个时间戳刻度里的等长改写比不出来，副本就会为一个它没有带上的版本担保；比较只读两边的文件，不新建，而实时扫描等的是新建的文件。`Placed::work()` 报这次同步的 `storage::FileWork`（storage-SPEC 8-31）：第一次放置 `created` 是树里的文件数，一次什么都没变的再放置 `created`、`rewritten`、`removed` 都是 0，`walked` 是两侧读过的目录项；`confinement::tests` 的 `a_sandbox_copy_is_synced_rather_than_made_again` 在 N 与 2N 个文件的树上断言这些数。**删不掉不把命令判成失败**（与 `backlog/member.rs`、`collect()` 同一条判断：命令的收场是调用方应得的事实，一个临时目录只值磁盘）。界：`MAX_FILES = 100_000`、`MAX_BYTES = 256 MiB`、`MAX_DEPTH = 64`，按源侧计；越界**拒**并报出越过的那一对数字，已同步一半的副本随之删掉——半份副本会为一堆没带上的文件担保，而本模块的全部理由是防这个。`MAX_DEPTH` 同时终结自指链接造成的无底走查。
- **`Mount`／`Fuel` 不沿用**：`Fuel` 是 wasmtime 指令计量、`Mount.guest` 是 guest 路径别名，二者 wasip1 专属。本模块保留的是**判断**（能力面＝能到达的路径集）而不是词形。宿主环境照旧不继承（exec 的 env allowlist 未动）；`SandboxJob.env` 的显式注入属 guest 面。
- **placement**：调用参数 `where: sandbox|host`，缺省 `sandbox`。`host` 是「在原地跑」——它才是碰得到人那棵树的那一臂，故必须由调用方按名说出，也正是与 A-8 同一条纪律（默认引导先在沙箱里做，出沙箱才需要审批）里「需要审批」的那个动作。python 臂无 host 形（它是 wasip1 guest）：要宿主解释器走 program 臂。
- **公开路径经 `runtime::tools`**：`confinement` 住 `tools/exec/`，doctor 的依赖回报与工具自己的 disclosure 都从 `runtime::tools::{Confinement, Guarantee, Kept, Missing}` 读这一份定义。
- 证据：`crates/runtime/src/tools/exec/tests.rs` 的 `a_sandboxed_command_writes_in_a_copy_and_leaves_the_source_tree_alone`（真命令、真树：沙箱里写得到、人那棵树不动；同一命令 `where: host` 则写进原树——此对拍使「没动」是能力判定而非命令没写）；`crates/runtime/src/tools/exec/confinement/tests.rs` 的 `the_sandbox_arm_a_machine_gets_is_chosen_from_what_it_has`、`every_sandbox_arm_states_what_it_does_not_hold`、`a_sandbox_refuses_a_tree_deeper_than_its_walk_can_end`、`a_sandbox_copy_is_synced_rather_than_made_again`、`a_sandbox_copy_goes_with_the_tool`。

**未决（§3 口径）**：同步仍按命令读两侧的每个目录项，并逐字节比较同长的文件，代价随工作树的大小长；一棵带大构建缓存的工作树每条命令要读两遍缓存。判定它的证据是一棵真实 room 的每条命令同步耗时（毫秒）与其文件数的读数；若读数显示读取成了主项，再比较「按 mtime 与长度跳过、只对同一时间戳刻度里的文件逐字节比较」。

### 8-13-3 runtime::tools::exec::yielding（派出的命令降一级；形状 4 adapter）

agent 派出去的构建、测试与 sprawling 的记账、视图、socket 服务抢同一批核；控制面必须赢，所以 exec 派出的每一条宿主命令都以低于核心的优先级起动，而不是继承核心的优先级。本模块只做「把一条 `Command` 改成降一级起动」这一件事，不决定哪条命令要降——凡经 `through_the_backlog` 的命令一律降，那是全部宿主子进程的唯一产地（§8-14），故降级只有这一处权威。

```rust
pub(super) fn one_level_down(Command) -> Command;
```

- **Windows**：`BELOW_NORMAL_PRIORITY_CLASS`（`0x0000_4000`，`WinBase.h`），经标准库的安全接口 `std::os::windows::process::CommandExt::creation_flags` 在创建时给出，不需要管理员，也不需要 `unsafe`。是绝对档位而非相对档位：核心在正常档时，子进程低一档。
- **Unix**：包一层 `nice -n 10 -- <program> <args>`（`--` 结束 `nice` 的选项，名字以连字符开头的程序才不会被读成选项），工作目录与环境变量逐项搬到外层命令上。`nice` 是 POSIX 规定的工具；增量是相对的，所以子进程总比核心低 10 个 nice 单位，不需要 `CAP_SYS_NICE`（降优先级从不需要特权）。Linux 上核心的 `PATH` 里有 `ionice` 时，再在外面包一层 `ionice -c 2 -n 7`：IO 优先级取 best-effort 类里最低的一级，而不是 idle 类——idle 类在磁盘忙时可以让一条构建一个字节也读不到，best-effort 最低级只是排在核心之后。没有 `ionice`（例如不带 util-linux 的系统）时只包 `nice`：IO 这一半是两半里较轻的一半，缺了它不该让每条命令都起动失败。
- **次序**：降级在清环境与放置之前做，于是 `env_clear` 与环境白名单落在最外层命令上，沙箱的包装（`LinuxNamespaces` 的 wrapper）再包住降过级的命令；优先级沿进程树继承，所以 wrapper 下的真命令同样低一档。
- 失败：Unix 上 `nice` 自己总能起动，找不到的程序只会变成退出码 127 与 stderr 里的一行字，所以本模块在包 `nice` 之前先按 `execvp` 的找法（带分隔符的名字相对工作目录，裸名字沿核心的 `PATH`）确认程序是一个可执行文件，不是就返回 `E_TOOL_UNAVAILABLE`，动作与恢复同 backlog 的 spawn 失败（「check the program name, or use the shell arm」）。找不到 `nice` 本身时，spawn 在 backlog 里以同一个码报出。Windows 上不包外层，找不到程序仍在 spawn 处失败。Sandbox 放置下这一查找发生在宿主上，沙箱里看见的 `PATH` 若不同，结果以沙箱里的起动为准。
- 证据：`crates/runtime/src/tools/exec/tests/yielding.rs` 的 `a_dispatched_command_runs_below_the_core`——同一条读自身优先级的命令，直接起动一次、经 exec 起动一次，断言后者的档位严格低于前者；Linux 臂 `a_dispatched_command_reads_and_writes_at_the_lowest_best_effort_io_level`——经 exec 起动的 `ionice` 读回自己的 IO 档位是 `best-effort: prio 7`。这一条只在 Linux 上编译与运行，先红与转绿都在合并火车的 Linux 任务里看到。

**决定（平台调用的取法）**：子进程的 CPU 优先级取第一档「安全 Rust」——`creation_flags` 与 `nice` 都是对外只给安全接口的现成路，不需要 Zig 叶子，也不需要 Lean 证明边界。**被否**：①起动后再对子进程调 `SetPriorityClass`／`setpriority`——要 FFI（`unsafe` 或 Zig 叶子），且子进程在改档之前已经以正常档跑了一段；②Unix 上用 `CommandExt::pre_exec` 调 `nice(2)`——`pre_exec` 本身是 `unsafe`。**重开参数**：Unix 主机上出现不带 `nice` 的受支持平台，或测得多包一层 `nice` 的起动开销占到一条命令墙钟时间的可见比例。

**逐 run 的 Job Object（`runtime::backlog::jobs`，形状 4 adapter）**：派出的命令起动之后，谁在吃内存要能归到派出它的 run，而一条 `cargo test` 真正吃内存的是它起的 `rustc` 与测试进程，不是 `cargo` 自己。所以 Windows 上每个 run 一个匿名 Job Object：`Backlog::run` 起动的子进程在登记进表的同一时刻装进它 owner 的 job（第一次装时创建），job 里的进程再起的进程由系统自动装进同一个 job，于是 job 的进程表就是这个 run 的整棵进程树。`Backlog::release(owner)` 丢掉这只 job 的句柄；job 不设 kill-on-close，丢句柄不杀进程，杀进程仍只归 `release` 与 `halt`。

```rust
pub struct RunProcesses { pub pids: BTreeSet<u32>, pub unfollowed: u32 }
impl Backlog { pub fn processes(&self) -> Result<BTreeMap<RunId, RunProcesses>, AxError>; }
```

- `processes` 按 run 给出它此刻在表里的进程：owner 是这个 run（窗口内或已转后台）的每条命令自己的 pid，并上这个 run 的 job 的进程表（job 只列还活着的进程）。已经结束、还没被 `harvest` 的命令仍列出自己的 pid，读数的一方在它后面读不到计数。已 `release` 的命令（owner 为 nobody）不归任何 run。
- `unfollowed` 是这个 run 的命令里有几条只读到了命令本身、没读到它起的进程：Windows 上创建 job、装进 job 或读 job 的进程表失败的那几条（这一次读数里整个 run 的命令都算），Unix 上是每一条，因为 Unix 上没有 Job Object（进程组是它的对应物，尚未接入）。装不进 job 不让命令起动失败：job 只服务于读数，为读数让一条构建失败是把代价付错了地方，失败落在 `unfollowed` 里给读数的人看。
- 失败：表够不着时 `E_STORAGE_FATAL`，与 `Backlog` 的其他读法相同。
- 每个进程的内存与 CPU 不在这里读：本 crate 不读平台计数（见下），由 sprawling 的 `bin::monitor` 按这里给出的 pid 去读。
- 证据：`crates/runtime/src/backlog/tests.rs` 的 `a_run_owns_the_processes_its_commands_started`——一条经 `run` 转后台的命令，其 pid 出现在它 owner 的那一项里，别的 run 那一项里没有；Windows 上 `unfollowed` 为 0；`release` 之后这个 run 不再出现。

**决定（job 的取法）**：Job Object 经 `win32job` 2.0.3（`Job::create`、`assign_process`、`query_process_id_list`，对外只给安全接口），本 crate 不写 `unsafe`；子进程的句柄经标准库的 `AsRawHandle` 取得。**被否**：①一整座城一只 job——分不出 run，而分解到 run 正是要它的原因；②只记直接子进程的 pid——`cargo`、`npm`、`sh -c` 这类命令自己几乎不占内存，读数会把一条吃掉几 GiB 的构建报成几 MiB；③`CREATE_SUSPENDED` 起动再装 job 再恢复——恢复线程要 FFI。代价：子进程从起动到装进 job 之间有一小段时间，那一段里它再起的进程不在 job 里（`cmd /C` 这类命令的第一个孙进程在这一段里起动的可能很小，但不为零）。**重开参数**：一个对外只给安全接口的 crate 能以挂起态起动子进程并在恢复前装进 job，或者读数显示 `unfollowed` 之外还有漏掉的孙进程。

**未决（§3 口径）**：核心线程升到正常档之上一级与空转安全阀在 sprawling-SPEC §8-93；派出进程的内存上限尚未落地，卡在一个事实上：`win32job` 2.0.3 对外只公开 job 的工作集上限（`limit_working_memory`，即 `JOB_OBJECT_LIMIT_WORKINGSET`，限的是常驻而不是提交量，按进程计），而在未提权的账户下设这一项被系统拒绝——`SetInformationJobObject` 返回 `ERROR_PRIVILEGE_NOT_HELD`（os error 1314，「客户端没有所需的特权」），普通账户的令牌里没有这一项要的特权，启用特权要 `AdjustTokenPrivileges`，是 FFI。于是这条路在普通账户上让每条派出命令都起动失败，或者静默不设上限，两者都不可取。作业级提交上限（`JOB_OBJECT_LIMIT_JOB_MEMORY`）不要特权，但设它的字段在 `win32job` 里是 crate 私有的，`process-wrap` 10.0.1 与 `windows-spawn` 0.1.0 也不设它。重开参数：一个对外只给安全接口的 crate 公开 `JOB_OBJECT_LIMIT_JOB_MEMORY` 或 `JOB_OBJECT_LIMIT_PROCESS_MEMORY`，或者这一处系统调用改走平台调用规则的第二档（Zig 叶子）。重命令共用的额度池尚未落地：池的大小由测得的核数与可用内存推出，哪些命令算重由城配置给默认表；可用内存的读数只在 sprawling 的 `bin::monitor::memory` 里读（sprawling-SPEC §8-94），由 `bin::assembly` 交给本 crate，本 crate 不读平台。内存紧时新 run 排队已在 sprawling-SPEC §8-46-3。

### 8-14 runtime::tools 四件（形状 4；tools.rs 为纯索引）

```rust
// tools/exec.rs —— 三臂（ExecArm 住 kernel::tool）
pub struct ExecTool { /* workdir、mounts、python_wasm: Option<PathBuf>、sandbox: Box<dyn Sandbox>、
                        shell: Option<PathBuf>、fuel —— 私有；全由装配／执行器注入 */ }
impl ExecTool { pub fn new(…) -> ExecTool; }
impl Tool for ExecTool { /* meta：name=exec、effect=Write{domain}、temporal=Timestamped、render=Terminal */ }
// Program 臂：std::process::Command（workdir 钉定、环境变量白名单——secret 恒不透传）；唯一真子进程产地
//            缺省在 §8-13-2 的 confinement 副本里跑；`where: host` 才在原地（那条路是一条命令碰得到人那棵树的路）
// Python 臂：sandbox.run(python_wasm, argv=["python","-c",code], mounts)；组件缺失→E_TOOL_UNAVAILABLE＋alternative＝Program 臂
// Shell 臂：探测缺失即拒（E_TOOL_UNAVAILABLE，不是降级）；存在则 sh -c／cmd /C；placement 与 Program 臂同一权威（§8-13-2）

// tools/edit.rs —— base_version 乐观并发＋写域双闸＋创建臂
pub struct EditTool { /* city_root、writable: WriteDomain —— 私有 */ }
impl Tool for EditTool { /* meta：name=edit、effect=Write{domain}、render=Diff、temporal=Timeless */ }
// new(city_root, addr, writable: WriteDomain)：writable＝该 Run 的写域（rules.write_domain()）。
// 每次调用先判路径后碰盘，三道依次：within_city（§8-30-1）把本平台的绝对路径换成城里的拼写，落在城外＝
// E_GATE_DENIED，恢复语指向 `exec`（什么算绝对路径由本平台判，§12.4）；Address::parse 杀穿越（..／前导
// 斜杠／空段，E_INVALID_ARGS）；WriteDomain::admits 杀域外与
// reserved prefix（E_OUTSIDE_WRITE_DOMAIN，recovery 报可写前缀清单）。工具静态声明的 Effect 只说它会写，
// 模型选的 path 要在这里判——判定住权威处，而不是 bench 里的第二份判定。
// args：{path, base_version, old, new}；version＝内容 B3Hash 前 16 hex；check_base 拒即 E_VERSION_CONFLICT；
// old 必唯一命中（零命中／多命中＝E_INVALID_ARGS 携计数）；回显＝unified diff＋new_version（逐次 diff 即回档粒度）

// tools/read.rs —— 一个参数，两条路
pub struct ReadTool { /* city_root、catalog: Arc<Mutex<Catalog>>、bound、block_store —— 私有 */ }
impl ReadTool { pub fn new(city_root: &Path, catalog: Arc<Mutex<Catalog>>, bound: ReadBound, block_store: &Path)
    -> Result<ReadTool, AxError>; }   // bound 见 §8-29-1，block_store 见 §8-29-5
impl Tool for ReadTool { /* meta：name=read、effect=Read、cost=Light、render=Generic、temporal=Timeless */ }
// args：{path}。先问 catalog，再当作地址。
// 创建臂：base_version=="new"（16 hex 永拼不出，无碰撞）→ 文件必不存在（存在＝E_VERSION_CONFLICT 报真实版本），
// old 必 ""，new＝全文；父目录自动建（域内已证）。缺文件而非创建形的拒词指向创建形；
// 缺参拒词报四字段契约。理由：没有创建能力的城里，Agent 在空房间里无法开始任何工作；
// 创建住 edit 而非新工具，因为「文件变更＋乐观并发」已是本工具拥有的唯一权威，“absent”只是版本的一个取值。

// tools/status.rs —— 十三字段
pub struct StatusSnapshot { pub who: String, pub addr: Address, pub mode: Mode,
    pub ctx_limit: Tokens, pub trust: String,
    pub write_domain: String, pub locks: Vec<String>, pub worktree_path: String, pub worktree_disk: ByteLen,
    pub signals_pending: u32,
    pub provider_mode: ProviderMode, pub neighbours: u32 }   // neighbours 在末尾，渲染序与声明序同一
pub enum ProviderMode { Normal, Degraded, LocalOnly }
pub struct ChildStatus { pub room: Address, pub kind: DelegateKind }
pub struct StatusTool { /* snapshot＋ children: Box<dyn Fn() -> Vec<ChildStatus> + Send> ＋ backlog: Option<Backlog> */ }
impl StatusTool {
    pub fn watching(snapshot: StatusSnapshot, children: Box<dyn Fn() -> Vec<ChildStatus> + Send>) -> Result<StatusTool, AxError>;
    pub fn reporting(self, backlog: Backlog) -> StatusTool;   // §8-28-2：末行 `backlog:` 从表里现读，与 children 同一理由
    pub fn metering(self, context: ContextReading) -> StatusTool;   // `ctx:` 行的用量从运行的读数现读
    pub fn clocked(self, clock: ClockReading) -> StatusTool;        // `now:` 行从驱动最近的读数现读（§8-53）
}
impl Tool for StatusTool { /* meta：name=status、effect=Read、temporal=Timestamped、render=Generic；渲染序末尾追加 backlog 一行 */ }

// ToolBench 住 runtime::bench（§8-3）：按 Effect 过门是回合层的次序，工具本身以 Box<dyn Tool> 递入。

- **`children` 只携地址与代理类别**：子 Run 在父嚽结之后才开，故父自己那一跑里 **子既无 run id 也无上下文读数**——四个字段里三个只能填零，而零与未知是两件事。现形状只携得出口的两件：派到哪个房间、哪一类代理。
- **`ctx` 的用量为何现读**：快照在派发时冻结，那时还没有任何一次调用，冻结的用量只能是零，而且整跑都是零——一个照 City.md 去问 `status` 的模型会被告知窗口是空的。用量住 `ContextReading`：Run 每回合把 provider 报的 `input_tokens` 写进去，`status` 被调用时读出，所以报的是本跑最近一次已完成调用的计数。上限 `ctx_limit` 仍在快照里，因为它整跑不变。
- **`neighbours` 追加在末尾而不插入到 `signals_pending` 旁边**：冻结序存在的理由是字段表增长时居民的习惯仍可迁移，而一次插入会把前十二行里的一半挪位。它只报**人数**不报名单：名单长度随人口增长，而 `status` 是一份定长文本（`render_children` 已为同一条理由被压成一行）；详情归 `neighbours` 工具，city-SPEC §8-15b。
- **数的是人，不是地址**：一间没人站着的房间没有读者，把它计入会让 `neighbours: 3` 读起来像「有三个人可以说话」而实际上一个都没有。空房间仍然在工具的答案里，因为它对 delegate 与搬入是真信息。
- **`children` 是闭包而不是快照字段**：派活发生在 `status` 工具造好之后，一份开跑前拍的快照永远是空的。派生台住 `collab`，而 depmap 不允许 runtime 依赖 collab，故本模块只收一个答「现在派了哪些」的闭包，装配层把台接上去——与 `RunHooks` 四个闭包同一纪律：第二实现不存在时不引 trait。
// runtime::compaction（形状 6 数据面＋形状 1 判定）
pub enum Content { Prose, Code, Diff, Log, Structured, Table, Markup, Unknown }   // 八类，Unknown 与 Markup 各是其中之一
pub enum Strategy { Head, Ends, Tail, Sections }
pub enum Shrink { Keep, Cut(Strategy), MustOffload }
pub fn detect(text: &str) -> Content;                       // 前几行上的前缀与计数，顺序即设计
pub fn plan(content: Content, size: ByteLen, budget: ByteLen) -> Shrink;
pub fn shorten(text: &str, strategy: Strategy, budget: ByteLen) -> elision::Cut;   // 标记与丢弃计数由 elision 一处产出（§8-42）
// 硬不变量：结果恒不大于输入，且在出口再验一次（真长了就退回原文）。切口落在字符边界。
// Structured 与 Unknown 恒不截断：被截断的 JSON 比缺席的 JSON 更糟；未知内容不拿猜测去丢东西。
// Markup 由以下特征判定： `\documentclass`／`\begin{document}`，或首几行里两条 ATX 标题且无一行像代码
// （源码文件的 `# ` 注释与标题同形，否则会保注释而丢代码）。检测先于 Table：LaTeX 的表格会让一行带 `|`。
// `Sections` 保整节至预算尽，再保被丢各节的标题行（骨架）；少于两条标题即只有一个标题，退回首端裁。
// 机制面（把大结果移出窗口）仍归 offload——本模块只答「缩不缩、留哪一头」。

// runtime::mode 的准入面（形状 1 判定）
pub struct Produced { pub tests_passed: Option<bool>, pub contract_moved: bool,
                      pub held_in: Option<bool>, pub held_out: Option<bool> }
pub enum Admission { Lands, Refused { because: &'static str, alternative: &'static str } }
pub fn admits(mode: Mode, produced: &Produced) -> Admission;
// `None` 不是 `Some(false)`：「没测」与「测了没过」是两件事。证据以 bool 入参而非 eval 的类型，
// 因为 eval 在本 crate 之外，而这里问的不是证据怎么来的，是够不够。UD 是唯一要双验证的模式。

// runtime::redact（形状 1 判定）——账本侧入口只有一个：`turn::ledger::Journal::append_redacted`，
// 它管 model_returned、tool_called、tool_result 三类载荷（§8-41）。
// 窗口块已在此前取出，故思考块签名不受影响；历史与上下文是两个汇，只有一个是永久的。
// 替换物是 `secret:redacted/<b3-16>` 标记而非 Vault 条目：模型复述的钥匙不是城被托付保管的凭证，
// 存它等于给它一条没人要求过的命，而哈希前十六位已足以看出两处是否同一个值。
pub enum Marker { Plain, Fingerprinted }         // 两个汇只差这一个参数，不差第二份实现
impl Marker { pub fn spell(self, found: &[u8]) -> String; }
pub fn redact(payload: &Map<String, Value>) -> (Map<String, Value>, u32);
pub fn redact_text(text: &str, marker: Marker) -> (String, u32);
// 「什么绝不可被打印」在本 crate 只有这一个家：账本走 `Fingerprinted`，诊断行走 `Plain`。
// 历史被检索与比对，故标记要能分辨两个值；一行日志写一次读一次，哈希后缀在那里只是一个
// 没人关联的关联句柄。标记恒可由 `kernel::SecretRef::parse` 解析，realm 恒为 `redacted`，
// 而金库里没有这个 realm——顺着标记去兑的读者得到的是一句诚实的「这里没有」。
pub fn fingerprint(found: &[u8]) -> String;      // b3 前十六位

pub struct ToolBench { /* route: kernel::tool::route::ToolRoute、domain: WriteDomain、
                          taint: TaintSet、seen: BTreeMap<IdemKey, Result<ToolOutcome, AxError>>、
                          prior_public_egress: bool —— 私有。`seen` 是「这把键答过没有、答了什么」的唯一一张表：键只在工具真正答过之后才写入，门拒/语法拒不留条目，故重试不算重放。 */ }
impl ToolBench {
    pub fn new(domain: WriteDomain) -> ToolBench;
    pub fn register(&mut self, tool: Box<dyn Tool>) -> Result<(), AxError>;
    /// Gate routing by declared Effect（收进回合层，executor 归还薄形）：
    /// exec 先 forecast（Suspected → **强制 checkpoint 先行**，见下）；Write → domain 门；
    /// Egress → egress 门（subject 由工具从自己的参数读出）；Spend → 门已接，本构建无声明它的工具；
    /// Spawn → 无门（派生深度是类型，`gate::spawn` 在派活处判）；Govern → 恒拒（run 不改写评判它的那些规则，
    /// 规则在 CONFIG.toml 与楼的 RULES.toml 里由人改）；
    /// Deny 与门的提问（E_APPROVAL_PENDING）都以 `Refused` 作 tool_result 回流，不吞掉回合。
    pub fn invoke(&mut self, call: &ToolCall, key: &IdemKey, now: TimeMs)
        -> Result<BenchOutcome, AxError>;
    // invoke 是下面三段按序串起来的一条调用，三段各自公开，供并行的工具波分开用：
    /// 串行的放行：去重、各道门、exec 的 forecast checkpoint。答得出的（重放、门拒、门问）当场答，
    /// 否则交出放行单 `Ticket`（键、工具名、效果、checkpoint 的 oid —— 私有）。
    pub fn clear(&mut self, call: &ToolCall, key: &IdemKey, now: TimeMs) -> Result<Clearance, AxError>;
    pub enum Clearance { Answered(BenchOutcome), Cleared(Ticket) }
    /// 可并行的执行：交出放行单指向的工具本身，调用方在任意线程上调它的 `invoke`，
    /// 不碰 `seen` 与 `taint`。交出工具而不交出 bench：bench 不是 `Sync`（checkpoint 持有
    /// git 仓库句柄），工具是。
    pub fn tool_for(&self, ticket: &Ticket) -> Result<&dyn Tool, AxError>;
    /// 串行的记账：成功的答案先并入 taint，再连同失败一起记进 `seen`，返回 `Ran`。
    /// 并行波按调用序逐条调它，所以 `seen` 与 `taint` 的写入次序与串行波相同。
    pub fn account(&mut self, ticket: Ticket, answered: Result<ToolOutcome, AxError>)
        -> Result<BenchOutcome, AxError>;
    // 预编译路由：名字→处理器由 `kernel::tool::route::ToolRoute` 在登记时排序一次，
    // 每次调用**一次**二分探测即得处理器（meta/subject/invoke 同一把借用）；
    // 被否：维持 BTreeMap 双探测（一次调用查两次＋每次探测的 String 分配——fx 反例的本仓对应物）。
    // 上面那句「按 Effect 过门」自己的名字住 `Doors`（domain/taint/sandbox/prior 四件同行值，
    // 自拥门判与失败形）：`None` 即此门已开，工具可跑；`Some` 即门已代这次调用给出答案。
    // 与 route 的同一把借用不相斥（字段级不相交借用），故一次探测服务整条链。
    fn admit(&mut self, call: &ToolCall, name: &str, effect: &Effect, subject: &GateSubject)
        -> Result<Option<BenchOutcome>, AxError>;
    /// 一扇门的判定对本 bench 意味着什么：Allow 放行，Deny 与 Ask 都是 `Refused`。
    fn settled(&self, outcome: GateOutcome) -> Option<BenchOutcome>;
    /// 同形：两处出网判定的「首次公开出网要记下来」只住这里。
    fn crossed(&mut self, outcome: EgressOutcome) -> Option<BenchOutcome>;
    /// 包信封的调用者要读 `temporal` 才知道时钟行该不该发。
    pub fn meta_of(&self, name: &ToolName) -> Option<&ToolMeta>;
    /// 检查点署名随网一起交给 bench。一个没有 `Provenance` 的检查点
    /// 写不出 `Sprawling-Run:`，而预测检查点恰恰是在一个拿不到运行上下文的
    /// 闭包里升起的，所以它在装配时就被交下。
    pub fn with_checkpoint(self, net: CheckpointNet) -> ToolBench;
    /// 检查点的三件东西恒同行：仓、它盖住的范围、写它的人。
    pub struct CheckpointNet { pub checkpoint: Checkpoint, pub scope: Vec<String>, pub of: Provenance }
    // `scope` 是这次 run 的**写域全部前缀**，不是它的房间：检查点窄于写域，
    // 两者之间写下的文件就进不了任何检查点（storage-SPEC §8-18）。
    /// 本 bench 服务的那份活。Spawn 门要铸一个人答得出的条目，
    /// 条目要有 actor（问谁）与 artifact（看什么）；两者都不在一次工具调用里。
    /// 未给即拒（fail-closed）——一个人问不到的派生就是没人批准的派生。
    pub fn for_job(self, asking: Address, job: Locator) -> ToolBench;
}
```

- L0 三件恒列 prefix（City.md 只放这一级）；catalog 只收 L2——L0 不进 catalog（名字即文档）但 tool_defs 恒含三件（wire 面要 schema）。
- **否决「Suspected → Discard 门」**：它与 kernel 既有设计冲突，以 kernel 为准。理由：`DiscardRequest` 只有 `Planned`／`Unplanned` 两变体，而 `decide` 对 `Unplanned` **恒判 Deny(NoRestoration)**——把 forecast 的预判包成 Unplanned 送进门，等于让任何含 `rm ` 的 exec 调用全被拒。`kernel::discard` 的注释早已写明正确意图：「text prediction is obfuscatable by design — hits route conservatively, and the git checkpoint net (S3) is the honest backstop」。故 **Suspected 不拒而先立检查点**：强制 `checkpoint.wave_pre` 先行再放行，删掉的东西因而可回档；**无 checkpoint 网时才拒**（`E_TOOL_UNAVAILABLE`），因为「无保护地跑」是唯一没人选择的结局。此路由使 A14 的先行半链在 exec 臂上机械成立。
- ToolBench 持 `Option<Checkpoint>` 具体类型而非新 trait：checkpoint 只有一个实现，为尚不存在的第二实现引缝会造空抽象（AGENTS.md：trait 只在已有第二实现的缝上引入）。
- **`BenchOutcome` 穷尽**：它是判定输出，下游必须穷尽匹配三臂——新增一种答案而不回答它就不编译（§7）；本 crate 的枚举全部如此，没有 `#[non_exhaustive]`，也没有通配臂。
- **三条规则各有一处**：`GateOutcome` 对本 bench 意味着什么只住 `settled`（Allow 放行、Deny 与 Ask 都以 `Refused` 回流），「首次公开出网要记下来」只住 `crossed`，参数的秘密扫描只序列化一次；`admit` 是那个 `match effect` 自己的名字。一份规则在几处各有一份实现，就是几个可以各自漂走的权威。
- `BenchOutcome` 三态：`Ran{outcome, checkpointed}`（checkpointed 携检查点 oid，供波后补记）／`Refused{refusal}`（回流不终止回合；门的提问也走这一臂）／`Duplicate{outcome}`。dedup 先于任何副作用；**key 在工具答过之后才记入 `seen`**，故被门拒的调用重试不算重放。**判重是 `seen` 上的一次 O(log n) 查找**，不抄键、不另立一张「领过权」的集合：第二张集合记的是 `seen` 键集的同一个事实，两份拷贝迟早分叉。工具按 `ToolName` 登记，`invoke` 以调用自带的名字查表，路由一次调用不分配。
- status 的 result 是**按冻结序渲染的文本**而非 JSON 对象：`serde_json::Map` 对键排序，JSON 对象没有读者可依赖的序，「冻结序」会悄悄变成字母序。序是「模型读到的东西」的属性，故落在模型读到的地方。
- 声明 `Egress` 的生产工具是浏览器工具（`bin::browser_tool`）；声明 `Spend` 的工具本构建没有，那扇门以测试替身驱动。

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
- **时间纪律**：时钟只经 `RunHooks::now` 进来，只由驱动在自己的线程上、在串行阶段调用；工具面（`ConcurrentInvoke` 的实现、装配层的 bench、citysim 的闭包）恒不采样，只收读数，`admit` 收的仍是回合时间戳。采样点：dispatch 两次（checkpoint、run_started）；每回合开头一次，得回合时间戳；每次模型尝试发出之前一次（`model_called`），回复到齐之后一次（`model_returned`）；每条工具调用放行之后、工具起跑之前一次（`tool_called`），它的答复交给工具面的 `account` 之前一次（`tool_result`），所以工具面打戳时读到的最新读数就是答复时刻；provider 失败而冻结时一次；结束时 freeze 一次，handoff 用它、run_frozen 用它＋1；取消时 freeze 沿用被打断那个回合的时间戳，因为这次冻结属于那个回合。回合里其余的行（`prompt_assembled`、`prompt_shape_compared`、`steer_received`、`cancel_received`、波前的 `checkpoint_committed`）带回合时间戳。哪几种行记自己的时刻，只由 `turn::ledger` 的 `Authored`／`Carried` 变体定一次：那四个变体不带读数就造不出来。计数器闭包（citysim）下采样的次数与次序是剧本的函数，所以字节照样可重放。这四种行的 `t` 怎么读，以 kernel-SPEC §8-4「信封 `t` 记的是什么」为准。
- **开头只读段的时刻**：各条在放行之后逐条采开始；全部 join 之后按调用序逐条入账，入账前、工具面的 `account` 之前采答复，所以一条的答复时刻是「这一波在调用序上轮到它入账的时刻」，不早于它真正答完，不晚于最慢那条答完。生成中提前起跑的读，开始时刻记成它被放行的那一刻：这两行量的是这一波为它花了多久。重开参数：出现声明 `Effect::Read` 而常超过 1 s 的工具时，改在工作线程上采样，那要一个 `Sync` 的时钟。
- **结束判定**：`calls_made == 0` 且这一答**说了话**，即 `Completion::Done(Evidence[model_returned])`；`calls_made == 0` 而内容为空、或 `stop == MaxTokens`，即 `Completion::Limit`（§8-37）；任一安全点命中 Cancel 即 `Completion::Cancelled`。三条均经 `freeze` 出口，故 **handoff_written＋run_frozen 是唯一出口**，无第二条退路。第四点 `BeforeSpawn` 与前三点同权：命中即 `Cancelled`，那个回合的 assistant 与 tool results **不入窗**，因为窗口前推是「回合成立」的后果而不是它的一部分。
- **第四种结束：回合中途的失败。** **两种 carrier 都经 `freeze` 出口，差别只在冻结之前写不写载体事件**：带 `Carrier::Event` 的码先写载体事件，`Carrier::Loadtime` 的码直接冻结。理由：「账本自身就是受害者时，没有什么真实的东西可写」对 `CasCorrupt`／`StorageFatal`／`LogVersionUnsupported` 成立，对 `WireMismatch` 不成立——供应方把兑换格式写错与账本健否无关；而对前三个码，写不进去的后果就是 `freeze` 的 append 自己失败并把那个失败向上抛，这比预先判定「写不进去」更诚实。一次没有冻结的 run 在账本上只剩 `run_started`，重启后仍报 `frozen: false`，页面就把每条消息都当 `steer` 发。冻结后**原错误仍然向上抛**：账本得到判决，调用方得到诊断，两件事不互相替代。否决「把 `WireMismatch` 重分类为 `Carrier::Event(ProviderDegraded)`」：该码在握手期也用于 wire 版本不匹配（那时连 run 都不存在），一个码两种含义去改分类表，会让 `kernel::event::kind` 那条「loadtime 白名单封死在五个」的测试变成对一件无关的事作证。
- **Conversation 归驱动持有**：入窗内容就是回合报告的前推结果（assistant＋tool results），放在调用方手里等于把一条不变量交给每个调用方自己维护。
- **闭包而非 trait，`invoke` 除外**：`now`／`interrupt`／`checkpoint`／`wait` 的第二实现尚不存在，而本库的纪律是 trait 只在已有第二实现的缝上引入；`invoke` 是 §8-3 的 `ConcurrentInvoke`，它的第二实现已在缝上。`RunHooks` 自身只是引用的容器，不持策略。
### 8-16 runtime::digest（形状 1 判定＋形状 2 值类型）

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

### 8-17 runtime::diagnostics（形状 4 薄壳＋形状 6 数据面）

设计权威是 `docs/logging.md`；本节只记接口与三处口径差异。

```rust
pub enum Level { Refuse, Effect, Decide, Trace, Wire }  // 全序：层底控到该级为止
impl Level { pub const DEFAULT: Level = Effect; pub const ALL: [Level; 5]; pub fn parse(&str) -> Option<Level>; }
pub struct Site<'a> { pub run: RunId, pub seq: Seq, pub module: &'a str }   // 三字段必填
pub type Sink = Box<dyn FnMut(Entry<'_>) + Send>;   // 一条 entry，不是一行文本（§8-38）
pub struct Diagnostics { /* floor: Option<Level>、sink —— 私有 */ }
impl Diagnostics {
    pub fn new(floor: Level, sink: Sink) -> Diagnostics;
    pub fn off() -> Diagnostics;
    pub fn floor(&self) -> Option<Level>;
    pub fn admits(&self, level: Level) -> bool;
    pub fn write(&mut self, level: Level, site: Site<'_>, message: &str);
    // 无读方法。这是本模块全部保证的形状半边
}
// 无自己的打码器：`write` 交给 sink 之前调 `redact::redact_text(message, Marker::Plain)`
```

- **无读方法即全部形状保证**：「判定与恢复逻辑不读日志」不靠纪律，靠这一点——把一行读回来在类型上拼不出。推论就是收口条件：删光日志，行为、重放与总账逐字节不变。
- **行上恒无时间戳**：锚点是 `seq`——两条时间线靠一个整数对齐，而采样壁钟会在一个不允许采样的库里开第二个时间源。想要时间的 sink 在装配层自己加。
- **坐标由 Ledger 自己说**：`storage::JsonlLedger::position()`（返回「现在写一条会落在哪」）。只给位置不给内容：一个能读记录的访问器会把判定逻辑引到它正在写的账上去。
- **双重防线**：`Sealed` 无 Debug/Display，入行在类型层就不成立（反例 `tests/ui/log_a_credential.rs`）；普通字符串里的明文由 `redact::redact_text`——**同一个**扫描器与**同一份**替换实现，不是第二个——就地换成 `secret:redacted`（`Marker::Plain`）。不丢整行：周围那句话通常正是读者要的。
- **不引 `tracing`**：它在此处的唯一功能是跨 `await` 携模块名的 span，而回合路径是同步的，该功能无消费者。理由见 `docs/logging.md` §7。
- **写入方三处**（§6 的三类各一）：命令被拒（`refuse`，写在 `handle` 而非调用方，因为每个调用方都要）；endpoint 附着与探测结果（`effect`）；dispatch 跑完（`effect`，作为指向 Ledger 的指针）。

## 8.5 两个设计

**turn 侧**：中断作相变入参（选中）vs 独立 `cancel()` 方法。后者表面更直观，但 cancel 方法可在任意持有点被调＝相内中断可表示，A9 退化成时序约定；选中方案把边界快照做成相变函数的形参，相内无入口，结构即断言。代价：调用方每相必须显式给 Interrupt（哪怕 None）——这个啰嗦是刎意的：它迫使执行器在每个边界问一次信号面。

**fork 侧：fork 消费 VerifiedLedger**（CLI 的 `fork` 与 `prefix` 仍如此；分支的对话重建只有 `inherited_indexed` 一扇门，理由见 8-2：账本唯一的写者打开账本时已验过，再验一次整链只为读一条 run 的几行，代价随历史长度增长）——分叉前必先验链，类型上把「从未验证的序列分叉」做成不可表示；分叉正确性与重放正确性因此是同一条断言。
**B（落选）：fork 直接吃原始行**（`prefix(lines: &[Vec<u8>], at_seq)`）——少一次验证成本，但打开「对损坏历史分叉」的路径，且 at_seq↔行号对应要自行重解 envelope＝第二解析权威。落选理由：验证成本 O(n) 在分叉频率下可忽略，而不变量 14（citysim 检查器）需要的正是 A 的类型保证。另 `verify_dir` 命名族落选：与 `verify_ledger_dir` 二选一，取后者（dir 一词泛滥易撞 S3 worktree 面）。

## 9 工作流程

`just replay <log>` → `sprawling replay` → `storage::audit_chain` → 全绿报行数与 tail seq，违规报 three-part。citysim 检查器与 A19 测试直接调 `verify_lines`／`prefix`。

## 10 实现逻辑

envelope 探查与全解共用 kernel 的解析（Value 探查仅取五键，不建第二记录类型）；行号从 1 计（人读）；错误 recovery 字段给「重放同一夹具于更新版本」或「检查介质」两句可执行建议。

## 11 边界枚举

空序列（合法：VerifiedLedger 空，tail_seq=None；fork 于其上恒越界）；**目录存在但不含任何账本段**（在本模块合法且与空账本同形；人输入路径的拒绝在 CLI）；单行创世；`at_seq=FIRST`（前缀＝仅创世行）；`at_seq=tail_seq`（前缀＝全量）；ig:true 且 kind 已知（照常全解，ig 只授未知时的跳过权）；篡改中段一字节（链断于下一行报错）；两段夹具跨段验证（storage 读面已拼平）。

## 12 Decisions

- 恢复层不造码（§8-49）：恢复段的失败恒是它拿到的那个类型化错误，可定义性随原码走；该层改变的只是「同样的请求再发一次」与「换一扇门再问一次」之间的选择。

- `E_INVALID_ARGS`（at_seq 越界）：不可定义掉——「从已冻结 Run 最后事件之后分叉」是用户可达输入；静默夹取是被明拒的替代。
- `E_LOG_VERSION_UNSUPPORTED`（v 判向＋未知 kind 无 ig）：不可定义掉——数据比二进制长寿。
- 链断/seq 洞/非规范字节：以 `E_CAS_CORRUPT` 报（存储完整性族；subject=行号与路径）——能否定义掉＝「介质位腐烂在设计边界外」，同 storage-SPEC §12。

### 12.1 定规：来源行不带摘要生产者指纹

**决定**：`prompt_assembled` 的来源行只记 `{addr, kept, marker, dropped}`；没有 `SummaryProducer`，也没有铸印它的 `compaction::producer::mint`。

**理由**：本仓库不做 LLM 压缩，没有任何路径产出摘要文档，生产里每个 `SourceDoc` 的指纹恒为缺席；一个没有写入者的字段让读者以为账本记下了一件它从不记的事，而铸印函数只有测试调用。

**被否**：保留字段与 `mint` 等将来接上——死机制照样要维护、要进公开面，而接上时的口径（谁写的、第几代）应由那时真实的摘要生产者决定，不是预先猜好。

**重开参数**：出现一条把模型写的摘要放进前缀的生产路径时，重开本条，指纹随那条路径一起设计。
### 12.2 定规：回合边界的压缩只在收尾边界换快照
- **决定**：一回合的 exchange 只允许在 `Turn<Recording>::record` 的收尾边界被压缩替换，时机是工具波全落地之后、一回合一次；判定由 `compaction::plan` 一处给出（8-44），`turn.rs` 不写任何阈值比较；`runtime::fork` 在同一边界重放同一判定。
- **理由**：压缩若在波中换快照，它看见的是半截波，分组与 `fork` 的逐回合重建不同，同一历史会折出两种字节，分支与母亲分叉——这正是确定性回放（ARCHITECTURE §10）要排除的失败。收尾边界是这一波的唯一完整分组点。
- **击败的备选**：①逐结果在波中压缩——分组随落地次序漂移，重放无法复现；②在 `turn.rs` 写一份阈值判断——「装不装得下」已经有一个家（`compaction::plan`），第二个家会各自漂移；③在边界把 `MustOffload` 的文本换成引用——这扇门没有 store，且城里没有工具解析得了引用，模型拿到的将是一条跟不上的指针，故该类文本原样保留、以 Ledger 为回路。

### 12.3 定规：run 结束即终止它留下的后台命令
- **决定**：`release(run)` 把这个 run 的命令改记为 `Nobody` 并终止它们，而不是让它们跑完（§8-28-1 第 4 条）。
- **理由**：结束之后没有任何一次工具结果能把结局交给它；`Sandbox` 放置的命令跑在 `Confined` 的副本里，而那份副本在同一次 drop 里被删，让它跑下去等于让它对着一棵已删的树跑；一条无主命令留着的进程、句柄与临时目录是每个会话的边际内存。
- **击败的备选**：让无主命令跑到自然结束、再把结局写成该 run 名下的一条账本事件——结局到达时已经没有读者，而事件表要为一件被选定不再发生的事多一种事件。
- **重开的参数**：`Host` 放置成为后台命令的常态、且其对宿主树的副作用本身就是人要的产物时，run 的结束不再是「没人要」的证据，这条规则应重新论证。
- **未决**：被终止的命令不在账本里留下一行：把「结束时终止了 N 条后台命令」写进账本需要 kernel 事件表多一种，归 kernel-SPEC 的事件表。

### 12.4 定规：一条路径算不算绝对路径，由本平台判定
- **决定**：`within_city` 只接手标准库 `Path::is_absolute` 在本平台答「是」的路径。于是同一个拼写 `/abs.txt` 在 Unix 上是城外的绝对路径，`read`、`search`、`edit` 答 `E_GATE_DENIED`，恢复语指向 `exec`；在 Windows 上它没有盘符，不是绝对路径，照旧交给文法，答 `E_INVALID_ARGS`。两个平台给出两个码。
- **理由**：模型照抄的是 serve 所在的系统给它看的路径——拖进来的文件、`exec` 的输出、日志——而这些路径都按本平台的规则写成。判定跟着 `Path::is_absolute` 走，仓库里就没有第二份「什么算绝对路径」的规则。Windows 上 `/x` 与 `\x` 指向进程当前盘的根，当前盘由启动 serve 的方式决定，不是城的属性。
- **击败的备选**：在 Windows 上把根相对路径按当前盘补全后再判。这要给 `within_city` 加一条只在 Windows 上存在的分支，而且同一条路径会随进程的当前盘落到不同地方。
- **重开的参数**：某个页面或 harness 在 Windows 上以根相对的形式给出城内文件的路径，模型照抄后被文法拒绝。

### 12.5 一回合里等来的四个时刻各采各的，钟交给回合的账本门

**决定**：`model_called`、`model_returned`、`tool_called`、`tool_result` 的信封 `t` 记各自那一刻，每条工具调用采开始与答复两次。时钟在 `Turn::begin` 交给 `turn::ledger::Journal`；`Authored::ModelCalled`、`Carried::ModelReturned`、`Carried::ToolCalled`、`Carried::ToolResult` 四个变体各带一个读数，其余变体只能带回合时间戳。

**理由**：结果上的 UTC 戳（§8-10）、`view --since/--until` 的时间窗与 run 页的调用用时读的是同一个值，一行只有一个时间。只采答复不采开始，调用用时只能推算，会把准入里的检查点提交算进工具用时。钟放在 `Journal` 里，`call` 与 `execute_concurrent` 的参数表不动，采样只发生在回合自己那一道账本门后面。

**被否**：①`ToolResult` 载荷加 `returned_ms`：一行两个时间，读者要知道信哪一个；②时钟随 `Generating` 进 `call`，再与工具面、`still_going` 合成一个值进 `execute_concurrent`：同一只钟两个入口，而 `call` 的参数已按 `budgets.toml` 钉在 5；③只采答复：两条并行对拍测试可以原样保留，但用时量不出来。

**代价**：同一 run 内 `t` 不再随 `seq` 单调，次序以 `seq` 为准；并行对拍测试改用停住的时钟比字节；`golden-p0` 重生成一次。

### 12.6 首个内容只数文字与推理，钟在回合里读

**决定**：`first_at` 是第一段非空文字或推理到达时回合读到的时刻（§8-50）。工具调用不算首个内容：模型口今天的增量只有这两种，只带工具调用的回复没有 `first_at`。

**理由**：回合的钟只有一个入口，即 `turn::ledger::Journal`（§12.5）；模型口的实现不读钟（`kernel::Model` 的约定），所以时刻只能在口的消费方读。增量汇点是每一扇流式门都经过的地方，包它一层就量到了三种兼容格式，gateway 一行不改。

**被否**：①gateway 在流里采样，再随 `ModelReturn` 交回：模型口多一个读钟的实现，citysim 的脚本模型也要学会造一个时刻；②给 `kernel::Increment` 加一种「工具调用开始了」的片段：增量随 `ServerFrame::Delta` 上线，那是一次线协议改形，每个读增量的页面都要多一臂，为的只是只带工具调用的那类回复；③把提前交出的完整工具调用（`EarlyCalls`）算作首个内容：只有 Anthropic 的流交出它，而它到的时刻是那一块的结束，不是开始，同一个字段在两种兼容格式里会量两种东西。

**重开参数**：run 页画首字耗时时，只带工具调用的回合多到让那一列大半空白。

### 12.7 定规：run 的开篇与收尾归 `Charter`，harness run 不借 `RunPlan`

**决定**：开篇两行与收尾两行由 `run::charter::Charter` 写，`Run::dispatch`、`Run::freeze` 与 `run::harness::HarnessRun` 都经它；`Charter` 是从 `RunPlan` 借出的视图（`RunPlan::charter`），harness 一侧从它自己的值借出同一个形状。harness run 的行序与结局判定住 `run::harness`，不住装配层。

**理由**：harness run 与模型 run 的开篇、收尾是同一件事：同样的 JobPinned、同样的 `run_started` 载荷、同样的冻结对与 1 ms 间隔。两处各写一遍，`run_started` 多一个键时（例如 `mode`）两份会各改各的。结局判定放在本 crate，是因为 `lifecycle::concluded` 已经是「一答冻成什么」的家，空回答冻成 `Limit` 这一条两边必须同一个读法。

**被否**：①给 harness 填一份 `RunPlan`：`CallShape` 与 `FrozenPrefix` 是城没有的事实，窗口、提醒与 `transcript` 都会读它们；②装配层直接 append 这些行：run 生命周期的行就有了两个作者，JobPinned 的次序与冻结对的 `t`/`t + 1` 规则各写一份；③把 `RunPlan` 拆成 `Charter` 加模型部分：每个构造 `RunPlan` 的地方（citysim、装配层、测试）与每个读 `plan.run`／`plan.who` 的地方都要改，借出的视图给出同一条映射而不动它们。

**重开参数**：`RunPlan` 的开篇字段与 `Charter` 的字段不再一一对应（例如 `run_started` 多一个只有模型 run 才有的键），视图要带 `Option` 时，改为 `RunPlan` 持有一个 `Charter` 值。

### 12.8 戳从驱动最近的读数渲染，到秒，默认每分钟；回合在工具面打包之前读答复时刻

**决定**：结果上的时钟行渲染成 ISO 8601 UTC、到秒（`clock: 2026-05-14T09:31:07Z;`）；读数取自 `ClockReading`，即驱动在工具面打包之前最后一次读到的那一刻（§8-53）；`turn::wave` 在调用工具面的 `account` 之前读答复时刻，所以那一刻就是答复时刻；`CLOCK_STAMP_DEFAULT` 是 `Minute`，于是 `Timestamped` 工具每条结果都带戳，`Timeless` 工具只在分钟桶变了时带；`Off` 仍逐字节等于没有这个功能。

**理由**：模型要知道一条命令是什么时候答复的，靠回合的时间戳说不出来，一回合可以跨几分钟；一条跑两分钟的命令，戳若是开始时刻就早两分钟。读数经 `ClockReading` 来，一跑仍只有 `RunHooks::now` 一个采样点，戳上的秒数恒等于账本里某一行的 `t`；ISO 形状是人读、模型读与 `view` 解析共用的一种写法，到秒是因为分钟在一回合之内分不出先后。

**被否**：①工具面自己读一次钟打戳——那是一跑里的第二个采样点，戳上的秒数可以与它 `tool_result` 的 `t` 差一秒，计数时钟下的剧本也会多出采样而改变字节；②用 `admit` 收到的回合时间戳——一条命令的戳会早于模型给出这条调用的那一刻；③在回合的 `account` 里打戳——要 `turn::wave` 把打包移出工具面，而把答复读数挪到 `tools.account` 之前已经给出同一个秒数，工具面一行不改；④答复读数留在写 `tool_called`/`tool_result` 两行之前、工具面的 `account` 之后——戳就是开始时刻，一条长命令的戳早出它跑的那么久。读数挪动不改变采样的次数与次序，计数时钟下的剧本与并行对拍测试的字节不变。

**重开参数**：出现要本地时间的读者、且城配置开始受理 `[clock] zones` 时，时区行与偏移格式重议；工具面开始在 `account` 里做耗时可观的事（例如同步写盘）时，重议答复读数是否仍在它之前。

### 12.9 城外书架上的 skill 由 catalog 携着正文交给 run

**决定**：阅览室准入的一件城外书架上的 skill 经 `Catalog::admit_carried_skill` 进 catalog，条目的 `expansion` 是扫描读到的 `SKILL.md` 正文，`read <名>` 交回这份正文（§8-29-6）。包里其余文件对它不可读。

**理由**：`docs/getting-started.md` 告诉人，挂上 `[skills] shelves` 再在 `reading_room` 里写下名字，居民就用得上那件 skill；而城外的文件没有城内地址，`read` 只开城根下的路径。携正文让这句话成真，又不给 `read` 开一条去城外读文件的路：模型能选的仍只有城内路径，城外的字节只以「人准入过的那一份」的身份进来。正文与哈希出自同一次读入，所以 pin 说的字节就是 run 读到的字节。

**被否**：①给城外持有编一个城内地址或把城外目录映射进城根——那是一个指向并不在那儿的文件的地址，失败落在第一次 `read` 而不在写清单的时候（city-SPEC §8-8）；②让 `read` 在调用时去城外路径读——读到的可能不是钉住的那份字节，且 `read` 多出一条不经读界判定的打开路径；③改文档，告诉人城外书架只能浏览不能用——书架挂了却用不上，人从 catalog 上看不出为什么。

**重开参数**：城外 skill 的包里附属文件也要按名读到（例如一件 skill 的 `SKILL.md` 指名它目录里的脚本）时，`Holding` 要带整包的字节或一份只读的城内镜像，本条与 city-SPEC §12.9 一起重议。

### 12.10 沙箱副本按工具一份、每条命令前同步，而不是每条命令新建一份

**决定**：`Confined` 留着一份副本，`place()` 把它同步成工作目录此刻的样子：不同的文件删掉再复制，多出来的项删掉，相同的文件不动（§8-13-2）。

**理由**：新建文件是实时扫描等待的地方。在 Windows x86_64 笔记本级、Defender 实时防护开的机器上，放置一棵 512 个 16 KB 文件的树 p50 约 0.9 s，约 1.8 ms/文件，磁盘空闲 98% 以上（这是工作树放置的读数；复制同样多的文件是同一种等待，这一点是推断）；每条沙箱命令复制整棵树，就是每条命令付这一次。同步之后，一条命令新建与改写的文件数等于上一条命令动过的文件加上工作目录变了的文件，与树的大小无关；树的大小只进目录项读取与同长文件的比较。

**被否**：①每条命令新建一份（原做法）：新建文件数是命令数乘以树的文件数；②用硬链接「复制」：命令在副本里的写入会写穿到人那棵树，共享可写文件的副本不是隔离；③按 mtime 与长度判定相同：同一个时间戳刻度里的等长改写判不出来，副本会为它没带上的版本担保；④就地改写不同的文件：命令可能把副本里的名字做成指向别处的硬链接或链接，就地写会写到那一头，所以不同的项先删掉再建。

**重开参数**：一个工具的两条命令需要同时持有同一份副本（今天后台命令各持一份，留着的只有一份）；或读数显示同步的读取成了每条命令的主项（§8-13-2 的未决）。

### 12.11 「只新建」下 exec 只在副本里跑，不开放 host

**决定**：`WriteLimit::Create` 下，exec 的 `host` 放置在起进程之前被拒；`sandbox` 放置照常（§8-55）。

**理由**：host 上的一条命令能覆盖、删除、改名任何它够得到的文件，也能建硬链接，而不要管理员权限的前提下，没有一种手段能让已有文件对它只读；`Create` 承诺的是「已有文件不变」，给不出这个保证的路就不开放写入。副本放置本来就把写入留在副本里，所以它在 `Create` 下的行为与原来相同，什么都不必改。

**被否**：①在 host 上跑完再比对、改了就回滚：回滚发生在改动之后，期间读到这个文件的人与进程看到的是改过的字节，而且一个被删掉又建回来的文件已经不是原来那个目录项；②把命令新建的文件从副本搬回树上：要给副本里每个新文件再判一遍写域与「只新建」，这是第二条写路径，今天没有读者要它。

**重开参数**：出现一个能让已有文件只读的放置（例如 `LinuxNamespaces` 臂把工作目录只读挂进去）时，那个臂在 `Create` 下可以开放；出现要在 `Create` 下由命令产出新文件的场景时，重议第②条。

### 12.13 时刻只有 `iso` 写出的那一种拼法可以读回，时间段是半开的、两端在构造时核对

**决定**：`parse_iso` 只收 `iso` 写出的形状（§8-10），`UtcSpan` 是 `[since, until)`，`until <= since` 在 `UtcSpan::new` 里拒绝（§8-57）。

**理由**：人从时钟行、`status` 的 `now:` 行或 `view` 的输出里抄下一刻，抄来的就是这一种形状；读回只收它，`iso` 与 `parse_iso` 互逆是一条可以逐值检验的性质，而不是两份各自宽容的规则。半开区间让相邻的两段（今天、明天）不重不漏，一行恰好落在交界那一毫秒时只属于后一段。矛盾的区间在值里拒绝，而不是让每个读者各自判一次：一个打错顺序的命令行得到一句错误，而不是一段看起来正常的空历史。

**被否**：①也收时区偏移并换算成 UTC——城的配置拒绝时区（city-SPEC 12.7），收偏移就有了第二条关于时区的规则；②也收只有日期、或带小数秒的写法——每多一种写法就多一条「它等于哪一刻」的规定（一天的开始还是整天、截断还是舍入），而它们都不再与 `iso` 互逆；③闭区间 `[since, until]`——相邻两段在交界那一毫秒重叠；④`until <= since` 时答空——与「这段时间里什么都没发生」分不开。

**重开参数**：页面或居民工具要按本地日期选一天时，「一天」由读者的时区展开成一个 `UtcSpan`，那时再定展开规则住哪里；本模块仍只读 UTC。

### 12.14 开篇的写法记在 `run_started` 上，`FromJob` 的分支仍改写第一条消息

**决定**：`Opening` 搬进 kernel，`run_started.opening` 记它；`fork::fold_run` 照记下的写法重建母 run 的第一条消息，只有 `FromJob` 改写成 `Inherited`（§8-58）。

**理由**：provider 的前缀缓存只认逐字节相同的前缀，分叉的第一个请求要以母 run 最后一个请求的字节开头；重建者唯一的材料是账本，所以写法必须在账本上。值的定义只能有一处：runtime 依赖 kernel，载荷住 kernel，所以枚举搬过去，runtime 再导出它，调用方的路径一处不改。`FromJob` 那一句指向母 run 前缀里的 JOB.md，分支没有那份文本，照抄是一条指向空处的话；把文本抄进分支的 run 段会让 run 段与母 run 不同，缓存一样不命中。

**被否**：①在 kernel 另立一个两值的记录类型、runtime 保留自己的三值枚举——同一件事两个定义，两边的词迟早不一致；②按 `goal` 是否为空推断写法——那是 `city::write_brief` 的规则在分叉里的第二份；③`FromJob` 的分支也照抄那一句——分支读到一条找不到对象的指示；④把母 run 的 JOB.md 抄进分支的 run 段——缓存在 system 段就不命中，还多一份没人派给分支的任务。

**重开参数**：分支的前缀能带上母 run 的 run 段（例如分支沿用母 run 的 brief 而不另写一份）时，`FromJob` 也照抄。

### 12.15 连接器把声音块存进 CAS，交出的是 locator 而不是附件

**决定**：MCP 答复里的 `audio` 块与图片块走同一步：容器认得、base64 解得开、不为空时存进 CAS，块换成一行带 locator 的字（§8-27-10）。它不进 `ToolOutcome.attachments`。

**理由**：桌面 server 录下的声音落在它自己机器的临时目录里，城里没有工具读得到；答复是它交回城里的唯一一条路（`crates/desktop/Spec.lean` D13）。base64 留在窗口里会把一段两分钟的录音变成五百万个字符，账本也跟着记下它们，所以它与图片一样在这一步离开答复。附件是模型要看的东西，`ImageRef` 带宽高，provider 的线把它画成图片；声音没有一个模型能读的形状，要读它的是一件工具，工具要的是 locator。连接器不判容器：哪些容器送得出去，由读这段录音的那一方判（`gateway::AudioType` 是那张表），连接器只认 `audio/` 这个前缀，于是容器的认法仍只有一处，而 runtime 也不为一张表多一条到 gateway 的依赖边。

**被否**：①给 `ToolOutcome` 加一个声音附件的槽（kernel 的类型多一臂，provider 的两条线都要为一种它们画不出的东西写一条拒绝）；②原样留在窗口里（见上）；③存进 CAS 并照着落盘一个 rest 文件给 `read`（`read` 读的是文本，一段 wav 的字节对模型没有用）。

**重开参数**：provider 的线有了声音输入的形状，模型自己能听；那时声音进附件，与图片同形。

### 12.16 按字节读的门在 runtime，交出一个 `Read` 与字节的来处

**决定**：`chosen_path` 的判定经 `BoundReader` 公开（§8-59）；它收模型写下的参数，判过之后交回一个 `Opened`：一个 `Read`，加上 `Named` 说它是一个文件还是一个块。`read` 读 Locator 也经它。

**理由**：判一条模型选的路径要依次做四件事（换成城里的拼写、文法与保留区与读界、解开链接再判、打开后核对没换过），漏掉任何一件就是从侧门把 `admit` 拒掉的东西交出去；把这四步交给每个调用方去拼，等于每个调用方都要记得它们。交一个 `Read` 而不是一份字节，是因为上限属于收字节的那个值：录音的上限在 `gateway::Recording`，图的上限在 `IMAGE_MAX_BYTES`，门若收一个上限参数，这两个数就要被抄到调用点。`Named` 是因为录音的容器在文件上与在块上认法不同，调用方要知道它拿到的是哪一种。

**被否**：①把 `chosen_path` 的几个函数直接公开，让 accounting 自己拼——四步的次序与 `still_judged` 都会在 crate 外再写一遍；②门收一个上限、交回 `Vec<u8>`——上限的数离开它的值，而 `ImageMaxBytes` 本来就不交出它的数；③门放在 accounting——它照样要 `chosen_path` 公开，判定仍会分在两个 crate。

**重开参数**：一个要列目录或要 `nearby` 的按字节读取者出现时，把 `read` 的没命中答复挪进这扇门。

## 13 依赖选型

kernel、storage（读面与 cas）；serde_json（envelope 探查）。dev：proptest、tempfile、trybuild、insta（prefix golden）。
`wasmtime`／`wasmtime-wasi = "48.0.3"`（feature `wasm` 内藏，钉版理由见 §8-13；wat 为 dev 依赖供 A10 模块）；`similar`？否——unified diff 自写最小形（edit 回显只需逐行对照，不引第三方 diff 库；被否理由：依赖面换一处 80 行纯函数，不值）。其余无新第三方（分段哈希经 kernel `B3Hash::digest`，不直依 blake3）。

## 14 硬编码声明

无（行号计法与 recovery 文句不构成行为常量）。

两处 pub(crate) 数据面（改须本 SPEC 同集）：信封附件封顶 `ENVELOPE_ATTACH_MAX_BYTES=1024`（§8-7：附件与负载分账的断言界）；net_notice／truncation／offload 提示句三定句（ASCII，住 pipeline／offload 实现内，改句＝改入窗字节＝过本 SPEC）。
一项常量读取（值与理由住 `kernel::consts_policy`，本文件不复写）：`EXCHANGE_BUDGET_BYTES`——回合 exchange 的入窗预算，`compaction::exchange` 是唯一读者。

## 15 影响面

citysim 链检查器复用 verify_lines；bin `replay` 子命令接线；prefix 重建器消费 VerifiedLedger。
citysim 是 `run::drive` 的调用方（§8-15），assemble 的签名变动波及它；kernel::model 的 canonical 类型波及 ScriptModel；ToolBench 持有全部门判；E_TOOL_OUTCOME_UNKNOWN 补写面（replay 的 dangling 检测）供 resume 路径消费。

## 16 测试与约束

单测：五步各拒绝分支＋ig 跳过；fork 越界；fork_draft 载荷形。proptest：对任意合法 draft 序列（经内存 Ledger 物化）verify 恒过；任意单字节翻转恒拒。A2/A19 演示测试入 crates/runtime/tests/。约束：clippy 零告警。
A4 golden（build_prefix 重跑逐字节同）；A15（rebuild_prefix 对拍）；A7 往返四断言；A18 零字节；watchdog 分级序；A10 三断言（feature `wasm` 下真 wasmtime＋WAT）；L0×失败注入矩阵（三臂×（正常／工具错／拒收））；ToolBench 门路由（Deny 回流／门的提问回流／dedup 先于副作用）。
回合边界的压缩（§8-44）：`compaction::exchange::tests` 三例（预算内全保留；超预算回复按 `plan` 裁、结构化结果整块保留；份额随 exchange 大小走）；`turn/tests/compaction` 两例（收尾边界才换快照、波中达阈值按全波分组一次压）；`fork/tests` 一例（分支继承的是压缩后的 exchange）；citysim `compaction` 两例（波中达阈值账本全字节保留、同剧本逐字节重放）。

## 17 模型体验

入窗字节大半由本 crate 决定：prefix 四段与断点（§8-4、§8-6）、catalog 的 Resident 行与二级披露（§8-11）、工具结果的信封与压缩（§8-7、§8-27）、上下文提醒（§8-34）与回合边界的压缩（§8-44）。replay/fork 是离线设施，其产物（分叉 Run 的入窗历史）经 prefix 组装间接入窗，自身不产生 prefix 字节。

## 18 文档同步

模块登记在 ARCHITECTURE 的模块图（`xtask modmap`）；canonical 类型的改动与 kernel-SPEC §8-23/§8-24 同一变更集；runtime 没有 api-baseline 文件，公开面即 `lib.rs` 的 `pub mod` 与根重导出。

### 8-15-1 RunHooks 的 deltas：说到一半的话往哪去

```rust
pub deltas: Option<&'a mut (dyn FnMut(&Increment) + 'a)>,   // RunHooks 的一个字段（§8-15）
```

**`None` 是必要前提，不是缺省值。** 一个增量改变不了 run 的任何判断：`Turn::call` 按 `Generating` 三臂选门——没人看（`Unwatched`）走阻塞门，有页面在读（`Watched`）走流式门，工具面要提前起跑只读调用（`Speculating`，§8-3）走推测门。citysim 与离线重放不看增量，所以增量不碰确定性。

**它不返回 `Result`。** 增量不是判断：下游任何东西都不得据它分支，而一个能拒绝的 sink 会让一个显示细节有能力弄失败一次调用。

**写进账本的那句话只从 `ModelReturn` 来。** 增量恒不参与拼装 `model_returned` 的载荷。于是「页面看到的」与「账本保存的」不可能出自对同一个回复的两次读法；流被切断表现为读取错误，永不表现为一个变短的回答。

**`Turn::call` 的 `'sink` 是显式命名的。** 调用方（`drive`）持有 sink 跨越整个 run 并把它交给每一轮；生命周期省略时，重借需要收缩 trait object 自己的生命周期，而 `&mut` 不允许。这不是风格，是这个签名必须显式的原因。

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

### 8-19 runtime::bench 目录化

| 文件 | 管什么 |
|---|---|
| `bench.rs` | `ToolBench` 与 `BenchOutcome` 的定义、装配面（`new`／`for_job`／`with_checkpoint`／`register`／`taint_mut`／`meta_of`）、`invoke` 的路由次序，以及 `kernel_error_from_storage` |
| `bench/admit.rs` | 门：`admit` 按 `Effect` 分派到 Write／Connector／Egress／Spawn／Govern 各门，`settled`／`crossed` 把一次判定翻译成 `BenchOutcome`，`scanned` 为两扇朝外的门备好密钥扫描的字节 |
| `bench/tests.rs` | 去重、门、taint、checkpoint 与注册冲突的夹具 |

### 8-20 runtime::replay 目录化

**三份文件，各答一个问题。** 崩溃恢复与链验证是两件事：链验证读的是字节与哈希，崩溃恢复读的是已验证行之间的配对关系（`tool_called` 有没有后继的 `tool_result`）。两者同处一个文件时，`replay.rs` 越过了 400 行上限。

| 文件 | 管什么 |
|---|---|
| `replay.rs` | `VerifiedLine`／`VerifiedLedger`／`Envelope`，以及 `verify_lines`／`verify_ledger_dir`／`fold_ledger_dir`／`rebuild_prefix`。它同时是子模块的父模块，声明 `mod resume;` 并 `pub use resume::{dangling_tool_calls, outcome_unknown_draft};`，因此 crate 内外的 `use` 一行未改 |
| `replay/resume.rs` | 崩溃恢复这一条规则的两次读法：`dangling_tool_calls` 认出结果未知的调用，`outcome_unknown_draft` 写下关掉它的那一行（`E_TOOL_OUTCOME_UNKNOWN`）。什么算悬空决定关帐行说什么，故同一个文件 |
| `replay/tests.rs` | 离线验证拒绝什么：更高的 `v`、无 `ig:true` 的未知 kind、漂移的 prefix 源文档、悬空的 tool_called |

### 8-21 runtime::prefix 目录化

| 文件 | 管什么 |
|---|---|
| `prefix.rs` | `SegmentSlot`／`FrozenSegment`／`SourceDoc`／`SegmentCaps`／`PrefixPlan`／`FrozenPrefix`，以及 `build_prefix`／`build_segment`／`truncation_marker`／`DOC_JOIN` 与 `system_blocks`／`segment_hashes`／`prompt_payload` |
| `prefix/tests.rs` | 冻结前缀保证什么：槽位次序、同输入同哈希、跨段去重与跳过入账、截断标记与字符边界、账本只记计划里的断点 |

### 8-22 runtime::tools::edit 目录化

| 文件 | 管什么 |
|---|---|
| `tools/edit.rs` | `EditTool` 与 `version_of`／`CREATES`：`new` 的参数模式声明、`Tool::invoke` 的判定次序（工具身份→地址→写域→版本→匹配数→落盘），创建臂 `create`，以及 `unified_diff`／`common_prefix`／`common_suffix` |
| `tools/edit/tests.rs` | edit 拒绝什么、回显什么：版本相符的落盘与 diff、陈旧版本、创建臂两种冲突、写域外、城外的本平台绝对路径（`E_GATE_DENIED`，盘上不留文件）与非法地址、缺文件的恢复话术、零次与多次匹配、错路由 |

### 8-23 runtime::run 目录化

| 文件 | 管什么 |
|---|---|
| `run.rs` | 一个 Run 的常量与状态类型（`RunPlan`／`SafePoint`／`Advance`／`RunHooks`／`Active`／`Frozen`／`Run<S>`）、`impl Run<Frozen>` 的三个读法、载荷构造 `payload`，以及驱动循环 `drive`。`drive` 带 `argument_count` 豁免，故留在原路径 |
| `run/lifecycle.rs` | 一个活着的 Run 在账本上做的三件事：`dispatch` 的调度对（job pin＋run_started）、`advance` 的一回合（四个安全点、波前检查点、报告前推入窗），以及唯一出口 `freeze`（handoff_written＋run_frozen）；连同只有 `advance` 用得上的 `fold_steer`。调度对与收尾对的字节由 `run/charter.rs` 写 |
| `run/charter.rs` | 一个 run 的开篇两行与收尾两行，模型 run 与 harness run 同一个作者（§8-52） |
| `run/harness.rs` | 回合由 harness 自己走完的 run：它在账本上写的每一行，与回答冻成哪一种 `Completion`（§8-52） |

**`impl Run<Active>` 保持为一整块，不按 dispatch／advance／freeze 三分**：`cargo public-api` 按 impl 块计数，三分会让公开面输出多出四行而规范路径不变。

### 8-24 runtime::digest 目录化

| 文件 | 管什么 |
|---|---|
| `digest.rs` | 摘要管线本身：`StructureNode`／`Digest`／`Breaker`／`BreakerVerdict`／`DigestOutcome`，纯函数 `structure_of` 与 `close_deeper`，以及唯一入口 `digest_once`。`digest_once` 带 `argument_count` 豁免，故留在原路径 |
| `digest/tests.rs` | 摘要对读者的四个承诺：标题树跳过代码围栏、模型写下的散文永远 suspect、同一内容哈希一生只摘要一次、熔断器计次开合 |

### 8-25 runtime::sandbox 目录化

**缝与唯一一个真适配器分家**：缝留在父文件，两百余行 wasmtime 专属代码在适配器自己的文件里。

| 文件 | 管什么 |
|---|---|
| `sandbox.rs` | 执行边界本身：`Fuel`／`Mount`／`SandboxJob`／`SandboxOutcome`／`SandboxExit`／`Sandbox` 缝，缺席判词 `AbsentSandbox`，两个替身 `EchoSandbox`／`FaultSandbox`，以及 feature `conformance` 下的 `assert_sandbox_conformance`。它声明 `#[cfg(feature = "wasm")] mod engine;` 并原样保留 `pub use engine::WasmtimeSandbox;` |
| `sandbox/engine.rs` | 唯一会真跑 guest 的适配器：`WasmtimeSandbox`（引擎配置、预开目录、燃料预算、stdio 管道）、host 侧拒词构造 `host_error`，以及把引擎的收场判成 `SandboxExit` 的 `classify` |
| `sandbox/tests.rs` | 两个替身对调用方的承诺：直通替身回声 stdin 并记下 job、故障替身按序发脚本且发完即止 |

### 8-26 runtime::tools::exec 目录化

**「发生了什么」与「怎么写下来」分家。** 三条臂判定发生了什么，结果载荷的键名（`arm`／`stdout`／`stderr`／`exit_code`／`outcome`／`handle`／`what`／`detail`／`background`／`env`）是另一件事：它们必须一处定义，否则一条臂可以把结果拼得跟邻居不一样。两者同处一个文件时 `exec.rs` 越过了 400 行上限。`ExecTool::new` 的 7 参豁免键 `crates/runtime/src/tools/exec.rs::new` 仍指着它原来的文件。

| 文件 | 管什么 |
|---|---|
| `tools/exec.rs` | 三臂本身：`ExecTool`（`new` 与 `run_program`／`run_python`／`run_shell`／`through_the_backlog`／`inherited_environment`）、环境白名单 `ENV_ALLOWLIST`、臂解析 `parse_arm`，以及 `impl Tool for ExecTool` 的路由。它声明 `mod outcome;` 并 `use outcome::{backgrounded, exceptional, settled, with_backlog, with_environment};` |
| `tools/exec/outcome.rs` | 这把工具能给出的每一种回答的形状：`settled`（原名 `outcome`，模块名占了这个词故改用动词过去式）、`backgrounded`、`exceptional` 三种载荷，以及 `with_backlog`／`with_environment` 两条尾巴。全部 `pub(super)`，不出 `tools::exec` |
| `tools/exec/tests.rs` | 每条臂对调用方的承诺：缺件时点名替代方案而拒绝、python 臂在沙盒里跑并报自己的退出码、燃料耗尽与 trap 原样抵达、无法识别的臂只拒不猜、program 臂真起子进程且环境被洗|

### 8-27 runtime::sieve（形状 1 判定＋形状 6 数据面；**本节是压缩器的唯一权威**）

> 读这一节即可实现，不需要再查别处、不需要再做任何参数选择。方法论取自 Hypabolic/Hypa 的 ADR-0002 与其 `Compression/` 实现；四处刻意背离记在 §8-27-7。

#### 8-27-1 它是什么，以及为什么不是 LLM

`sieve` 按**产生结果的命令**决定留下什么。它与 `compaction` 相邻而不重叠：`compaction` 看文本形状（Prose／Code／Diff／Log／Structured／Table／Markup／Unknown），`sieve` 看命令身份（`cargo build` 与 `git status` 的噪声形状完全不同，而两者都是 Log）。

不用模型做压缩，理由是被架构强制的而非偏好：确定性（ARCHITECTURE 的确定性一节）要求同一颗种子重放出逐字节相同的对话，一个会思考的压缩器会让重放不可能。它同时省掉一次调用的钱与延迟。

**顾问不是这条链上的模型。** 它在 `pipeline::package` 之外先跑，判断作为 `PackContext.adviser` 进来，而且答案本身就是账本上的一条记录（`adviser_answered` 或 `adviser_fell_back`）——重放读到的是那条记录，不是再问一次。所以「同一颗种子重放出逐字节相同的对话」仍然成立，而 `sieve` 自己一个模型也不调：它只有七条固定阶段。

#### 8-27-2 位置与法则

管线原法则「offload 恒先于 truncation」扩写为：

```
tee（原文钉进 CAS ＋ 实体化 rest 文件）
  → sieve（命令感知）
    → offload / truncation（既有三臂）
```

**tee 恒在最前**：这是 `offload` 既有的 store-before-cut 不变量上提一层。因为原文一定先落盘，sieve 才敢压得比 Hypa 狠——Hypa 那条「压缩率异常高时拒绝」的护栏存在，是因为它不能保证全文带内可恢复；本城可以。

#### 8-27-3 作用范围

**只作用于 `exec` 结果。** 命令感知的东西对没有命令的工具无可感知：`read` 结果、模型输出、MCP 结果各自走既有的 `compaction`／`redact` 路径，本节不动它们。MCP 结果不过 sieve，但过同一个 `package` 的落盘一步，见 8-27-10。

#### 8-27-4 七条不变量

1. 输出恒不长于输入。
2. 输入非空时输出非空。
3. 任何裁剪之前，原文已钉入 CAS。
4. 每个被筛过的结果都携带 rest 文件路径与 CAS locator。**压缩是强制的；可恢复也是强制的**——没有落 tee 的压缩不许发生。
5. 切口落在字符边界上。
6. **这条路上不用正则表达式**（判「重要」的四类模式全部手写线性扫描，见 §8-27-6）。
7. 账本记的是**模型看到的字节**＋原文 locator。重放复现模型看到的东西，不是命令打印的东西。

#### 8-27-5 阶段顺序与参数（全部已定，实现者不另选）

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

过滤表住 `<city>/.sprawling/FILTERS.toml`，楼级可覆盖 `<building>/.sprawling/FILTERS.toml`，走既有三层阶梯且**整值解析而非逐字段合并**（直接沿用 kernel-SPEC §8-22 给 `[sandbox]` 定的口径①，一条规则一个权威）。**信任问题不存在**：过滤表住保留区，没有任何写域够得着它。

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
4. **无正则。** Hypa 的分类器整个是正则；本城这条路上不用模式引擎。手写扫描约 40 行，更快且无回溯风险。

不抄的三样：它的十个 compiled reducer 清单（维护跑步机）、`Microsoft.ML.Tokenizers`（依赖加谎言）、SQLite 与 `hypa trust`（账本＋CAS＋保留区规则已经更强）。

#### 8-27-8 验收

每张内建过滤器一组 golden；一条性质「输出 ≤ 输入」；一条性质「输入非空则输出非空」；一个 citysim 场景——同一颗种子、同一张过滤表，重放出逐字节相同的窗口。

#### 8-27-9 接口与文件切分

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
    Passed { text: String, reason: PassReason, account: Option<SieveAccount> },
                                  // 地板以下／全部阶段被拒：原文一字不动；跑过的阶段随 account 出去
    Cut(SieveRecord),
}
pub enum PassReason { BelowFloor, NothingShrank }
pub struct SieveRecord { pub text: String, pub original: Locator, pub rest_path: String, pub filter: String,
                         pub lines_in: u64, pub lines_out: u64, pub bytes_in: u64, pub bytes_out: u64,
                         pub stages: Vec<StageReport> }
impl SieveRecord { pub fn offloaded(&self) -> ResultOffloaded; }
pub struct ResultOffloaded { pub original: Locator, pub len: u64, pub substitute_len: u64,
                             pub rest_path: String,
                             #[serde(flatten)] pub sieve: Option<SieveAccount> }
impl ResultOffloaded { pub fn payload(&self) -> Result<Payload, AxError>; }   // result_offloaded 的唯一写方
pub struct SieveAccount { pub filter: String, pub lines_in: u64, pub lines_out: u64,
                          pub stages: Vec<StageReport> }
pub struct StageReport { pub stage: Stage, #[serde(flatten)] pub outcome: StageOutcome }
pub enum Stage { StripAnsi, FoldBlank, DedupTemplate, DiffPrevious, Filter, CutLongLine, Truncate }
pub enum StageOutcome { Applied { bytes_before: u64, bytes_after: u64 }, Noop, Rejected { grew_to: u64 }, Unavailable { reason: String } }
pub fn sieve(input: SieveInput<'_>, table: &FilterTable, site: &mut OffloadSite<'_>, history: &mut SieveHistory)
    -> Result<Sieved, AxError>;
// tee 走 offload::tee（pub(crate)；offload() 自身也改经它，store-before-cut 只有一处）；history 无论 Cut／Passed 都记本次原文

- **账目不为任何一条臂而丢。** `Cut` 携 `SieveRecord.stages`；`Passed` 携 `account`——NothingShrank 时七条阶段全在，BelowFloor 时 `None`（没有阶段跑过，`reason` 就是全部账目）。`StageOutcome` 四变体（`Applied`／`Noop`／`Rejected`／`Unavailable`）是每个阶段唯一的答案形状。**顾问不是第八个 stage**：问／答／回落有自己的事件族（`adviser_asked`／`adviser_answered`／`adviser_fell_back`），在这里再记一份就是同一事实两个家；「顾问就是第八个 stage」是 E-2 核验更正前的措辞。
- **pass 的账目去处**：经 sieve 但未被裁的结果若随后走普通 offload 离窗，`ResultOffloaded.sieve` 携这名 account；留在窗口内的 pass 不写账，因为没有任何东西离窗。

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
| `sieve.rs` | 1 判定 | 阶段定序、逐级「不变长才接受」的判定（`Draft`：行与账同行）、页脚 |
| `sieve/key.rs` | 2 值 | `CommandKey`：arm／program／args，Ord |
| `sieve/record.rs` | 2 值 | `Sieved`／`SieveRecord`／`Stage`／`StageOutcome`／`StageReport`／`SieveAccount`／`ResultOffloaded`——`result_offloaded` 的唯一形状，筛与直接搬运两条路都经它 |
| `sieve/filter.rs` | 6 数据 | `Filter`／`FilterTable`：TOML 形、三张内建、三层整值覆盖、命中规则 |
| `sieve/scan.rs` | 1 判定 | 重要行五级优先级的四类手写扫描；受保护片段（URL、设备码、`secret:` 引用、`路径:行:列`、退出码行）的判定 |
| `sieve/stages.rs` | 1 判定 | 去 ANSI、空行折叠、模板去重、长行截断、head/tail/中段截断 |
| `sieve/diff.rs` | 1 判定 | `SieveHistory` 与跨调用差分 |
| `sieve/tests.rs` | — | 三张内建过滤器的 golden、两条 proptest 性质、阶段账 |

**三处读法**（§8-27 未写明处，按此实现；改口径先改这里）：

1. **受保护片段的「不得触碰」**读作：含受保护片段的行不进模板去重、不被长行截断——这两级会**改写**一行；在第 7 级截断里它算最低一级重要行（排在 note/help 之后），与其他重要行一起按序取前 60。把它读成「恒不丢」会让一份三百条 URL 的清单压不动，与不变量 1 的目的相悖。跨调用差分**不豁免**它：把上一次逐字相同的行计入「未变 N 行」不改写任何一行，那行在 rest 文件里原样在，模型上一次也已读过；豁免它会让每个 rustc 诊断块被 `-->` 行切成折不动的短段，差分在它为之而存在的 cargo 场景上恒为 Noop。空行同理计入。
   第 3 级模板去重另豁免过滤表 keep 命中的行及其分组（否则 `  |` 这样的诊断沟槽行跨块折叠，`10 |     let x0 = 1; [×6 similar lines]` 说的是并不相同的六行）。
2. **过滤表的 `head`/`tail`** 是第 7 级 HEAD/TAIL 的逐表覆盖，不是第二次截断；`keep_*` 命中的行进第 7 级中段候选，优先级与 §8-27-6 的 warning 同级。这样截断只有一处权威。
3. **generic 的 `on_empty`** 缺省为 `"(no output kept), exit {code}"`；`{code}` 在无退出码（sandbox trap／fuel 耗尽）时写 `none`。这是不变量 2 在通用路径上的执行体。

**三个 reducer 与表的关系**：cargo 与 git 是两个以代码构造的 `Filter` 值，rustc 诊断分组是 cargo 那张的 `keep_contains` 命中行向后扩到空行为止（同一诊断块整体进中段候选）；generic 是空谓词的 `Filter`。表内条目与内建同形，故楼级表可以整张换掉 cargo 的裁法而不动代码。

**citysim 侧**：`Scenario` 增 `sieve: Option<SieveWorld>`（CAS＋environment＋过滤表＋本 run 的 history）；有它时执行器把名为 `exec` 的工具结果经 `package_exec` 走带 `SieveRequest` 的 `package`，模型看到 `{content, exit_code, sieve:[载荷]}`。`tools/citysim/tests/sieve.rs`：同一目录同一表跑两遍账本逐字节相同；第二次同命令只出新错误与 `[unchanged: N lines…]`。

#### 8-27-10 连接器结果：`runtime::pipeline::connector`

形状：decision（与 `pipeline::exec` 同形）。一台 MCP server 的一次回答 `{ content: [块…], … }` 进窗口之前，文本块合起来量一次长度：

```rust
pub const CONNECTOR_CAP_BYTES: u64 = 16_384;
pub fn package_connector(outcome: ToolOutcome, offload: OffloadSite<'_>) -> Result<ToolOutcome, AxError>;
```

- **声音块进 CAS，窗口里只留引用**：`type: "audio"` 的块与图片块在同一步、同一种位置上处理。`mimeType` 以 `audio/` 开头、`data` 解得开 base64、字节不为空，三条都成立时，字节以 `put_for` 存进 CAS，块换成一个文本块 `[recording attached: <mimeType>, <字节数> bytes, <locator>]`；任一条不成立，换成 `[recording left out: <原因>]`。它不进 `ToolOutcome.attachments`：那里是模型看得见的图片，而模型听不见声音；要用这段录音的是一件工具（例如 `transcribe`），它要的是 locator（§12.15）。
- **图片块进 CAS，窗口里只留引用**：`type: "image"` 的块在文本那一步之后、在它原来的位置上处理，所以替它的那行字不并入被存下分窗的文本，模型不用翻页就读得到。`mimeType` 是 `image/png`、`data` 解得开 base64、字节不超过 `IMAGE_MAX_BYTES`、PNG 头读得出宽高，四条都成立时，字节以 `put_for` 存进 CAS，`ToolOutcome.attachments` 多一张 `ImageRef`（与浏览器截图同一种形状），块换成一个文本块 `[picture attached: image/png <宽>x<高>, <locator>]`。任一条不成立，块换成一个说明为什么没带图的文本块（`[picture left out: <原因>]`）。base64 恒不进窗口，也恒不进账本。其余非文本块（资源）照旧按原顺序留在其后。
- 回答没有 `content` 数组，或既没有图片块也没有声音块且文本合计不超过 `CONNECTOR_CAP_BYTES`：原样返回，一个字节不动。
- 超过：全部文本块按原顺序以换行连成一份，交 `package`（`sieve: None`，带落盘处）；`content` 换成**一个**文本块，装 `package` 给出的替身（开头一段加 `read` 可分窗读的路径），非文本块（图片等）按原顺序留在其后；`package` 记下的 `ResultOffloaded` 放进结果的 `offload` 字段，与 `exec` 的 `sieve` 字段同一种账。
- 替身是什么由 `package` 一处决定，本模块不另判：`Markup`（Markdown 一类）按 8-7 走节标题骨架而不入 store，故一份转换出来的长 Markdown 进窗口的是骨架，没有可翻的路径，`offload` 为空表；其余文本按 `OFFLOAD_MIN_BYTES` 入 store。
- 失败只有 `package` 自己的失败（`E_INVALID_ARGS`，原样上抛）。

**上限与 `exec` 同值、各有其名**：两者今天取同一个数，是因为窗口里一件工具答案的代价与来源无关；分开命名，是因为改其中一个不该悄悄改另一个。**决定**：交 `package` 而不是在这里另写一套截法——截多少、存不存由它一处决定，连接器答案与 `exec` 答案在窗口里守同一条规则；不包装时一次回答可以把整整 `MESSAGE_CEILING`（8 MiB）送进窗口。备选「让 `agent_protocols::McpTool` 自己截」被否：协议层没有 CAS 也没有 room，落盘处只有装配层有。调用点只有一个：`bin::assembly` 的 `Placing` 对 effect 为 `Connector` 的调用（首答与重放同样）调它。图片在这一步进 CAS，理由与截长文本相同：协议层没有 CAS 也没有 room。只量 PNG 的理由与浏览器相同：别的格式要第二个解码器才量得出边长，一个量错的边长比没有更糟；不是 PNG 的图片以一句话告诉模型改要 png。

### 8-28 runtime::backlog（形状 4 适配器＋形状 6 数据面）

**问题**：`turn.rs` 首段写明中断只在相位边界被消费，而 `Command::output()` 阻塞在系统调用里、不在边界上，所以一条挂死的命令若不入表，`halt` 停不住；没有回合上限与花销上限的城里，这会是一处无界失效。

**设计**：一张表，成员是后台 exec 子进程与 `delegate` 子 run。

- **`exec` 恒经此表**，不设 `background` 参数——两条路径就是两个权威，而有洞的那条永远是没人想起的那条。
- 先阻塞等一个短窗口（**10 s**），短命令因而感觉上仍是同步的；超时则返回一个句柄并继续在后台跑，结果落**起它的那个 run 的下一次 `exec` 结果的尾部**，恒不落别的 run。给 agent 的说法是「长任务不要干等：起了它，做别的，结果到了会接在后面的工具结果尾部」。
- `halt {scope}` 遍历该表并终止其成员；工作线程不再进入阻塞系统调用，halt 因而真的停得住。
- `status` 报告表中属于本 run 的成员：几条在跑、各自跑了多久。

**红测试**：一条不会结束的命令起后，`halt` 使其进程终止且 run 回到边界；十秒内结束的命令不产生句柄；后台结果确实出现在起它的那个 run 的下一次工具结果的尾部，而同城另一个 run 随后的 `exec` 结果里没有它；一次调用的 verdict 恒只答它等到的那扇窗——窗口外落定的命令不回写本次结果，收割是下一次调用的事。

#### 8-28-1 接口与三处已定的实现选择

```rust
pub struct BacklogId(u64);                      // Display；一次 serve 内唯一
pub enum Started {                              // 短窗口的穷尽结果，恒不是 bool
    Settled { exit: Exit, stdout: String, stderr: String },
    Backgrounded { id: BacklogId, what: String },
}
pub struct Standing { pub id: BacklogId, pub scope: Address, pub what: String }
pub struct Finished { pub id: BacklogId, pub what: String, pub exit: Exit,
                      pub stdout: String, pub stderr: String }

// backlog::waiting —— 等多久，与「怎么结束的」两件事的唯一住处
pub struct PollBudget { /* polls、interval_ms 私有 */ }
impl PollBudget { pub const DEFAULT: PollBudget; pub fn new(polls: u32, interval_ms: u64) -> PollBudget; }
pub enum Exit { Ended { code: i32 }, Signalled, Unknown { why: Unseen } }
pub enum Unseen { LeftTheTable, WaitRefused }   // 各带一句给模型看的话

#[derive(Clone, Default)]
pub struct Backlog(/* Arc<Mutex<Table>>，表内是 BTreeMap */);
impl Backlog {
    pub fn with_window(window: PollBudget) -> Backlog;   // `new()` 即 PollBudget::DEFAULT
    pub fn run(&self, owner: RunId, scope: &Address, what: String, command: Command) -> Result<Started, AxError>;
    pub fn halt(&self, scope: Option<&Address>) -> Result<usize, AxError>;   // None＝整城
    pub fn harvest(&self, owner: RunId) -> Result<Vec<Finished>, AxError>;   // 只收 owner 起的
    pub fn release(&self, owner: RunId) -> usize;                            // owner 结束：终止它留下的命令，它们不再欠任何人
    pub fn standing(&self, scope: &Address) -> Result<Vec<Standing>, AxError>;
}
```

三处实现选择，各有理由：

1. **短窗口靠固定次数的轮询走完，不采样时钟，且窗口由调用方注入。** 默认 10 s ＝ 500 次 × 20 ms，住 `PollBudget::DEFAULT`；表持一份 `PollBudget`，`Backlog::with_window` 换掉它。理由是本仓那条「时间是入参，唯一采样点是 `bin::assembly`」——一个为了等十秒而调 `Instant::now()` 的模块会把那条规则打穿，而计数不需要时钟。**注入不是为了可配置**：`Settled` 与 `Backgrounded` 两种载荷形状不同，一个不能选窗口的调用方要观察后一种就得真等十秒，于是套件要么慢十秒、要么不判它交付的那个形状。

1b. **退出码是穷尽枚举，不是一个整数。** `Exit::Ended { code }`／`Signalled`／`Unknown { why }` 三臂：一个整数分不出「程序返回了负一」「被信号杀死」「本城没问出来」，而读结果的模型要分得出。`exec` 结果里 `exit_code` 键**只在 `Ended` 时出现**，另两臂写 `outcome`（`signalled`／`unknown`）与一句 `detail`；键名仍只由 `tools/exec/outcome.rs` 拼（§8-26）。
2. **子进程的输出写文件，不走管道。** 管道缓冲区填满会让后台子进程停在写系统调用上，于是「后台」变成「挂死」——那正是本节要修的那个洞的另一种写法。文件住 `std::env::temp_dir()` 下按 `BacklogId` 命名的一层目录，收割时读完即删。
3. **表是一份共享句柄（`Clone` 的 `Arc<Mutex<_>>`）。** 装配层持一份，每个 `ExecTool` 持一份克隆，于是 `halt` 够得着 `exec` 起的东西而不必让 `halt` 认识 `exec`。表内是 `BTreeMap`，遍历序恒定。
4. **后台命令的结局只欠起它的那个 run（`owner: RunId`）。** 表是全城一张（第 3 条），所以「谁收割」必须由表按成员记下的 owner 判，而不是由「谁先调 `exec`」判：不带 owner 的 `harvest` 会把一栋楼（包括 confidential 楼）里一条命令的 stdout／stderr 原文交给城里任何一个随后调 `exec` 的 run，进它的模型与账本，而起它的 run 反倒收不到。一条命令的归属是穷尽枚举 `Claim` 而不是 `bool`：`Window(RunId)`（短窗口里，归正在轮询它的那次调用）、`Run(RunId)`（已转后台，只交给这个 run 的 `harvest`）、`Nobody`（它的 run 已结束，结局不交给任何人）。`ExecTool` 在 drop 时调 `release(run)`：每个 run 的工具台在它冻结后被丢弃，于是 drop 就是「这个 run 再也收不到」的那一刻。`release` 把这个 run 的命令改记为 `Nobody` **并终止它们**（与 `halt` 同一个 `kill`）；此后任何一次 `harvest` 都会把已结束的 `Nobody` 成员不读即删（进程句柄与临时目录因此有界），输出不交给调用者。`release` 不会失败：它只把认领降为 `Nobody`、只终止进程，这两件事在表的任何状态下都成立，所以一张被死线程锁住的表它也照做（取 `PoisonError::into_inner`）；表的其余调用照旧答 `E_STORAGE_FATAL`。它的调用方是 drop，没有人可以转交失败，一个会失败的 `release` 只能被丢弃，而被丢弃的那一次正是命令被留着为一个不存在的 run 跑下去的那一次。**被否决的做法**：按 `scope`（楼或房间地址）收割——同一房间里前后两个 run 地址相同，于是后一个 run 仍会收到前一个的输出，而 confidential 的界是按 run 的模型与账本行划的，不是按地址。run 结束即终止它留下的后台命令（§12.3）。

#### 8-28-2 第二类成员：委派下去的 run

**问题**：表里若只有子进程，`halt {scope}` 遍历表时看不见任何 run——一轮委派下去的活在它的 scope 被停摆之后照样跑到冻结。所以委派下去的 run 是表的第二类成员。

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
- **每个 call 之前问一次**：一次工具波是模型一条回复里要的 N 件副作用，不是一件。`SafePoint::BeforeToolCall { turn, call }` 是第五个安全点，`Turn::<ToolWave>::execute` 在每条 call **入账之前**问 `still_going(call) -> Interrupt`，答案走相位边界那一个 `consume_boundary`：Cancel 写下同一条 `cancel_received` 并终止回合，于是停城不必等整波跑完，也不会为没做的活留下 `tool_called`；Steer 写下 `steer_received`，这条 call 照做。两条 call 之间能立刻执行的指令仍只有「停」，改道的文字属于执行器的 `Window`，由执行器在回答之前折进去、在下一次组装时交给模型。答案取整个 `Interrupt` 而不是只说停不停的 `NextCall`，是因为窄答案让波内到达的 steer 只进了窗口、没进账本：模型读到了一段账本上没有来历的文字。
- **只有委派下去的 run 入表**。根 run 由 `Cancel` 结束，那是另一个动词（glossary「Halt」行）；把根 run 也入表会让 `halt` 与 `Cancel` 变成同一件事。
- **`harvest` 与短窗口都跳过 run 成员**：它们收的是进程的退出码，run 的结局在账本上。
- **`status` 的那一半**：末行 `backlog:` 追加在冻结序末尾（追加规则同 `neighbours`），报 `standing(addr)` 里属于本 run 地址的成员——`bg-3 cargo build (command)` 逐条分号相连，没有则 `none`。**不报跑了多久**：本表不采样时钟（§8-28-1 第 1 条），一个为了报时长而采样的 status 会是第二个采样点。
- **接线在 `accounting::worker::dispatching::running`**：`at.parent.is_some()` 时先 `enrol_run`，drive 结束后 `leave`；`Driving` 携 `member: Option<BacklogId>`，中断钩子在人的打断与 steer 之前先问 `stopping`。`halt` 在记账线程上被处理而子 run 在驾驶池上跑（sprawling-SPEC §8-42）；红测试用 `attach_interrupts` 在子 run 的安全点上调 `halt`。

**红测试**：一轮委派下去的 run 起后，其 scope 被 `halt`，子 run 以 `cancelled` 冻结且没有再叫过模型；`status` 的第十四行报本 run 起的后台命令。

#### 8-28-3 exec 输出的实时流：读增量、装配点注入、线上帧、服务端与页面两段缓冲

**现状**：一个 `exec` 调用的 stdout／stderr 只在调用结束时随工具结果进账本，页面（client 的监视器）在那之前只看得到「运行中」。第 2 条让输出写进 scratch 下的 `out`／`err` 两个文件，所以实时流不需要改子进程怎么写，只需要有人在它还在写时读这两个文件的增量。

**读增量——`runtime::backlog::tail`（shape：adapter）**：

```rust
pub enum Stream { Out, Err }                          // 恒不是 bool
pub struct Chunk { pub run: RunId, pub member: BacklogId, pub stream: Stream, pub bytes: Vec<u8> }
#[derive(Clone)] pub struct Sink(/* Arc<dyn Fn(Chunk) + Send + Sync> */);
impl Sink { pub fn new(deliver: impl Fn(Chunk) + Send + Sync + 'static) -> Sink; }
impl Backlog { pub fn with_sink(self, sink: Sink) -> Backlog; }
impl PollBudget { pub(crate) fn read_per_poll(self) -> usize; } // interval_ms × READ_BYTES_PER_MS
```

- 没有失败返回：读增量是可丢弃的预览，账本里的工具结果才是这段输出的权威；一次读失败只让这一拍少送一块，下一拍从同一偏移再读，偏移只在真的读到字节后前移。
- `READ_BYTES_PER_MS = 64`（每秒 64 KiB，一个人在页面上读得过来的上界的数倍），一拍的上界由它乘间隔得出，stdout 与 stderr 各得一半，所以刷屏的 stdout 饿不死 stderr。
- 决定：sink 是一个闭包而不是 trait——今天只有一个生产装配者（服务端扇出）与一个测试收集者，两者都只要「交出一块」这一个动作。

- 窗口里的读停在交给后台那一刻，偏移随成员进表；此后本 run 的每次 `harvest` 对它自己的、仍在跑的后台命令从同一偏移接着读。块在放开表锁之后才交给 sink，所以 sink 慢不会让表上其他调用等它。

**装配点注入**：一座被端上来的城（`RunWorker::serve`）把一个 `Sink` 装到自己的 `Backlog` 上，sink 把每块译成 `wire::LiveOutput`（wire-SPEC §8-48）交给 `Serving::outputs`，那里送进第四条广播通道。没有被端上来的城（citysim、replay、一次一条命令的 worker）不装 sink，所以一个字节都不读。

**页面缓冲**：`client/src/core/live_output.ts` 为每个 run 留一段 `Tail`（stdout、stderr 与丢掉的行数），每条流只留最新的 `LIVE_LINES = 400` 行；`tool_result` 一到就丢掉这个 run 的那段。监视器的终端记录把它画在仍在跑的那一条下面，丢掉的行数照 `mon_lines_cut` 说出来。

**服务端缓冲**：`sprawling::serving::output_ring`（sprawling-SPEC §8-90）为每个 run 按字节留最新的一段，后来打开页面的会话在 `Welcome` 之后先经 `ServeConfig::outputs_so_far`（wire-SPEC §8-48）拿到它，再接实时帧；这个 run 的 `tool_result` 落账时清空。

**设计**（四段共同遵守的规则）：

- **读的地方是短窗口的轮询，不另起线程**。`Backlog::run` 每一拍轮询在 `settle` 之后按上次的偏移读两个文件新增的字节，交给调用方注入的一个 sink；窗口外交给后台的命令由 `harvest` 那一拍同样读增量。sink 缺席（citysim、replay、没人看的城）时一个字节都不读，行为与今天相同。
- **每拍读的字节有上界**，按 `PollBudget` 的间隔推出而不写死：一拍最多读 `PollBudget::read_per_poll()` 字节，即 `interval_ms × READ_BYTES_PER_MS`，间隔越长每拍读得越多、喂给页面的速率不变；读不完的留到下一拍，所以一个刷屏的子进程让页面落后，而不让轮询变慢。
- **服务端每个 run 一个有界环形缓冲**，按字节计上界，满了丢最旧的整块；后来打开 run 页的会话先拿到缓冲里的内容，再接实时帧。丢了多少不上线：`LiveOutput` 没有这一栏，而加一栏要让 `WIRE_V` 再加一，换来的只是预览里的一个数——调用落账时整段输出本来就会到。缓冲在这次调用的 `tool_returned` 落账时清空，因为那时账本里的结果是这段输出唯一的权威。
- **线上是一种新的 `ServerFrame`**，与 `Delta` 同一条规则：可丢弃，不带账本序号，调用的结果落账时页面扔掉它，两者不一致时账本赢。这一帧让 `WIRE_V` 加一并重新生成 `client/src/wire.ts`。
- **页面也是有界环形缓冲**，按行计，监视器的终端记录画它；溢出的行数照 `mon_lines_cut` 的样子说出来。

### 8-29 runtime::tools::read 区间读（字节预算）

**两道天花板，紧的那道说了算。** 512 行界定一次作答携带多少**结构**；`INTERVAL_CAP_BYTES`（64 KiB）界定它花掉窗口的多少**字节**——本仓源码一行约四十字节，而一个生成物可以整份压在一行里，故行上限单独不成其为界。

**字节上限切在行边界上，且落在 `Interval::cut` 内部而不在下游。** 理由只有一条：`next_offset` 是调用方续读的唯一凭据，若字节在本函数报出 `next_offset` 之后才被裁掉，那个数就会指过一批没人交付的行，而这个缺口是无声的。首行恒交付，无论它多长——一次返回空的作答会把收到的 offset 原样递回去，调用方于是永远问同一个问题。

`search` 共用同一个上限，接在它已有的 `truncated` 上：`MATCH_CAP = 64` 界定**几条**答案上路，不界定它们**多大**。**首个命中同样受它约束**：一条命中的上下文块自己就超过上限时，由 `elision::splice` 从尾部切进上限、带 `[truncated: N bytes]`，然后停走并报 `truncated`。`read` 的首行恒交付而 `search` 不，是因为 `search` 报的是行号、续读走 `read`，不存在一次空答让调用方原地打转的那个缺口；一整行压缩产物原样塞进一条命中，就是这道上限要拦的那种输入。

**取 64 KiB 的推导与实测见 kernel-SPEC §8-8 该常量的 doc**；一句话是：上下文提醒按 25%／65% 排成梯子，没有哪一步可以整级跨过，故单条结果 < 35% 窗口，而 64 KiB 在最坏字节-token 比率下是 128K 窗口的 20.5%。

`ReadTool` 增 `offset`（0 基行号，缺省 0）与 `limit`（缺省与上限**均为 512 行**）。被截断时结果携 `total_lines` 与 `next_offset`，所以「我拿到的是不是全部」不需要猜。`bytes` 字段的语义不变，仍是本次返回文本的长度。

理由：整读一份千行级的 SPEC 或数十 KB 的 ARCHITECTURE，要么吃掉整个窗口，要么被管线从中间剪掉，而剪掉的往往正是要改的那一段。512 是选定值。

#### 8-29-1 参数与结果（实现照此，不另选）

```rust
// args：{path, offset?: u64, limit?: u64}
// offset 缺省 0；limit 缺省 512，大于 512 者**夹到 512** 而非拒绝——
// 模型多要一点不该赔掉一个回合，它拿到的截断字段会把真相说清楚。
// offset／limit 非整数或为负＝E_INVALID_ARGS；limit==0 同。
const LINE_CAP: u16 = 512;
```

**上界由类型给，不由换算的失败支给。** `limit` 以 `u16` 携带：大于 `u16::MAX` 的请求与大于 512 的请求是同一件事，都夹到 512，而 `usize::from(u16)` 没有失败支。`offset` 以请求里的 `u64` 携带，在切行处与总行数比较：一个 `usize` 装不下的 offset 越过了内存装得下的任何文本的末尾，于是落进下表「越过末尾」那一行，而不是被换成一个碰巧很大的数。`total_lines`、`next_offset`、`bytes` 都以 `usize` 直接成为 JSON 数。

切行按 `split_inclusive('\n')`：每行连它自己的换行符一起数、一起还，所以 `offset=0, limit>=total` 的返回与整读**逐字节相同**，`bytes` 字段的旧语义因而不动。

| 情形 | `text` | `total_lines` | `next_offset` |
|---|---|---|---|
| `offset + 返回行数 < total_lines`（截断） | 该区间 | 有 | 有，＝`offset + 返回行数` |
| 读到文件末尾 | 该区间 | 无 | 无 |
| `offset >= total_lines`（越过末尾） | 空串 | 有 | 无——后面没有东西了，给一个 `next_offset` 就是请模型原地打转 |

目录路径与 catalog 条目走同一条切行路：一个条目短到永不触顶，而两条路就是两个权威。

#### 8-29-2 保留区判定移出

`resolve` 里「`Address::parse` 后判 `is_reserved`」这一段移进 `runtime::tools::chosen_path`（§8-30-1），`read` 与新的 `search` 同调它。理由是一条硬约束：模型选的路径能不能到保留区，全城只允许有一个答案与一组测试。

#### 8-29-3 一件藏品是一个目录：`<名>/<相对路径>`（`runtime::tools::read::package`）

```rust
// read::package
pub(super) fn open_in_package(catalog: &Catalog, city_root: &Path, asked: &str) -> Option<Result<Found, AxError>>;
pub(super) fn open_document(city_root: &Path, shelved: &Address, asked: &str) -> Result<Found, AxError>;  // 单文档 skill
```

- **阅览室准入的是整个包**：catalog 条目带着包目录（`CatalogEntry::package`，由 city 的扫描给出，city-SPEC §8-8）时它是一个包，不从落点的写法去猜——一份恰好叫 `SKILL.md` 的单文档会让整个 section 被当成包；`<名>/<相对路径>` 打开包目录下的那个文件。准入是人写阅览室时做的，所以包内文件与 `SKILL.md` 一样不经读界与保留区判定——它们住在同一个被准入的目录里。
- **名字在前、路径在后，名字先查 catalog**：与整名命中同一条理由（§8-29 起首），一个恰好同名的城内目录遮不住它。首段不是 catalog 里的包（没有这个名字，或它是单份文档）即返回 `None`，交回普通路径那条路。
- **相对路径逐段判形，不做规范化**：空段、`.`、`..`、带反斜杠或冒号的段一律 `E_INVALID_ARGS`，恢复语说出「包内相对路径，只用普通段」。规范化会把一条爬出包的路径「修」成另一条，而拒绝让写错的那一方看见自己写了什么。
- **链接按落点判，不出包目录**：拼出的路径解开链接后的真实位置，必须落在「规范化的城根 + 书架上写的包路径」之下，否则 `E_GATE_DENIED`，恢复语说出「指包里的文件本身，而不是包里链接背后的东西」。以书架上的写法而非包目录的真实位置为准，所以包目录本身是链接时同样拒绝。免于读界的理由只覆盖被准入的那个目录；链接背后的文件没有被准入，而书架上的文件可以由人手或 `exec` 写进来，安装时的预检挡不住它们。文件不存在时同样按落点判：真实位置由 `chosen_path::real_location`（§8-30-1）求出，不存在的尾段不可能是链接，所以包里一条链接背后的缺失文件落在链接目标之下，出包即 `E_GATE_DENIED`，不交给读取去列链接背后的目录。
- **单文档 skill 同样不走链接**：`open_document` 解开链接后的真实位置必须恰是「规范化的城根 + 书架上写的文档地址」，否则 `E_GATE_DENIED`，恢复语说出「请人把文档本身放上书架」。理由与包内文件相同：免于读界的是书架上写的那份文档，链接背后的东西没有被准入。
- **catalog 锁中毒＝`E_STORAGE_FATAL`，不落空到普通路径**：整名与包名同一个口径。落空会让一个与 skill 同名的城内文件或目录顶替它——正是「名字先查 catalog」要挡的那件事；恢复语说出「结束本 run 再续」，因为同一进程里这把锁再也拿不回来。

#### 8-29-4 没命中时给出 `nearby`（`runtime::tools::read::miss`）

```rust
// read::miss
pub(super) enum Floor { Document, Directory { dir: PathBuf, named: String } }
// 打开 read 落到的地方；Located::Absent 不打开，直接答没命中（§8-30-1）；
// Located::Present 打开之后须仍是判过的那个文件，否则 E_GATE_DENIED，读的是已打开的句柄。
pub(super) fn text_at(asked: &str, at: Located, floor: &Floor) -> Result<String, AxError>;
const NEARBY_CAP: usize = 16;
```

- **文件不在＝`E_INVALID_ARGS`，`nearby` 携最近一层存在的目录里的条目**：从被问路径的真实位置往上找第一个存在的目录，**不高于这次调用被准入的那一层（`Floor`）**：普通路径是它首段的真实位置，包是书架上写的包目录，catalog 里的单份文档没有目录可列（`Floor::Document`）。城根与书架上别的藏品因此不会出现在候选里——它们不是这次调用被准入的东西。条目按调用方够得着的写法拼出：普通路径是城内相对路径，包是 `<名>/<相对路径>`；保留区里的项不列，模型本来就读不到它们。按与缺失文件名的共同前缀长度（不分大小写）降序、再按路径排，截到 `NEARBY_CAP`。上限界定的是一次拒绝花掉多少窗口，不是目录多大；被截掉的是最不像的那些。
- **路径是目录＝`E_INVALID_ARGS`，`nearby` 携这个目录自己的条目**：`text_at` 在打开之前问判过的真实位置是不是目录，是就不打开，答「这是目录」，候选按上一条的写法、同一个 `NEARBY_CAP` 列出目录里的条目（不按名字远近排，只按路径排）。模型读一个目录，是在问里面有什么；不先问的话，Windows 打开目录得到的是 os error 5（拒绝访问），Unix 读目录得到的是 EISDIR，两者都落进下一条的「在而打不开」，模型拿到 `retry: no` 和一句按操作系统区域设置写成的系统报错，却拿不到它要的条目。
- **文件在而打不开＝`E_STORAGE_FATAL`，不给 `nearby`**：名字是对的，候选只会误导。
- **打开之后再核一次，不符＝`E_GATE_DENIED`**：`text_at` 打开判过的真实路径，然后核两件事：这条路径此刻的真实位置仍是它自己（路上没有换进来的链接），此刻这条路径上的文件与打开的句柄是同一个文件（`same-file` 的 `Handle`，比的是卷与文件号）；任一不符即拒，拒因不说出链接指向哪里。之后只从已打开的句柄读。两道核验各挡一种换法：判定之后换进来并一直留着的链接，打开与再开都穿过它而相等，只有重求真实位置看得见；打开时是链接、重求前又换回的，只有句柄比对看得见。剩下的窗口要在打开、重求、再开之间来回换三次。打开放在判定之后而不是之前，因为先打开就会在判定前打开链接背后未经判定的东西，Unix 上一个 FIFO 会让这次打开一直阻塞。catalog 里的单份文档也先经 `real_location`（§8-30-1）求真实位置，所以 `text_at` 的每个调用方交来的都是真实路径，核验对它们一视同仁。
- **列目录是尽力而为**：目录列不出或名字不是 Unicode 时 `nearby` 为空，调用方要的拒因是「没命中」本身。
- **恢复语指向 `search`，不指向 `exec`**：每栋楼的工具集都有 `search`，而 City Hall 的工具集里没有 `exec`（city-SPEC §8-22）；一句指向一件不存在的工具的恢复语会让规划者空转一个回合。

#### 8-29-5 打开一个 Locator：`cas:` 与 `file:`（`runtime::tools::read::locator`）

```rust
// read::locator —— `read` 与 `BoundReader`（§8-59）共用；交回 Locator 本身与它的字节，action 是拒词里点名的工具
pub(in crate::tools) fn open_locator(asked: &str, reader: &BoundReader, action: &'static str)
    -> Option<Result<(Locator, Vec<u8>), AxError>>;
/// cas: 块按哪栋楼判读取界的唯一判定处。
fn judged_at(hash: &B3Hash, origins: &[storage::BlockOrigin],
             bound: &dyn Fn(&Address) -> ReadVerdict, action: &'static str) -> Result<Address, AxError>;
// ReadTool::new(city_root, catalog, bound, block_store: &Path)：块仓是城的，run 可能写在没有自己块仓的 worktree 里。
// ReadTool 把 city_root、bound、block_store 收成一个 BoundReader；Locator 的字节由它读出，再按 UTF-8 交文本。
```

- **以 `cas:` 或 `file:` 开头的参数是 Locator**，按 `Locator::parse` 判形，判不过即 `E_INVALID_ARGS`；其余参数走 catalog 与普通路径，不受影响（一个城内地址不含冒号，两者不相交）。
- **`cas:` 块按存块时记下的楼判读取界，只在 `judged_at` 一处决定**，判本身仍是 `chosen_path::admit`（§8-30-1）那一个。来源是 `Cas::put_for` 在存块时写下的（storage-SPEC §8-3）：一个块为几栋楼存过就有几条来源，取读者能读的第一栋；一栋都读不了就取第一条来源，让 `admit` 按那栋楼的理由拒绝；没有来源的块（上架的技能包、从未存过的哈希）＝`E_GATE_DENIED`，恢复语让它改读块所出自的 `file:`。只按楼判、不按 run 判：读得了那栋楼的文件就读得了为那栋楼存下的字节，而 run 只记作出处。另一条路是按账本里哪一行写了这个哈希来判，落选：模型写的文字（例如委派的 `goal`）会落进带 `addr` 的行，那样的归属可以伪造。`file:<addr>@<oid>` 按 `<addr>` 判。
- **`file:` 在该 oid 上做 git 读**（`storage::blob_at`，storage-SPEC §8-29），读的是那一次提交里的字节而不是工作区此刻的文件；地址在该提交里不是一个文件（目录、不存在）＝`E_INVALID_ARGS`。
- **范围**：`cas:` 带的范围照 Locator 本身只交回那一段（`Cas::get_range`）；`file:` 带范围＝`E_INVALID_ARGS`，恢复语让它去掉范围改用 `offset`／`limit`——提交里的文件没有一份按范围读的实现，而 `offset`／`limit` 已答同一个问题。之后都按 `offset`／`limit` 切（§8-29-1）。字节不是 UTF-8＝`E_INVALID_ARGS`，read 只交文本。
- **为 run 存块的调用方都走 `put_for`**：转录（`Transcript::materialise`，记房间）、卸载的原件（`offload::tee`，记命令所在的房间，来源随 `OffloadSite` 传入）、截图（`bin::browser_tool`，记这栋楼）、交接单 must-read 里的规范文档（`accounting::worker::freezing`，记 run 所在的房间）、run 的任务书（`accounting::worker::dispatching::running`，记房间；run id 由任务书的定位符派生，所以先 `put` 取得哈希，run 立起后再 `put_for` 补记来源）、子 run 的交回说明（`accounting::worker::dispatching::handback`，记子 run 与它的房间）。仍走 `put` 的有两类：上架的技能包不是为某个 run 存的，读不到它的 `cas:`，它按 catalog 名读；冻结前缀的各段（`intern_prefix`）只为让账本里的前缀可审计，一个段为同一栋楼的所有 run 共用，不作为定位符交给任何 run。

#### 8-29-6 城外书架上的 skill：catalog 携着它的字节

```rust
// runtime::catalog
pub fn admit_carried_skill(&mut self, entry: CatalogEntry) -> Result<(), AxError>;
// entry.expansion＝扫描读到的 SKILL.md 正文；entry.hash＝同一份字节的哈希；entry.package＝None
```

- **按名读到的是钉住的那份字节**：城外书架是别的程序的目录，城给不出地址（city-SPEC §8-8），所以 catalog 不交地址而交正文。正文由 city 的扫描读一次、哈希一次（`Holding::carried` 与 `Holding::hash` 出自同一次读入），`read <名>` 经 `Expansion::Said` 交回它，走 §8-29-1 同一条切行路。于是这个 run 读到的字节恒等于 `run_started` 里那条 `SkillPin` 说的字节，哪怕那份文件在 run 进行中被它的主人改了。
- **包里的其余文件读不到**：`package` 为 `None`，`<名>/<相对路径>` 不进 §8-29-3 那条路，落回普通路径并按城内路径判（多半是 `E_INVALID_ARGS` 的没命中）。城外的文件不在城根之下，`read` 不打开城外的路径；要整包可读，人把它装进城库（city-SPEC §8-28），那条路把包里每个文件落在城内。
- **一件 skill 在 catalog 里只有一个名字**：城外的与城内的同名时，书架扫描已经按「近架盖远架」留下城内那一件（city-SPEC §8-8），catalog 收到的是一件；`admit_skill` 与 `admit_carried_skill` 之间的重名仍拒，与同一扇门里的重名同一个拒词。

### 8-30 runtime::tools::search（形状 1 判定＋形状 4 适配器）

**问题**：没有检索工具，找一个符号只有两条路——写 Python（要可选的 CPython-WASI 构件，很多机器上根本没有），或走 shell（Windows 上是 `findstr`，而 shell 本身是楼级配置可以关掉的）。旧对话有了一个地址，而**没有检索的地址比没有地址更糟**：模型被告知那里有东西，却够不着。

#### 8-30-1 runtime::tools::chosen_path（形状 1；模型选路的唯一判定处）

```rust
// 本 run 能读什么：装配层把 kernel::address::may_read 闭合在读者的楼与城的规则上（city-SPEC §8-2）。
pub type ReadBound = Arc<dyn Fn(&Address) -> ReadVerdict + Send + Sync>;
// 入一个字符串，出一个地址或一个三段式拒绝。本身无 I/O；bound 可能为他楼读一次规则。
pub(crate) fn admit(asked: &str, action: &'static str, bound: &dyn Fn(&Address) -> ReadVerdict)
    -> Result<Address, AxError>;
// 解析失败＝E_INVALID_ARGS；Address::is_reserved()＝E_GATE_DENIED；
// 读界答 Confidential 或 RulesUnreadable＝E_GATE_DENIED，后者的 subject 带上规则读不出的原因。
// 已准入的地址在盘上真正落到哪里：解析真实路径（沿途每一个 symlink 或 junction），把它在城里的地址
// 再交给 admit 判一次，返回真实路径。落在城外＝E_GATE_DENIED；落到保留区或关上的楼＝admit 的那条拒绝；
// 文件不存在也同样判，真实位置由 real_location 求出，答 Located::Absent；解析失败于别的原因＝E_STORAGE_FATAL。
pub(crate) fn land(city_root: &Path, addr: &Address, action: &'static str,
    bound: &dyn Fn(&Address) -> ReadVerdict) -> Result<Located, AxError>;
pub(crate) enum Located { Present(PathBuf), Absent(PathBuf) }
// 一条路径在盘上的真实位置，末尾几段不存在也算：盘解析最深的那个存在的祖先（沿途链接全解开），
// 其下不存在的段原样接上——不存在的段不可能是链接。接上的段不是普通名字（`..` 在内）＝E_GATE_DENIED；
// 路上某个存在的条目解析不了（目标已不在的链接在内）＝E_STORAGE_FATAL。
pub(crate) fn real_location(written: &Path, action: &'static str, subject: &str) -> Result<Located, AxError>;
// 打开之后再核一次：判过的真实位置仍解析到它自己（其上没有新放的链接），且此刻在那里的文件就是打开的那一个；
// 否则＝E_GATE_DENIED。read 与 BoundReader（§8-59）打开文件后都调它。
pub(crate) fn still_judged(asked: &str, judged: &Path, opened: &same_file::Handle, action: &'static str)
    -> Result<(), AxError>;
// 模型写下的路径在城里的拼写。本平台不算绝对路径的（`Path::is_absolute`，§12.4）原样交回；绝对路径解开真实位置（real_location）后落在
// 城根的真实位置之下，交回它相对城根的拼写（段以 `/` 相连，城根本身交回空串）；落在城外＝E_GATE_DENIED，
// 恢复语说出 `read`／`search`／`edit` 只到城内、城外的文件经 `exec` 读。它不判保留区与读界：交回的拼写
// 照旧过 admit 与 land。
pub(crate) fn within_city<'a>(city_root: &Path, asked: &'a str, action: &'static str)
    -> Result<Cow<'a, str>, AxError>;
```

**城内的绝对路径换成它的地址，再走同一道判定。** 页面把拖进输入框的文件存到城里（`hall/dropped/…`），插进消息的是文件在盘上的绝对路径；模型照抄它看到的路径，于是 `read` 以文法拒绝一个本就在城里的文件，resident 只能回头让人重打一遍。`read`、`search` 的起点与 `edit` 因此先调 `within_city`，再把交回的拼写交给 `admit`（`edit` 交给 `Address::parse` 与写域门）。换算只做「这是城里的哪个地址」这一件事，保留区、读界与链接仍由原来那一处判，所以没有第二个权威。落选的方案：在页面上把路径改写成城相对地址——页面不知道城根的真实位置（链接、junction、大小写），而且模型从别处（`exec` 的输出、日志）拿到的绝对路径同样会被拒。

**缺失的文件按它会落在哪里判，与存在的文件同一个函数。** 若文件不存在就交回字面路径，链接背后的缺失文件就绕过了判定：随后的「没命中」会列出链接目标那个目录的条目——城外的、机密楼的、包外的。`land`、`read::package` 与 `read::miss` 都经 `real_location` 求真实位置，各自只判「落点在不在我的范围里」；`..` 若接在已解析的祖先之后，会在盘从未看过的地方退出那个目录，所以拒绝而不接。

**判定时不在的文件，之后也不打开。** 接上的尾段是盘在判定那一刻没有的东西；若随后照这条路径打开，判定与打开之间在那里放下的一条链接会把打开带到它指向的任何地方——判过的是一处，读到的是另一处。所以 `real_location` 把「全都在」与「尾段不在」分成 `Located` 的两臂，调用方对 `Absent` 只报缺、不打开：`read` 直接答没命中，`search` 的起点答「不是城里的地方」，遍历里的链接略过。

**判的是盘打开的那个地址，不只是模型写下的那个。** 文法准入的地址仍可能穿过一个链接：开放楼里一条指向机密楼、保留区或城外的链接，打开的是链接的目标，只判字面地址就等于把 admit 拒掉的东西从侧门交出去。所以 `read` 与 `search` 的起点都走 `land`，`search` 遍历中遇到的每一个链接也走 `land`——链接的判定只有这一处。

它是 `read` 原有那段判定的搬家，不是它的第二份。三道判定次序固定：文法、保留区、读界——前两道不碰盘，读界为他楼可能读一次规则，所以排最后。`ReadTool::new(city_root, catalog, bound)` 与 `SearchTool::new(city_root, bound)` 各持同一个 `ReadBound` 的一份 `Arc`；装配层建一次，交给两件工具。`search` 遍历时对每一个候选文件同样只问 `Address::is_reserved()`——kernel 的那个原语——所以「什么是保留区」自始至终一个权威；读界则只在城根那一层问，一栋楼整栋开或整栋关（city-SPEC §8-3「楼是顶层地址」），楼里的条目继承楼的答案，不为每个文件再读一次规则。

#### 8-30-2 参数与结果

```rust
// args：{text, path?, context?}
// text：子串，必填且非空。**不是正则**。
// path：城相对前缀，缺省＝本 run 可读的全部：城根下每一栋楼先过读界，关上的整栋不走。走 chosen_path::admit。
// context：每侧上下文行数，缺省 0，上限 4（更大者夹到 4）。
const MATCH_CAP: usize = 64;      // 命中上限；到顶即停走，结果自陈 truncated
const FILE_BYTE_CAP: u64 = 1 << 20; // 单文件上限 1 MiB，越界不读，计入 unreadable
const UNREAD_SHOWN: usize = 16;     // unread 列出的条数上限
```

结果：`{matches: [{path, line, text}], count, truncated, unreadable, unread: [{path, why}]}`。`line` 是 **0 基**，与 `read` 的 `offset` 同一套编号，所以「搜到再读那一段」是把一个数字原样递过去。`unreadable` 是遍历中没能看过的目录与文件数——打不开的，和大于 1 MiB 的：**找不到与看不了是两个答案**，把后者吐成前者就是把一次失败抹掉。`unread` 按遍历次序列出其中前 16 条，各带一句原因：超过单文件上限的那一句指它去按区间 `read`，打不开的那一句带上系统给的错误（措辞只在 `search.rs` 里写）：一个超大文件 `read` 仍能按区间读，模型要知道是哪一个才去读。按策略跳过的（二进制、保留区、机密楼）不计入它。规则读不出的楼（`ReadVerdict::RulesUnreadable`）不是策略而是失败：它照样整栋不走，但计入 `unreadable`，并在 `unread` 里以楼的地址带上规则读不出的原因——静默跳过它，模型会把「这栋楼的规则坏了」读成「这栋楼里没有」，而修规则的只能是人。二进制指读得出而不是 UTF-8；读不出的文件是「打不开」，不再被当成二进制吞掉。

**不用正则表达式**，理由是模式引擎会把回溯放在模型和它的下一个回合之间。子串扫描是线性的，且一个模型写错的正则不会变成一次挂死。

#### 8-30-3 遍历跳过什么，以及为什么

| 跳过 | 理由 | 权威 |
|---|---|---|
| 保留区子树 | 一跑不读治理自己的东西 | `Address::is_reserved`（kernel） |
| 读界关上的楼 | 机密楼对楼外全关；规则读不出的楼同样关 | `kernel::address::may_read`（city-SPEC §8-2） |
| `.git` 目录 | 它是对象库不是文本，扫它只产出乱码命中 | 本节 |
| 非 UTF-8 文件 | 二进制里没有可读的行 | 本节 |
| `land` 以 `E_GATE_DENIED` 拒绝的链接 | 链接的目标落在城外、保留区或关上的楼 | `chosen_path::land` |
| 指向目录的链接 | 顺着链接走可能绕回自己走过的地方；要搜目标目录，按它真实的地址去搜 | 本节 |

`land` 以别的码拒绝的链接不在这张表里：真实位置解析不出（`E_STORAGE_FATAL`，比如一个指回自己的链接）是盘没有作答，不是一栋关上的楼，所以它计入 `unreadable`，`unread` 里带上那个错误；`chosen_path::walked` 只把 `E_GATE_DENIED` 当作跳过，其余的错误交给遍历者。

大于 1 MiB 的文件不读——把一个大对象读进内存找子串是一次停顿——但它不在这张表里：它是「没看」，计入 `unreadable` 并在 `unread` 里说出来。

#### 8-30-4 红测试

超过 512 行的文件返回恰 512 行、并给出真实 `total_lines` 与可续的 `next_offset`；`search` 找到子串并带上下文；`search` 对保留区前缀以 `E_GATE_DENIED` 拒绝；两者共用的 `chosen_path::admit` 有且只有一组测试，读界的两种关各一条拒绝。`search` 不带路径时不走关上的楼；大于 1 MiB 的文件计入 `unreadable` 并在 `unread` 里带原因；一条超过字节上限的首个命中被切进上限并带标记。开放楼里一条指向机密楼的链接：`read` 穿过它以 `E_GATE_DENIED` 拒绝，`search` 从开放楼走下去不交出机密楼里的命中。

### 8-59 runtime::tools::bound_reader：模型点名的字节，经读界判过后按字节读（形状 4 适配器；sprawling-SPEC 8-131、8-142）

**问题**：`read` 只交文本。城的工具 `ocr` 与 `transcribe` 要的是一张图、一段录音的字节，它们在读界之内的任意一栋楼里，或在连接器存进 CAS 的块里（§8-27-10、§12.15）。判「这条模型选的路径能不能读」的是 `chosen_path`（§8-30-1），在 accounting 里再写一份判定就是第二个权威。本节把那份判定公开成一扇按字节读的门，城里读别楼文件与 `cas:` 块的工具都只经它。

```rust
// runtime::tools::bound_reader（形状 4 适配器）
#[derive(Clone)]
pub struct BoundReader { /* city_root、bound、block_store —— runtime::tools 内可见 */ }
impl BoundReader {
    pub fn new(city_root: &Path, bound: ReadBound, block_store: &Path) -> BoundReader;
    // asked：城相对路径、城内的绝对路径、`cas:` 或 `file:` Locator。action 是拒词里点名的那件工具。
    pub fn open(&self, asked: &str, action: &'static str) -> Result<Opened, AxError>;
}
pub struct Opened { /* named、字节的来源 —— 私有 */ }
impl Opened { pub fn named(&self) -> &Named; }
impl std::io::Read for Opened { … }
pub enum Named { File(Address), Block(B3Hash) }   // 路径与 `file:` 是 File，`cas:` 是 Block

// runtime::pipeline::connector（形状 1 判定）：一张图的唯一认法
pub fn png_picture(bytes: &[u8]) -> Result<ImageRef, AxError>;
```

- **判定是 `read` 的那一套，次序也一样。** 以 `cas:`／`file:` 开头的走 `read::locator`（§8-29-5）：`cas:` 块按 `judged_at` 选出的楼过 `admit`，`file:` 按自己的地址过 `admit`。其余走 `within_city`、`admit`、`land`（§8-30-1），打开之后经 `still_judged` 核对打开的就是判过的那个文件。拒绝与 `read` 同码：文法不对＝`E_INVALID_ARGS`；reserved subtree、读界关上的楼、城外的绝对路径、经链接出城＝`E_GATE_DENIED`；不存在的 `cas:` 块（存块时没有记下任何楼）＝`E_GATE_DENIED`；判定时不在的文件与目录＝`E_INVALID_ARGS`；在而打不开＝`E_STORAGE_FATAL`。拒词的 action 是调用它的那件工具，所以模型读到的是自己哪一次调用被拒。
- **catalog 名不经这扇门。** 一件 skill 由人放进楼的阅览室，读它是 `read` 的事；一张图或一段录音不在 catalog 里，`ocr` 与 `transcribe` 收到一个 catalog 名，按路径判，多半是 `E_INVALID_ARGS`。
- **`read` 的 Locator 也经它。** `ReadTool` 持一个 `BoundReader`，`cas:`／`file:` 读出字节再按 UTF-8 交文本。路径仍走 `read` 自己那条路：没命中时它要列 `nearby`（§8-29-4），那是读文本的答复；按字节读的只说「不在」，恢复语指向 `search`。
- **门不设上限，上限归收字节的那个值。** `Opened` 是一个 `Read`：文件按需读，读多少由收它的值定，`gateway::Recording::read_from` 读到它的上限多一字节为止（gateway-SPEC §8-33）。`cas:` 块与 `file:` 的字节在打开时已整份在内存里，因为 `Cas::get` 与 `storage::blob_at` 只交整份；它们是这座城自己存下的，大小受存它的那条路约束。`ocr` 把一个文件整份读进来再判 `IMAGE_MAX_BYTES`，与 `read` 整份读一个文件相同：`ImageMaxBytes` 不交出它的数，按上限读要 kernel 给它一个读法。
- **`Named` 说字节从哪里来，容器怎么认归调用方。** 文件有名字，块没有。录音的容器在文件上看扩展名、在块上看开头的字节（gateway-SPEC §8-34），那是 gateway 那张表的事，本模块不认容器。
- **一张图只有一种认法：`png_picture`。** 连接器存图（§8-27-10）与城工具 `ocr` 读图都调它：字节在 `IMAGE_MAX_BYTES` 之内、PNG 头读得出宽高时，答一个指向这些字节的 `cas:` 哈希的 `ImageRef`；超限是 `IMAGE_MAX_BYTES` 自己的拒绝，头读不出是 `E_INVALID_ARGS`，主题说出为什么。连接器把拒绝写成那一行 `[picture left out: <主题>; <恢复语>]`。
- **验收**：`runtime::tools::bound_reader` 的测试。门对读界外的路径、reserved subtree、不存在的 `cas:` 定位符各拒一次，码与 `read` 对同一个参数的拒绝相同；楼里的文件与为本楼存下的块按字节读回，`named` 分别是 `File` 与 `Block`。

### 8-31 runtime::tools::exec 的环境声明

> 权威在 kernel-SPEC §8-22 的「11.1 增」段与 city-SPEC §8-4 的「11.1 增」段；本节只说 exec 这一侧怎么用它，以及构造面因此怎么变。

**只放四个名字的后果**：`ENV_ALLOWLIST: [&str; 4] = ["PATH", "LANG", "LC_ALL", "TZ"]` 加 `env_clear()` 时，城里的 resident 跑不动 `cargo build`——MSVC 链接器读不到 `%ProgramFiles(x86)%`，退回裸 `link.exe`，撞上 PATH 上 Git 那个 coreutils `link`。

**改法**：`ENV_ALLOWLIST` 原样留着，它是**每一栋楼无条件继承的地板**；楼在 `[sandbox] env_passthrough` 里逐名声明的是**地板之上加的那几个**。两份清单合并后按名取值，取不到的名字不进（一个没设过的变量不该变成空串——空串与未设置在 Windows 上是两件事）。

**构造面**：总在一起走的那几个值是一个有名字的值，`ExecTool::new` 因此只收三个参数：

```rust
pub struct ExecSetup {                 // 形状 2 值类型
    pub workdir: PathBuf,
    pub mounts: Vec<Mount>,
    pub python_wasm: Option<PathBuf>,
    pub shell: Option<PathBuf>,
    pub fuel: Fuel,
    pub env_passthrough: Vec<EnvVarName>,
    pub domain: Address,
    pub run: RunId,                    // 这张工具台服务的 run：它起的后台命令只交还给它（§8-28-1 第 4 条）
}
pub fn new(setup: ExecSetup, sandbox: Box<dyn Sandbox>, backlog: Backlog) -> Result<ExecTool, AxError>;
```

三个参数，在函数参数上限之内。

**账本上留什么**：配置写入时不记事件——`CONFIG.toml` 是「一跑受什么治理」的权威，再记一条同事实的事件就是第二个权威。账本记的是**一跑拿它做了什么**：`exec` 的结果载荷增 `env` 字段，列出这次真正递给子进程的名字，按名排序。名字不是值——值恒不入账本。

**验收**：声明了名字的楼里，`exec` 的子进程恰好看到那几个（加地板四个）；没声明的楼里只看到地板四个；凭据形状的名字在配置解析处被拒。以及一次真实演示：声明了名字的楼里 `exec` 跑得动 `cargo build`。

### 8-32 runtime::transcript（形状 2 值类型）

> 旧对话得到一个地址，于是 §8-30 的 `search` 从「方便」变成「必要前提」——一个不能检索的地址比没有地址更糟。

**它是什么**：一跑冻结时，把**模型实际看到的消息**——工具调用与其结果、sieve 产出的压缩形、指回被搁置部分的 rest 指针——写成 `<room>/<run-id>.jsonl`，一行一条 `ChatMessage`（`kernel::model::wire` 的 serde 形，原样，所以 `search` 找到的行号就是消息序号）。frozen prefix 不在其中：它是每次请求都相同的那一半，账本的 `prompt_assembled` 已经持有它的哈希。

**它不是账本**：账本住 `.sprawling/`，`read` 对那里的每一条路径都拒绝；而且账本是**全城一条链**——把它交给一个 resident 就是把一栋 confidential 楼的事件也交出去。transcript 不加密，谁读得到它由读界答：它住在 run 的房间里，`read` 与 `search` 对它的路径和对房间里任何文件一样先过 `chosen_path::admit`，所以本楼的 resident 读得到，非机密楼的 transcript 他楼也读得到，而机密楼的 transcript 楼外读不到（city-SPEC §8-2）。

**三步定序，不可颠倒**：①凭据扫描（`redact::redact` 逐消息走一遍，与 `model_returned` 入账本同一把扫描器）；②钉入 CAS（`storage::Cas::put`，内容寻址，同一份 transcript 写两次是一次）；③实体化（写到房间，只读位）。先扫后钉：一个钉进 CAS 的密钥永远删不掉。

```rust
pub struct Transcript { run: RunId, lines: Vec<String>, redacted: u32 }   // 私有字段，一处构造
impl Transcript {
    pub fn of(run: RunId, conversation: &Conversation) -> Result<Transcript, AxError>;  // 逐消息序列化、扫描
    pub fn address(room: &Address, run: RunId) -> Result<Address, AxError>; // `<room>/<run-id>.jsonl`，纯函数：路径在跑之前就已知
    pub fn materialise(&self, cas: &mut Cas, city_root: &Path, room: &Address) -> Result<TranscriptRecord, AxError>;
    pub fn lines(&self) -> &[String];  pub fn redacted(&self) -> u32;
}
pub struct TranscriptRecord { pub address: Address, pub original: Locator, pub redacted: u32 }
impl Run<Frozen> { pub fn transcript(&self) -> Result<Transcript, AxError>; pub fn plan(&self) -> &RunPlan; }
```

`Frozen` 状态因此保留 `Window`：冻结后唯一还需要它的读者就是这一处。地址是纯函数，所以 `freeze_plan` 在 drive 之前就能把它写进 Handoff 的 `context` 段——`handoff_written` 载荷携一行 `transcript at <room>/<run-id>.jsonl`，一条测试钉住这一行的存在。实体化发生在 drive 归来之后的 `conclude`（装配层持 CAS），失败记 diagnostics 而不让一次已冻结的跑变成 `Err`：冻结已在账本上，transcript 是它的副本。

**接线**：`RunPlan::predecessor` 为 `Some` 时，run segment 增一行 `Predecessor transcript: <address>`——继任者从 prefix 就知道去哪里 `search`。

### 8-33 runtime::tools::succeed 与 succession（形状 4 适配器）

> 三件事一起落地，缺一件就是一个洞。

**动词**：`succeed {reason}`。一跑请求由继任者接替自己：**同地址、同深度、同工具表**，无需人在环内。它答的是「继任者将在你冻结后于 `<addr>` 启动；先把 `Handoff.md` 写在你的房间里」，而不是一个结果——与 `delegate` 同理，工具不能从一个 run 的工具台里驱动另一个 run。桌面 `SuccessionDesk` 至多持一份请求（第二次调用覆盖第一次，理由随之更新）；装配层在 `conclude` 里读它，**被取消的跑不接替**（与 delegate 的第四安全点同一条规则）。

**深度守恒**：succession 与 `delegate` 是两个动词。`delegate` 让深度加一；succession 不加。`Assignment::depth()` 从 `parent` 推出，继任者**继承前任的 `parent`**（而不是以前任为 parent），所以深度按构造守恒，工具表因而与前任逐名相同——`delegate` 在内。红测试：继任者的工具表与前任逐名相等。

**账本**：`Assignment`／`RunPlan` 增 `predecessor: Option<RunId>`，写进 `run_started` 的 `predecessor` 键；`Provenance` 增同一指针（storage-SPEC §8-17：第六条 trailer `Sprawling-Predecessor`，仅在有前任时出现；`model_fields` 同时写 `predecessor`）。`accounting::views` 从 `run_started` 折出 `predecessors: BTreeMap<RunId, RunId>`，`Query::Commit` 的答 `CommitAnswer` 增 `lineage: Vec<RunId>`——本跑在前，逐级向前到第一任（wire-SPEC §8-18）。红测试：三次接替后 lineage 有四个 run。

**Handoff 住房间**：`city::handoff(city_root, room)`／`handoff_path(city_root, room)` 读写 `<city>/<room>/Handoff.md`；模板在 `city::open_room` 打开房间时铺下，楼级 `lay_out` 不铺它。理由是同楼并发：一栋楼一份 Handoff，两个房间同时冻结就是两份内容抢一个文件。

**质量防线（不可选）**：`accounting::worker::probing::probe` 的 handoff 探针在每次 succession 真的跑。`handoff_probe()` 给出固定四问（版本 1）；装配层 `accounting::worker::probing` 在 `conclude` 读到接替请求时，用前任的 adapter 对前任的 transcript 问一遍（before），在继任者 `freeze_plan` 之后、第一回合之前，用继任者的 prefix 问一遍（after），`compare` 后记一条 `eval_run`（`probe`／`version`／`kept`／`lost`／两份答案）。探针答案不是判定，`lost` 报的是位置，人自己去读两份答案——这正是 sprawling-SPEC §8-39 交接探针一节定的口径。每次 succession 两次模型调用，这是这道防线的价钱，写在明处。

### 8-34 runtime::reminder（形状 1 判定）

**两道阈值**，以 provider 报回的 `input_tokens`（事实）对模型的 `context_tokens`（`CallShape::context_tokens`，来自 endpoint 簿）计算；**恒不用窗口字节数**（那是估计）。

| 阈值 | 说的话 |
|---|---|
| 25%（`CTX_REMINDER_FIRST_PERCENT`，恒不可调） | 只报用量：`[context] 25% of the window used (N of M input tokens).` |
| 第二道：缺省 65%（`CTX_REMINDER_SECOND_DEFAULT`），合法域 30–90、可由配置梯子调（kernel-SPEC §8-22） | 报用量，并说明剩余预算仍够写 handoff 并 `succeed`，过了这一点就不够了 |

**每道阈值一跑恰响一次**。状态是穷尽枚举 `Sounded { Nothing, Quarter, Handover }` 而不是两个布尔；一跳越过两道（0→70%）时只响高的那一道，低的一并作废——两句话叠在一起是噪声。

```rust
pub struct ContextGauge { window: Tokens, second_at: u64, sounded: Sounded }
pub struct ContextReading(Arc<AtomicU64>);   // Clone＋Default；Run 写、status 读的同一格
impl ContextReading { pub fn tokens(&self) -> Tokens; pub(crate) fn record(&self, used: Tokens); }
impl ContextGauge { pub fn new(window: Tokens, second: Option<SecondThreshold>) -> ContextGauge; pub fn observe(&mut self, used: Tokens) -> Option<ContextReminder>; }
pub enum ContextReminder { Usage { used: Tokens, window: Tokens }, HandoverWindow { used: Tokens, window: Tokens } }
impl ContextReminder { pub fn render(&self) -> String; }
```

`second` 来自 `RunPlan.second_threshold`：配置梯子冻结的值，`None`＝没有一层说话，取 `CTX_REMINDER_SECOND_DEFAULT`，「缺席取默认」只在这一个构造点判定。`window == 0`（簿上没写）恒不响：没有分母就没有百分比，与 `UnplannedProgress` 同一条理。整数算术：`used * 100 / window` 用 checked 乘法。

**接线**：`TurnReport` 增 `usage: Option<ModelUsage>`；`RunPlan` 增 `second_threshold: Option<SecondThreshold>`（Run 起点冻结，理由住 kernel-SPEC §8-22）；`RunPlan` 增 `context: ContextReading`，Run 在每回合观察之前把同一个 `input_tokens` 记进去，装配层把同一格交给 `StatusTool::metering`——只有一处写，窗口提醒与 `status` 读的是同一个数；`Run<Active>` 持 `ContextGauge`，每回合以 `usage.input_tokens` 观察，响则以 `Conversation::push_reminder` 落在该回合工具结果之后——与 steer 同一扇门，所以它「落在下一次工具结果的尾部」。`pipeline::PackContext` 同时增 `reminder: Option<ContextReminder>` 作第四个附件，句子只在 `ContextReminder::render` 一处定义。

**改这一格的入口**：`wire::Command::ConfigureBuilding` 的 `context_second_threshold`（wire-SPEC §8-45）写的就是 `RunPlan.second_threshold` 读的那一格——写入落那一级的 `[context] second_threshold`，下一个 Run 起点冻结时读到；正在跑的那个 Run 不受影响（冻结的理由见 kernel-SPEC §8-22）。

### 8-48 回合没有上限

`RunPlan` 没有回合上限与花销上限，`drive` 是一个 `loop`。一次跑的结束只有三种来路：一回合作出结论、一次带 carrier 事件的失败（写进历史后冻结为 cancelled）、或一个安全点送到的中断。

- **理由是刹车只留一个**：没有人能在一件事跑之前给它定价，而一个替人说停的数字，停的时刻恰好是人最不希望它停的那一刻。要停一片就 `Halt`——它会终止那片里的后台成员；要停一条就 `Cancel`。
- **`Completion::Limit` 保留**：回合上限没有了，这个词却仍有一条来路——一条什么都没说的回复（§8-37）。

### 8-37 一条什么都没说的回复，不是「做完了」

`Run<Active>::advance` 的收尾判定从「本回合没有工具调用」一条，改为三问：没有工具调用、**答里有内容**、且不是停在上限上——三条同时成立才是 `Completion::Done`，否则是 `Completion::Limit`。

- **理由是证据**：`Completion::Done` 必须引一条 `model_returned` 作证据，而一条 `content` 为空的 `model_returned` 证明不了任何工作完成。判它做完，等于在「什么都没说」的那一刻告诉人「你的活干完了」，并且把那条空记录作为凭据写进账本。
- **`stop` 随回合走**：`StopReason` 随 `ToolWave` 与 `Recording` 两个 typestate 到 `TurnReport::stop()`，判定读它而不是从内容去猜——一条被截断但**有内容**的回复同样不是完成，那件事只有 `stop` 说得清。
- **否决「只看内容空不空」**：`stop == MaxTokens` 而内容非空的回复是半句话，判它完成同样是假历史；只看内容会把它漏掉。
- **citysim**：剧本以显式说一句话收尾（`citysim::concluding`），因为它们要断言的是「跑到工作做完」，而不是「模型不说话了」。剧本 `a_reply_that_says_nothing_freezes_as_limit_rather_than_done` 用**供应方真实报文**（`content: []`、`stop_reason: max_tokens`）经生产翻译喂进来，钉住这条规则。

### 8-36 写域的两道闸各问一个问题（kernel-SPEC §8-46 末段）

- **`bench::admit`** 对 `Effect::Write { domain: area }` 改调 `kernel::reach(&self.domain, area, &self.taint)`：工具声明的是一块区域，门口只问这块区域够不够得到。
- **`tools::edit::invoke`** 解析出 `target` 后调 `kernel::domain(&self.writable, &target, &TaintSet::empty())`：`Allow` 继续，`Deny { refusal }` 原样作 `Err`（三段式因此由 kernel 一处产出，工具不自拼 `Outside` 的话术），`Escalate` 在写域门上不可能出现——`GateOutcome` 刻意穷尽，这一臂如实答一条 `E_INVALID_ARGS` 说明该不变量，而不是 `unreachable!`。空 `TaintSet`：taint 是 bench 的事实，工具这一层没有它，拒词因此少一句「派生自 N 个外部来源」——那句话仍由门口那道 `reach` 说。
- **验收**：`tools/edit/tests.rs` 钉住「Documents 域的工具创建 `<city>/hall/note.md` 成功、创建 `<city>/hall/note.rs` 被拒（`E_OUTSIDE_WRITE_DOMAIN`，主语是文件）」；`bench/tests.rs` 钉住「Documents 域、声明区域为 `hall/mayor` 的 `Write` 效果在门口放行」。

### 8-35 去重答的是第一次的结果，而不是一句「你已经问过了」（形状 1 判定）

**「同一次调用做两遍」的正确答案是第一次的结果**：回一个错误，重试的模型学到的是「这件事失败了」，而它其实成功了——那是幂等只做了一半：副作用被挡住，答案没有被记住。所以 `ToolBench` 记住的是键与答，两个调用方（`accounting::worker::driving::lane`、`citysim::executor`）把 `Duplicate` 回成第一次的 `ToolOutcome`。

```rust
seen: BTreeMap<IdemKey, Result<ToolOutcome, AxError>>   // ToolBench 私有
pub enum BenchOutcome { …, Duplicate { outcome: ToolOutcome } }   // 调用方回第一次的 ToolOutcome，checkpointed 为空
```

- **写入点**：键与答在工具答过之后一起写入（`account`），失败的答也记下；被门拒的调用不入表，所以被门拒后的重试不算重放。
- **`Duplicate` 仍是一个独立变体而不是并进 `Ran`**：`checkpointed` 对重放恒为空，而 `Ran` 的调用方要按 `checkpointed` 决定波后清扫；把两者合并会让「这一波要不要扫」多出一个恒空的分支。
- **代价写在明处**：一次运行期间每个成功调用的结果都留在内存里。这与 `seen` 本来就要活到运行结束是同一条寿命，多出来的是 payload 的字节；一次运行的工具调用数以百计而非以百万计。

### 8-38 sink 收到的是一条 entry，不是一行文本（形状 2 值类型）

**sink 收的是 entry 而不是渲染好的行**：否则「把日志行推给浏览器」的装配层要把刚渲染好的那行**再解析回来**才能拿到 `level`／`run`／`seq`／`module`，一条日志行由什么字段构成就有写与读两个权威，而读的那个漂了也没人看得见。

**改法**是把字段本身交给 sink，文本留作其中一种去处：

```rust
pub struct Entry<'a> { pub level: Level, pub site: Site<'a>, pub message: &'a str }  // message 已过扫描
pub type Sink = Box<dyn FnMut(Entry<'_>) + Send>;
pub fn render(entry: Entry<'_>) -> String;   // 一行 JSON：文本的唯一产出点
```

- **`redact` 的位置不动**：`write` 仍在交给 sink 之前扫一遍，所以任何 sink 拿到的 `message` 都是已脱敏的，不存在「某个 sink 忘了扫」。
- **`render` 转公开而不是复制一份**：写终端的 sink 调它，携字段上路的 sink 不调它，两者因此不可能对「一行由什么构成」得出两个结论。
- **无读方法这一条不受影响**：`Entry` 只在写入那一刻存在于 sink 的参数位上，本模块仍不提供任何把已写的行读回来的途径。
- **被否**：保留 `&str` sink，由装配层 `serde_json::from_str` 回读。它多一次序列化与一次解析，且给「字段名」造了第二个权威——而第二权威失配的表现是浏览器上少一个字段，不是一次编译失败。

### 8-39 一段 prefix 自带它的来源，`prompt_assembled` 因此只有一条分支

`FrozenSegment` 自带来源注记，而不由 `build_prefix` 另行拼出；否则同一件事有两个作者——照计划装配出来的 prefix 写一种行，在城里按手边字节装配出来的 prefix 写另一种（或不写）。`replay::rebuild_prefix` 只读得懂前者，而页面要读的恰好是后者。

```rust
pub struct SegmentSource { pub addr: Address, pub kept: u64, pub dropped: u64 }
impl SegmentSource { pub fn whole(addr: Address, kept: u64) -> SegmentSource; }

impl FrozenSegment {
    pub fn new(slot: SegmentSlot, bytes: Vec<u8>) -> FrozenSegment;              // 无来源文档
    pub fn assembled(slot: SegmentSlot, bytes: Vec<u8>,
                     sources: Vec<SegmentSource>) -> FrozenSegment;
    pub fn sources(&self) -> &[SegmentSource];
}
```

**三条口径：**

1. **来源行只有一个作者。** `prompt_payload` 的两条分支合成一条：每一行的 `slot`／`hash`／`len`／`sources`／`skipped` 都从段本身取，`breakpoints` 恒上线。由此，在城里装配的 prefix 与照计划装配的 prefix 在同样的键下写同样的行，`rebuild_prefix` 读的仍是它一直在读的那四个键（`addr`／`kept`／`marker`／`dropped`）。
2. **`marker` 由 `dropped > 0` 派生而不是独立字段。** 两个互相蕴含的字段是两个可以互相矛盾的字段。
3. **没有来源文档的段写空表而不是省略键。** resident 段由身份与目录拼成，不出自任何文件；空表说的是「它不来自文档」，缺席的键说的是「不知道」。
4. **`breakpoints` 记本次请求实际发出的断点，而不是四个 slot 名。** 行由 `BreakpointPlan::breakpoints()` 拼出，值是段界的 slot 名与 `tail`。类型仍是 `Vec<String>`、键名不变、`#[serde(default)]`，所以写着 `["city","building","resident","run"]` 的记录照读：那是 slot 清单，其中 `run` 段界从未在请求里出现过。**被否**：保留 slot 清单另加一个 `plan` 键——那样账上仍有一行名为断点却不是断点的数据，读者要自己知道该信哪一个。

5. **`prompt_assembled` 每个 run 只写一条。** `turn::prompt::PromptRecord` 记着本 run 最后写下的载荷；`assemble` 照常算出本回合的载荷，与之相等就不写，不等才写并记住。prefix 在 run 内冻结，断点计划只随「对话是否为空」变，所以此后各回合的行是第一条的逐字拷贝；回合间真正会动的请求区域由 `prompt_shape_compared` 逐回合记，尾锚恒落在最后一条消息上，无须逐回合重述。比较的是载荷本身而不是「是不是第一回合」：哪天某个回合的载荷真的变了，账上就多一条，账本不会替一个请求声称它没带的断点。读者据此按 run 取段：`storage::attribution` 以 run 为键保存段权重（storage-SPEC），`sprawling::views::prefix` 读一跑的第一条。旧账每回合一条，照读，因为同一 run 的各条相同。**被否**：之后的回合写一条引用首条的短记录——它不携任何读者需要的事实，只多一行链。

**被否**：让页面在被问的那一刻重新装配一次 prefix 去拿来源。此刻的文件不是当时的文件，那样画出来的是一份没有任何人收到过的提示。

### 8-40 一个 backlog 的草稿目录：名字必须唯一，而 `BacklogId` 给不了这个唯一

```rust
static BACKLOGS: AtomicU64;                                  // 本进程开过几个 backlog
fn scratch_dir(&self, id: BacklogId) -> PathBuf;             // temp/sprawling-<pid>-<backlog>-<member>
```

**三条口径：**

1. **`BacklogId` 与目录名回答的是两个问题。** id 在**一个** backlog 内部指认一个成员，下一个 backlog 的 id 又从 1 开始；而目录落在整台机器共用的临时目录里。只用 `<pid>-<id>` 起名，等于假定那个计数器是进程全局的——它不是。同一进程开两个 backlog，两者的第一条命令必然撞同一个路径。
2. **撞上之后是无声的。** `File::create` 截断另一方正在写的文件，`collect` 读完即 `remove_dir_all`，删掉另一方还在写的目录。人看到的是一条**退出码为 0、输出为空**的命令——既不报错也不重试，因为从每一方各自的角度看都一切正常。
3. **生产只开一个 backlog，唯一性照样不交给调用方。** `accounting::worker::lifetime` 全进程一个，而每个跑命令的测试各开一个、`cargo test` 又让它们同时跑。这不构成「只是测试问题」：把唯一性建立在「调用方只会开一个」之上，是把一条不变量交给调用方保管。`backlog::tests` 直接判目录名而不去赛跑两条命令——缺陷本身是确定的，只有损害是时序性的。

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

### 8-42 runtime::elision（形状 2 值类型＋形状 6 数据面；**「这里被裁掉了」的唯一权威**）

```rust
pub enum Elided { Nothing, Head, Middle, Tail }          // 被拿走的那一段在哪一头
pub struct Cut { pub text: String, pub dropped: ByteLen, pub place: Elided }
impl Cut { pub fn whole(text: &str) -> Cut; }            // 什么都没拿走
pub fn marker(dropped: ByteLen) -> String;               // `[truncated: N bytes]`
pub fn gap_marker(dropped: ByteLen) -> String;           // 同一句，独占一行
pub fn marker_room(text_len: usize) -> usize;            // 计数未知时按最宽预留
pub fn gap_marker_room(text_len: usize) -> usize;
pub(crate) struct Boundary<'a>;                          // 一个切口：切点两侧的字节，各是完整字符
impl<'a> Boundary<'a> {
    pub(crate) fn before(text: &'a str, at: usize) -> Boundary<'a>;
    pub(crate) fn after(text: &'a str, at: usize) -> Boundary<'a>;
    pub(crate) fn head(self) -> &'a str;
    pub(crate) fn tail(self) -> &'a str;
    pub(crate) fn offset(self) -> usize;
}
pub fn boundary_before(text: &str, at: usize) -> usize;  // 不大于 at 的最大字符边界（Boundary 的偏移投影）
pub fn boundary_after(text: &str, at: usize) -> usize;
pub fn splice(text: &str, front: usize, back: usize, place: Elided) -> Cut;
```

**四条口径：**

1. **一句话，一个产地。** prefix、pipeline、compaction、sieve 四处都会拿走字节，给读者的那句话只有 `marker` 与它的分行形式 `gap_marker`。`replay::rebuild_prefix` 逐字重写同一句才重算得出段哈希：拼写一旦有第二个家，离线重放就在哈希对拍处失败，而失败发生在与改动无关的另一个模块里。
2. **`dropped` 数的是源字节，标记自己不算。** 去掉标记后的长度加上 `dropped` 恒等于输入长度。计数由 `splice` 从两个边界值算出，与插标记是同一个动作；调用方拿两个长度相减求丢弃量，减出来的数会把插进去的标记当成幸存文本，因此这条路在接口上不再存在。
3. **切口只由一个值给，而它给的是两侧的字节。** `Boundary::before` 与 `Boundary::after` 是全 crate 仅有的两处字符边界判定；判定的答复就是这个值本身——它向文本要那一次切分，拿回切点两侧的字节，所以**不存在一个能让调用方自己算成非边界的下标**。`boundary_before`／`boundary_after` 是这个值的偏移投影，供只需要位置的调用点用；`splice` 与 `keep_sections` 都向它要两侧的字节，因而 `dropped` 与幸存的字节出自同一个值，合起来恒是输入。同一个循环写成几份时行为一开始都相同，分叉是无声的：任一处改成向上取整或加最小保留量，另几处不会跟。裸下标与它的文本是两条可以互相矛盾的事实，`str::get` 在那个矛盾上答 `None`，而调用方读成「这里什么都没留下」——丢掉的正是截断本该保留的字节。
4. **预留按最宽算。** 标记的宽度随计数的位数变，而计数要等切完才知道；`marker_room(text.len())` 给出该文本能产生的最宽标记，因为丢弃量不可能超过文本自身长度。于是「结果不大于预算」由预留保证，而不是由一次事后检查补救。

**关门测试**（`elision::tests` 与 `compaction::tests`）：`splice` 的 `dropped` 加上去掉标记后的长度恒等于输入长度——**对任意一对下标成立**，包括落在字符内部的与顺序颠倒的一对（proptest 量化，不是举例）；任何切口落在字符边界；`compact` 对一段日志报出的丢弃量与幸存字节数加起来是原文长度。

### 8-49 turn::recovery —— 模型调用恢复管线的段契约（形状 2 值＋形状 3 内缝）

**错误分层三层各管一段。**「同一个请求还能不能再发一次」（`AxError::retry`）的唯一家是 `gateway::endpoint::failure::ProviderFailure`；本模块的恢复段只修**同样的请求再发一次也注定同样失败**的形状类失败——换一扇门再问一次；可重试失败与「再发也一样」的拒词归 `runtime::watchdog`（§8-9）处置。三层互指互不越权：`ProviderFailure` 对可重试族的恢复语说由 watchdog 退避后再发同一个请求，段对那一族恒 `Skipped`。
```rust
// turn/recovery.rs（形状 2 值＋形状 3 内缝；pub(super)：turn 之外没有第二个用户）
pub(super) enum SegmentOutcome {
    Recovered(ModelReturn),   // 修好了：回合拿这个 ModelReturn 照常写 model_returned
    Failed(AxError),          // 认出失败且动了手，没修成：修复重发拿到的原错误，码与恢复语必带
    Skipped(AxError),         // 不是本段要修的：被递上的错误原样转手给下一段
}
pub(super) trait Segment {
    fn attempt(&mut self, failure: &AxError, call: &mut ModelCall<'_>) -> SegmentOutcome;
}
pub(super) struct ModelCall<'a> { /* journal、ledger、model、request、streamed —— 全私有 */ }
impl ModelCall<'_> {
    pub(super) fn open(journal: &mut Journal, ledger: &mut dyn Ledger,
                       model: &mut dyn Model, request: &ModelRequest) -> ModelCall<'_>;
    pub(super) fn streamed(&self) -> bool;               // 失败那次是不是从流式门出去的
    pub(super) fn resend_blocking(&mut self) -> Result<ModelReturn, AxError>;  // 换阻塞门再问一次
    pub(super) fn ask(&mut self, segments: &mut [&mut dyn Segment],
                      generating: Generating<'_, '_>) -> Result<Settled, AxError>;   // §8-50
}
/// 接力：名单序逐段递上失败；Skipped 转手即问下一段；Recovered／Failed 即止；
/// 全段转手则把最后转手的错误原样答成 Skipped。
pub(super) fn recover(segments: &mut [&mut dyn Segment], call: &mut ModelCall<'_>, failure: AxError)
    -> SegmentOutcome;
pub(super) struct BlockingResend;                        // 今天唯一的生产段
```

**五条口径：**

1. **三值穷尽且闭。** 新答案必须逼每个调用方表态；`Skipped` 必携它被递上的那个错误——code／action／subject／recovery／retriable 逐字段同行，跳过是转手而不是吞掉，`let _ =` 一族在本模块没有落点。每段的答案都是类型化 `AxError`，稳定码与恢复语由 `AxError` 的构造契约保证必带（"a failure with no next step is unconstructible"）。
2. **接力序就是名单序**，第一个非 `Skipped` 的答案终止接力。`recover` 自己也答同一个三值型（全段转手＝`Skipped(原错误)`），管线套管线仍是一种形状。
3. **落账先于效果，每次外发都算。** `model_called` 在每一次尝试之前落账（首打与段的重发同走 `ModelCall`），历史里第二条 `model_called` 就是那次修复重发——与 §8-9 重试同一读法，不是一次无声的重复。
4. **错误形不动。** 不新增 AxCode、不给 AxError 加字段；`AxCode::carrier()` 的穷尽不变。
5. **回合层只见 `Result`。** `ask` 把三值折成 `Ok(return)`／`Err(该带的那个错误)`：段间谁转手了什么是本模块面的事实，回合层不需要第二次解读。

**失败码**：本层不造码。`Failed` 携带的是修复重发拿到的原错误（生产段即阻塞门自己的失败，码由 gateway 一处给出，通常是 `E_WIRE_MISMATCH`）；全段转手即首个失败的原码，逐字段不变。`E_WIRE_MISMATCH` 的"能否定义掉"随 §12 走：恢复段只是把「同样的请求再发一次」换成「换一扇门再问一次」，没有改变该码的可定义性。

**生产段 `BlockingResend` 的触发点，一个都不多**：失败那次走的是**流式门**且失败码是 `E_WIRE_MISMATCH`。理由：两扇门是同一条缝的两个口（`kernel::model` 保证同答同败），但**流式装配与整身解析是两条解析路径**——流式工具调用拼接出的半句话在阻塞门是一份完整 body。其余失败一律 `Skipped`：可重试族归 watchdog，门拒与配置族换门重发只会把同一个拒词买回来。

**被否（各记理由与会重开它的参数）：**

- **空响应后追问一句的段**：「一条什么都没说的回复」意味着什么，§8-37 的 `concluded` 是唯一判定处（`Completion::Limit`），调用层再判一次就是第二个家，而 nudge 重发还会改写那条以供应方真实报文钉住的剧本。参数：有"空响应是瞬态"的实测数据时，与 §8-37 一同重开。
- **「剔除被拒工具后重发」段**：本仓的 provider 拒绝从不回引 body（`ProviderFailure::Refused`），没有类型化触发点可依，靠错误文本嗅探即造脆弱权威；无声砍工具是能力的静默降级，与"拒答即声明"相悖。参数：gateway 的拒绝族带上类型化的拒绝面之后重开。
- **「补悬空 tool_result 后重发」段**：回合窗口按构造无悬空调用（被取消的回合不入窗；`fold_run` 遇开波回退到上一安全点），触发点不存在；账本侧关帐的权威在 `replay::resume`，不写第二份。参数：出现能把悬空对话推进窗口的新入口时重开。
- **盲重试段（对可重试失败原样重发）**：该判定属于 watchdog 与 `gateway::admission`（§8-9：watchdog 只判断还有没有下一次），段越权即第二个重试权威。
- **`Skipped` 无载荷（unit 变体）**：结构上确实吞不掉错误，但"这一段转手的是哪个错误"也在答案里读不出来了，而谁把什么交给谁正是段契约要陈述的事实。
- **段＝闭枚举（形状 6）**：多段场景只能靠触发点拼装，契约测不直接，且新修复进来即改枚举与全部 match。trait 的第二实现是测试里的记账段（本仓缝规则认可的替身一族：测试时钟、计数店）。

### 8-50 回合记下回复的首个内容几时到（`turn::recovery`）

```rust
pub(super) struct Settled {
    pub(super) returned: ModelReturn,
    pub(super) speculated: Speculated,
    pub(super) first_at: Option<TimeMs>,   // 落账的那一次尝试的首个内容；写进 model_returned（kernel-SPEC §8-75）
}
```

1. **首个内容**：一段非空的文字（`Increment::Said`）或推理（`Increment::Thought`）。空增量不算；心跳、角色声明与只带用量的帧从来不成为增量（gateway 的 `increment_of` 把它们挡在口外），所以口内口外是同一个定义。
2. **在哪里读钟**：`ask` 把所选的门收到的增量汇点包一层，第一段非空增量到达时经 `Journal::read_clock` 读一次，此后不再读。`Speculating` 而没有页面在看（`deltas: None`）时也包：那扇门照样是流，量它不需要有人看。`Unwatched` 走阻塞门，没有增量，`first_at` 缺席。
3. **属于落账的那一次尝试**：流式尝试失败、换阻塞门重发（`BlockingResend`）修好的回复，`first_at` 缺席。那次重发之前写下的 `model_called` 是它的起点，而它没有流；把失败那次的读数挂到它上面，读者算出的首字耗时量的是另一次请求。
4. **读钟失败即回合失败**：与 `model_returned` 自己那一刻同一只钟、同一种失败。汇点不能失败（`kernel::Increments` 没有返回值），所以读数先存下，模型调用返回之后再抛出。

### 8-51 一次调用在账上带出它的登记，与它被裁掉的原文在哪（`turn::wave`、`pipeline`）

**(a) 工具面交出整份登记**

```rust
pub trait ConcurrentInvoke {
    /// 这次调用那件工具的登记；None 是工具台不认识的名字（它不是只读的）。
    fn meta_of(&self, call: &ToolCall) -> Option<&ToolMeta>;
    // ahead、admit、tool、account 不变
}
```

- `meta_of` 取代原来的 `effect_of`：工具波判只读前缀、推测门判能否提前起跑，读的都是登记里的 `effect`；`tool_called` 还要照录 `effect` 与 `render`（kernel-SPEC §8-75(b)）。一个方法交出整份登记，三处读同一个答案；两个方法各交一项，就是两处各自去查同一份登记。
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

### 8-43 重试上限住 kernel

两个 crate 互不依赖，而 gateway 决定要不要再发一次请求、`Watchdog` 决定要不要冻结这次运行，读的是同一个事实——所以 `Retries` 住 `kernel::retries`，两边都直接用 `kernel::Retries`，不导出别名：别名让读者以为有两个类型，调用点于是写出一个两臂恒等的 `match` 去「转换」它们。

### 8-44 runtime::compaction::exchange（形状 2 值＋形状 1 判定）：回合边界的压缩

一回合加进 window 的东西是一对值：发出调用的助手回复与那波的答案——少了任何一半都不是对话，所以它们是一个值 `Exchange`（形状 2），由 `Turn<ToolWave>` 在波落地时收集、由 `runtime::fork` 从同一批记录重建。压缩只发生在 `Turn<Recording>::record` 的收尾边界（工具波全落地之后），这扇门一个回合只开一次（形状 1 判定住 `compact_texts`，调用的是 `compaction::plan`）。

- **判定并入 `compaction::plan`，`turn.rs` 不另写阈值**：exchange 的总字节对着预算 `EXCHANGE_BUDGET_BYTES`（住 `kernel::consts_policy`）触发；触发后每段文本按**均分份额**过 `plan`：`Keep` 与 `MustOffload` 原样，`Cut(Strategy)` 走 `shorten`。除 `plan` 外这条路上没有第二次「装不装得下」的比较。
- **波中永不换快照**：半落地的波会让压缩按半截分组，而 `fork` 是一回合一回合同一值重建的——两边分组不同就是同一历史两种字节，分支从此与母亲分叉。故收集期不压、只在收尾边界对全波一次压；红测用「全波分组与半波分组产出不同字节」钉住这一点。
- **`MustOffload` 在这扇门上不动文本**：它要的是整块离窗留引用，而这扇门没有 store（tee 在落地管线那一侧），城里也没有工具解析得了引用——所以文本留原样。回路不是窗口：进这扇门的文本恒等于账上 `model_returned.data.content` 与 `tool_result` 里的那一份（`turn/wave.rs` 只打印一次、`fork` 由同一条记录重建），而 `exec` 结果在落地时已被管线裁过或 tee 过——原件在 CAS、账上有 `result_offloaded` 指过去。两条路都没有「只存在于窗口里」的字节。
- **thinking 与 redacted thinking 永不碰**：thinking 块带覆盖其字节的签名，redacted 形态是封好的密文。
- **`runtime::fork` 同边界重放**：`fold_run` 的 `Wave` 持同一个 `Exchange`，在波收齐（或整波丢弃）的同一点 `compact()`——live 折叠与离线重建因此同源同字节，C16 的承诺由这条对拍承接。
- **唯一的失败**：`Exchange::compact` 在一段文本计不进 `u64` 时以 `E_INVALID_ARGS` 报（动作＝压缩这一回合的 exchange，主体＝那段文本，recovery 指向本模块）——这是「没有人解析得了的窗口字节」，不是可恢复的压缩结果。live 路径（`record`）与回放路径（`fold_run`）在同一个值上走同一次判定，故同一段文本两边同样拒，回放不会因为压缩而少一条分支。
- **数字一个家**：预算只住 `consts_policy::EXCHANGE_BUDGET_BYTES`，本文件不复写它的值。

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
- **`Stage` 带什么由调用方定**：`RunHooks::checkpoint` 仍只收时刻；stage 哪些路径，由持有 `ToolBench` 的一侧决定，因为只有 bench 知道每个调用的工具。`ToolBench::invoke` 在 `BenchOutcome::Ran.wrote` 里交回工具的 `Tool::writes`（kernel-SPEC `Writes`）；sprawling 的 lane 把上次 checkpoint 以来各调用的 `wrote` 用 `Writes::and` 并起来，下一次 checkpoint 只 stage 这些路径，并在 checkpoint 后清零。run 的第一次 checkpoint、以及并出来是 `Domain` 或 `Nothing` 的那次，stage 整个写域：第一次之前的树没有任何本 run 的提交担保；`Nothing` 出现在 run 的第一道 checkpoint：那时还没有调用跑过。**失败的调用并入 `Domain`**：`ToolBench::invoke` 答 `Err` 时没有 `wrote`，而工具可能写到一半才失败，它自己对写了什么的说法不再可信；lane 于是把 `Domain` 并进去，下一次 checkpoint stage 整个写域。只丢掉它、留下同波其他调用的 `Paths`，会让那半截写不进任何提交。**被否**：`RunHooks::checkpoint` 收一个范围参数——run 驱动拿不到工具的 `Effect`，这个参数只能由 lane 填，等于把同一个并集在两层各拼一次。
- **调用可能不可能写，由 `RunHooks::writes` 答**：`ToolDef` 只有名字、描述与 schema，`Effect` 住 `ToolBench` 的注册表里，所以 lane 在把 bench 借给 `invoke` 之前取出 `ToolBench::declared_writes`（名字到 `Writes::of(effect)` 的表），`writes` 查这张表。它按声明答，不按参数答：判定发生在波跑之前，而 `Tool::writes` 读的是一条跑完的调用。**被否**：`RunPlan` 带名字到 `Effect` 的表——`RunPlan` 是冻结的 run 描述，进账本的重放读它，而工具的 `Effect` 是 bench 注册时的事实，不该在两处各记一份。
- **否决「空波一律跳过」**：结束回合的空波前那次 checkpoint，是把上一波的写带进提交的唯一时机；跳过它，run 写下的文件就没有任何提交持有。
- **否决「每波都 checkpoint」**：一个没跑过任何调用的 run，提交的是一棵没变的树，却多付一次 stage 与 commit。

### 8-46 runtime::bench::outside（形状 1 判定；**外来内容进 run 的唯一入口**）

```rust
// crates/runtime/src/bench/outside.rs
pub(super) fn entered(effect: &Effect, taint: &TaintSet) -> Result<TaintSet, AxError>;  // 一次答案进门后 run 的 taint
```

- **判定表**（对 `Effect` 穷尽，无通配臂）：`Connector { label }` → 并入 `mcp:<label>`；`Egress` → 并入 `web`；`AttachUserBrowser` → 并入 `web:browser`；`Read`／`Write`／`Spawn`／`Govern`／`Spend` → 原样返回。新增一种 `Effect` 不回答这张表就不编译。
- **调用点只有一个**：`ToolBench::invoke` 在工具答出 `Ok` 之后、把答案交还模型之前，用 `entered` 的返回值替换 bench 的 taint。之后同一 run 的每扇门（`command`／`reach`／`undoable`）读到的都是长大后的集合，所以读过一段 MCP 回答或网页的 run 再调 exec，由 `kernel::gate::command` 答 `E_TAINTED_ACTION`。失败的调用不并入：没有内容进门。来源标签为空时答 `E_CONFIG_INVALID`，这个错误就是这次调用被记下的答案，模型读不到那段内容。
- **为什么按 `Effect` 判，而不是让每个工具自报**：`Effect` 已经是工具注册时声明的「这次调用伸向哪里」，门按它分派；来源再让工具另报一次，就是同一事实的第二份定义，漏报的工具会把外来内容当成内生数据放进来。
- **为什么是一个函数**：外来内容在 run 里要过的每一道手续（今天是标 taint，之后是密钥托管的入站扫描）都挂在这同一处，第二个消费者改这一个函数，不另开入口。
- **未定**：他楼文件（`read` 经 read bound 落进另一栋楼的路径）今天的 `Effect` 是 `Read`，这张表因此不标它；要标它，需要 `GateSubject::Path` 在 bench 里能判出「不在本楼」，证据是一条读他楼文件后 exec 被拒的 bench 测试。run 起点的 taint 是 sprawling 的 `Assignment.taint: kernel::TaintSet`，由 `Unasked::taint` 给出（一次 arrival 标为 `arrival:<source>`），原样放上 bench。

### 8-47 runtime::conversation：已发出的消息不再被改写，空回复之后的工具结果除外（形状 2 值类型）

```rust
impl Conversation {
    pub fn mark_sent(&mut self);   // 本次组装发出了 messages() 的全部；之后到达的 user 文本不再并入其中任何一条
}
```

- **规则**：`Conversation` 记下上次组装发出了几条消息（`sent`）。user 文本（steer、提醒）只并入**尚未发出**的最后一条 User 消息；最后一条 User 消息已经发出时，文本进一个待投槽（`held`），由下一次 `push_tool_results` 接在这一波结果之后，即词汇表里 Steer 的落点「下一份工具结果的末尾」。待投槽不在 `messages()` 里，所以它永远不会出现在一条它到达之前就已组好的请求中。
- **调用点**：活的 run 在 `Turn::assemble` 答出 `Advanced` 之后调一次（`run::lifecycle`）；`fork` 在读到本 run 的 `prompt_shape_compared` 时调一次：`prompt_assembled` 每个 run 只写一次（8-39），而 `prompt_shape_compared` 是每一回合组装之后紧接着写的那一行。两边的标记来自同一个事实（这一回合的请求组好了），所以分支按同一规则重放出同样的字节。
- **理由**：`BeforeCall`／`BeforeWave`／`BeforeToolCall`／`BeforeSpawn` 四个安全点都在组装之后，那时窗口最后一条仍是刚随请求发出的 User 消息。把 steer 并进去，下一次请求里 steer 排在一条没读过它的助手回复之前：模型看到的时间顺序是假的，而且已发出消息的字节变了，provider 的前缀缓存从这条消息起全部失效。
- **字节**：`tools/fixtures/golden-p0` 的剧本在第 0 回合收到 steer，它第二次请求的 run 区域在工具结果之后带着这条 steer。
- **例外：空回复之后的工具结果**：一条没有内容的回复不推助手消息，所以随后的 `push_tool_results` 碰到的最后一条仍是已发出的 User 消息。结果和待投文字并进这条消息，它的字节因此变了，provider 的前缀缓存从这条消息起失效。这里接受改写，因为另一条路是在它之后另开一条 User 消息，即被否的①：两条相邻的 User 消息是入口不变量要排除的形状。时间顺序仍然是真的：这条消息之后没有模型读过的回复。改写之后这条消息重新算作未发出（`sent` 退到它之前），所以在下一次组装之前到达的 steer 并进它，排在这批结果之后，不再多等一波。
- **一条回复没有任何调用时**：run 就此结束（8-37），待投文字不再有下一次组装；它已由 `steer_received` 入账，账本仍是它的来历。
- **fork 的切点落在一波之内时**：这一波整波丢弃（半个交换没有 provider 接受），分支只继承 `messages()`，待投槽里的文字不随分支走。待投文字只在「组装之后、这一波结果之前」存在，所以它针对的正是被丢弃的那一波；分支从没看到那一波，把它接到分支的第一条消息里，模型会读到一句指向不存在的上下文的话。这段文字已由 `steer_received` 入账，母 run 的账本仍是它的来历。被否：让 `Inherited` 带上待投文字——分支的首条 User 消息会以一句针对别人那一波的 steer 开头。
- **被否**：①在已发出的 User 消息之后另开一条 User 消息——两条相邻的 User 消息正是本模块入口不变量要排除的形状；②由执行器在工具结果之后再调一次 `push_steer`——待投状态会住在 `Conversation` 之外，`fork` 要复刻第二份同样的记忆，两个家会漂移。

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
- **载荷**：`report` 写 kernel 的 `HarnessReported`，`conclude` 写 `HarnessAnswered`（kernel-SPEC §8-4，两者都是 record-only）；`cancel_received` 的载荷是空对象，与回合里的那一条相同，截断的缘由不进账本（kernel 事件表不为它加键），由冻结的 `Limit` 与 `Cancelled` 分开。检查点的载荷由调用方从 `storage::Checkpoint::wave_pre` 取来，本 crate 不开仓库，理由与 `RunHooks::checkpoint` 相同。
- **失败**：任何一行写不进账本，原错误向上抛，run 停在它写到的那一行；这与 `Run::freeze` 自己的 append 失败时相同：账本自身就是受害者时没有真实的东西可写。
