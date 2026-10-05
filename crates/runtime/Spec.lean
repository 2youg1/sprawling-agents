-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import crates.runtime.spec.Backlog
import crates.runtime.spec.Bench
import crates.runtime.spec.Catalog
import crates.runtime.spec.Clock
import crates.runtime.spec.Compaction
import crates.runtime.spec.Conversation
import crates.runtime.spec.Diagnostics
import crates.runtime.spec.Digest
import crates.runtime.spec.Elision
import crates.runtime.spec.Fork
import crates.runtime.spec.Handoff
import crates.runtime.spec.Mode
import crates.runtime.spec.Offload
import crates.runtime.spec.Pipeline
import crates.runtime.spec.PolicyTake
import crates.runtime.spec.Prefix
import crates.runtime.spec.Reminder
import crates.runtime.spec.Replay
import crates.runtime.spec.Run
import crates.runtime.spec.Run.Checkpoint
import crates.runtime.spec.Sandbox
import crates.runtime.spec.Sieve
import crates.runtime.spec.Tools
import crates.runtime.spec.Tools.BoundReader
import crates.runtime.spec.Tools.ChosenPath
import crates.runtime.spec.Tools.Exec
import crates.runtime.spec.Tools.Exec.Container
import crates.runtime.spec.Tools.Exec.NativeMacos
import crates.runtime.spec.Tools.Read
import crates.runtime.spec.Tools.Search
import crates.runtime.spec.Tools.Succeed
import crates.runtime.spec.Transcript
import crates.runtime.spec.Turn
import crates.runtime.spec.Turn.Durability
import crates.runtime.spec.Turn.Recovery
import crates.runtime.spec.Turn.Speculation
import crates.runtime.spec.Watchdog

/-! # runtime 的规格

`sprawling-runtime`（库名 `runtime`，目录 `crates/runtime`）是一次 run 从派发到冻结的全部：回合的四相与取消边界、冻结前缀、工具面与它们的读界、工具结果的打包与压缩、分叉与离线重演、时钟与打戳。

本文件是 crate 的规格入口，分部在 `spec/` 下，布局见 ARCHITECTURE.md §11「Specifications in Lean」。接口一节一节写在规定它的那个模块的分部里，每一节保留它的标签 §8-n，别处引作 `crates/runtime/Spec.lean §8-n`；本文件 §8 列出每个标签住在哪个分部。决定写作 `D<n>`，放在它所管的声明正上方，或它所管主题的那个分部的末尾，别处引作 `runtime D<n>`；D1 到 D16 沿用这份规格在 Markdown 时 §12 的条目号，D12 空着，§12 末尾列出每条住在哪里。

能写成定理的规则在分部里证明，Lean 模型是「必须守住哪些性质」的权威，Rust 代码是「怎样守住」的权威：回合的边界（`spec/Turn.lean`）、开头只读段的提前起跑（`spec/Turn/Speculation.lean`）、一个回合付几道落盘屏障（`spec/Turn/Durability.lean`）、run 的唯一出口与结局判定（`spec/Run.lean`）、一波之前立不立 checkpoint（`spec/Run/Checkpoint.lean`）、分叉的前缀、切点与开篇写法（`spec/Fork.lean`）、打戳与按 UTC 选一段时间（`spec/Clock.lean`）、合并时的准入（`spec/Mode.lean`）、模型选路的判定（`spec/Tools/ChosenPath.lean`）与按字节读的门（`spec/Tools/BoundReader.lean`）。其余分部只有节注释：它们写的是接口的形状、取舍与被否的备选，由 Rust 的类型、trybuild 反例与各模块旁的测试守住（§16）。
-/

/-! ## 1 需求分解

| 模块 | 一句话 |
|---|---|
| `replay` | 离线重演验链：EventRef 第二铸造点；A2 的执行体 |
| `fork` | 分叉前缀：母 Run 事件序列到节点为止的逐字节前缀；A19 的执行体 |
| `turn` | 回合 typestate 四相＋取消边界；形状 5；相内中断编译不过 |
| `prefix` | FrozenSegment 四段＋分段哈希＋易变类型隔离；形状 5＋2 |
| `handoff` | 五段构造点＋resume 消费 Handoff 产新 Run 种子；形状 2 |
| 完备化 | prefix 四段全量（封顶＋截断标注＋跨段去重＋跳过入账）＋断点 ≤4＋Steer 边界消费＋窗口组装入 Assembling 相 |
| `pipeline`＋`offload` | 结果信封三附件＋offload 四不变量（独占有损可还原）＋截断定序 |
| `clock`＋`catalog`＋`mode` | ISO UTC 的唯一拼法＋ClockStamp 与它的发放规则＋ClockReading（§8-10、§8-53）＋渐进披露三类条目＋截断锁的三档：常驻核心、至多 1 KiB 的休眠索引、`describe` 与 `call` 两扇门（§8-60、§8-61）＋两个 mode 的目录行与合并时的准入（§8-54）＋会话中换运行策略的策略格（§8-62） |
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

每一行的模块由 `spec/` 下同名的分部规定（§8 的表）；表里的 A2、A4、A7、A10、A15、A18、A19 是 §2 的验收项。
-/

/-! ## 2 验收标准

- A2 演示：对 jsonl 落盘目录与内存行序列各跑一次 verify，逐事件验 prev 链与 seq 连续；任一字节被篡改即拒并报行号。
- A19 演示：从任一节点取分叉前缀，与母 Run 原始行 0..=at_seq 逐字节相同；`at_seq` 越界＝`E_INVALID_ARGS`（恒不静默截到末尾）；母序列不因分叉改变。
- 未知 kind：无 `ig:true` 即拒（方向语义：更新的写方）；带 `ig:true` 的行跳过类型化解读但链照验。
- A4——同一 PrefixPlan 两次 build 逐字节同（golden）；A15——prompt_assembled 载荷＋同源文档经 `rebuild_prefix` 重建，四段哈希逐段相同。
- A7——offload 往返（替代体≤原件且≤上限；循 rest_path 续读与循 CAS 取回字节一致；外部清理后自 CAS 重物化字节一致；命中既有哈希直引原 Locator）。
- A18 零字节——granularity=Off 时打包输出与未接 clock 特性逐字节相同。
- A10 三断言结论书——fuel 内成功／未授能力被拒／fuel 耗尽中断（真 wasmtime 上）。
- L0×失败注入矩阵；A6 双守（Done 恒携 kind 合法证据，运行时纵深校验）；A8（到限恒 `Completion::Limit`，恒不记完成）。

分部里的定理是模型对性质的证明：

- `spec/Turn.lean`：没有人喊停的一波按调用序给每条调用记下 `tool_called` 与 `tool_result`（`an_uninterrupted_wave_accounts_every_call`）；取消落在第 k 条调用之前，账本上恰是前 k 条调用的两行再一条 `cancel_received`（`a_cancel_before_a_call_accounts_exactly_the_calls_before_it`），被取消的波以它的 `cancel_received` 结束（`a_cancelled_wave_ends_with_its_cancel`）；Steer 不结束回合（`a_steer_never_ends_a_turn`）。
- `spec/Turn/Speculation.lean`：推测结果不是事件，Ledger 顺序即串行顺序。
- `spec/Turn/Durability.lean`：只读调用不等落盘、写调用意图先落盘时，任一崩溃点上对外效果之前的记录都已落盘，写动手时它的 `tool_called` 已在盘上，盘上的记录是逐条落盘的参照次序的前缀，`EventRef` 只给已落盘的记录；一个回合 `1 + 写调用数` 道屏障，参照是 `2 + 2 × 调用数`。
- `spec/Run.lean`：`handoff_written`＋`run_frozen` 是唯一出口，`run_frozen` 只写一次（`freeze_is_the_only_exit`）；`Done` 要说了话、又不是停在输出上限上（`done_needs_words_and_no_ceiling`）；收尾两行相隔一毫秒，时钟到顶时拒绝（`run_frozen_follows_its_handoff_by_one_millisecond`、`a_clock_at_its_ceiling_cannot_close`）。
- `spec/Run/Checkpoint.lean`：判定表恰好在上一次提交之后跑过可能写的一波、或还没有提交过而这一波可能写时立 checkpoint（`stages_exactly_when_needed`、`every_wave_stages_exactly_when_needed`）。
- `spec/Fork.lean`：分叉点过了尾就拒绝，前缀恰是母序列的头 `at_seq + 1` 行（A19）；切点不晚于要求的那一行、能切、并且只退到必须退的地方（`the_cut_never_splits_a_wave`、`retreat_goes_back_no_further_than_needed`）；只有 `FromJob` 被改写（`only_from_job_is_rewritten`）。
- `spec/Clock.lean`：`Off` 什么都不带（A18）；第一条结果与每一条 `Timestamped` 结果都带戳；`Timeless` 的结果一个桶里至多一次（`a_timeless_result_is_stamped_at_most_once_per_bucket`）；矛盾的区间被拒，相邻的两段不重不漏。
- `spec/Mode.lean`：合并当且仅当常规落地且证据在场（`lands_exactly_with_its_evidence`）。
- `spec/Tools/ChosenPath.lean`：准入的地址读得通、不在保留区里、读界开着；保留区在读界被问之前拒绝；`land` 交回的真实位置同样被判过；只判字面地址有反例（`judging_only_the_written_address_lets_a_link_out`）。
- `spec/Tools/BoundReader.lean`：一个块只在为它存过、读界开着的楼里读得到；没有来源的块被拒；门对路径的拒绝与 `read` 同码；按路径打开的文件是在它落下的地方被判过、判定时在场的那一个。

每个模型都带一个可实现的正常路径（第一条结果带戳、没有人喊停的一波、`a_fork_is_the_mothers_first_lines`、Lean 里的 `example`），所以这些保证不是从一个无法满足的前提推出来的。生产实现与模型的对应由 §16 列出的 Rust 测试检查；一条 Lean 定理证明的是模型，不是 Rust。
-/

/-! ## 3 假设与歧义

1. **verify 的规范复验**：v1 无升级器链，故对每行断言 `canonical_line(parse_line(raw)) == raw`（写方规范性质）。未来 v>1 经升级器读入后此断言只对原版字节成立——届时随升级器一并改约（本文更新）。
2. **fork 的 run_forked 落账**：事件写入母城 Ledger 由调用方（runtime 回合层／citysim）执行；fork 只产 EventDraft 与前缀，不持 Ledger 句柄——保持纯函数形。
3. **同一套重建器**：A15 与 A19 共用 verify 输出；重建器＝verified 行序列本身。
5. **命令结果的戳是答复时刻**：回合在调用工具面的 `account` 之前读答复时刻（§8-15、D8），所以工具面打戳时 `ClockReading` 里最新的就是它，戳的秒数等于这条调用 `tool_result` 的 `t`；`turn::tests::concurrent` 的 `a_stamp_the_face_reads_is_the_moment_its_answer_records` 在串行与开头只读段两条路上钉住这一点，`accounting::worker::driving::tests::sieving` 的 `a_served_command_result_ends_with_the_second_its_call_answered` 在一次真实派活上比同一个 `t`。
4. **前缀续期未接线**：`prefix::warmth` 的 `Warmed` 与记账已在（§8-4-2），而 run 结束后按 `next_due` 醒来发续期的那条循环还没有；接上它要先定续期的 usage 记成哪一种事件。
6. **`contract_kept` 的证据今天没人量。** 城读不出一次翻新有没有动到可观察的契约，装配层把 `Produced.contract_moved` 恒填 `false`，所以选了 `contract_kept` 的 run 在合并时恒放行（§8-54）。要让这一要求真的拒，得有一个读得出契约的量具（例如 run 前后同一组对外测试的结果对照）把它填进 `Produced`；判定它的证据是一次动了对外行为、测试仍绿的翻新在 citysim 里被放行。
7. **`Create` 管不到楼的 MCP 工具。** 一个 MCP server 是楼自己声明的外部进程，它写不写文件、写在哪里，城看不见（§8-55 只覆盖城自己的写路径：edit、exec 与链接）。候选是 `Create` 下不挂载声明了写效果的连接器，或只挂载声明只读的；判定它的证据是一个会写文件的连接器在 `Create` 的 run 里改动了已有文件。

模型自己的假设写在各分部的定理假设里，不写成公理：路径判定把 kernel 的文法、保留区与装配层的读界当参数（`spec/Tools/ChosenPath.lean` 的 `Rules`），不重述它们；`within_city` 对绝对路径的换算（D4）与打开之后的 `still_judged` 复核不在模型里，前者是平台的事实，后者防的是判定与打开之间换进来的链接，由 `tools::read::tests::doors` 在真实文件系统上检查。打戳的模型不带时区行，`iso` 与 `parse_iso` 互逆由 `clock` 的测试逐值检查，历法本身没有在 Lean 里证明。
-/

/-! ## 4 现状分析

verify 为 O(n) 全量；消费面（测试/夹具/citysim）规模千行级，无性能议题；seq→偏移索引归 storage::index。
-/

/-! ## 5 权威信源

Fork 三规则；重放/分叉/幂等；at_seq 越界、未知 kind、崩溃恢复行；`crates/kernel/Spec.lean` §8-4/§8-9；`crates/storage/Spec.lean` §8-1。
-/

/-! ## 6 命名统一

**跨 crate 类型住处**：`kernel` 的门／计划／脊／事件／错误／弃置／秘密七面已切目录（如 `error::shape::AxError`）；本 crate 经 `kernel` 顶层重导出引用，公共拼写不变，住处是 kernel 内政（`storage::checkpoint::Checkpoint` 住 `checkpoint` 同例）。

replay、verify、VerifiedLedger、VerifiedLine、fork prefix、`at_seq`。不引入「重播/回放/复演」等同义词。

Lean 里的名字与 Rust 的对应：

- `Runtime.Run.Checkpoint.for_wave`／`record_wave` ↔ `CheckpointPolicy::for_wave`／`record_wave`；`step` ↔ `run::lifecycle` 的 `advance` 每一波先判后记的那两行；`Truth` 与 `tracks` 是模型里树的真相，Rust 没有对应。
- `Runtime.Clock.observe` ↔ `StampGate::observe`，`due` ↔ 其中那一个布尔式；`UtcSpan.contains` 的 `moment` ↔ Rust 的参数 `at`，`«until»` ↔ `until`（两者在 Lean 里是关键字）。
- `Runtime.Fork.«prefix»` ↔ `fork::prefix`；`cut`、`retreat` ↔ `fold_run` 维护的开着的一波与 `Inherited::at`；`rebuilt` ↔ `fork::rebuilt`。
- `Runtime.Turn.consume_boundary` ↔ `Turn::consume_boundary`；`wave` ↔ `Turn<ToolWave>::execute_concurrent` 在每条调用之前问 `still_going` 的那一部分；`turn` ↔ `run::lifecycle` 的 `advance` 里一个回合的四相；`measured` ↔ `turn::ledger` 里带读数的四个变体。
- `Runtime.Run.concluded` ↔ `lifecycle::concluded`；`loop`、`drive` ↔ `run::drive`；`close` ↔ `Charter::close`。
- `Runtime.Mode.admits`／`admits_evidence` ↔ `mode::admits`／`admits_evidence`；`Refusal` 的每个构造子 ↔ 一对 `because`／`alternative`。
- `Runtime.Tools.ChosenPath.admit`／`land` ↔ `chosen_path::admit`／`land`；`judge` 是 `admit` 在解析之后的两道；`Landing` ↔ `real_location` 的城内与城外。
- `Runtime.Tools.BoundReader.«open»` ↔ `BoundReader::open`（`open` 是 Lean 关键字）；`judged_at` ↔ `read::locator::judged_at`，`first_open` 是其中的 `find`。
-/

/-! ## 7 模块边界

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
-/

/-! ## 8 接口先行

每一节的接口写在规定它的模块的分部里。标签照旧，按标签找分部：

| 标签 | 分部 |
|---|---|
| 8-1 | `crates/runtime/spec/Replay.lean` |
| 8-2 | `crates/runtime/spec/Fork.lean` |
| 8-3 | `crates/runtime/spec/Turn.lean` |
| 8-4 | `crates/runtime/spec/Prefix.lean` |
| 8-4-1 | `crates/runtime/spec/Prefix.lean` |
| 8-4-2 | `crates/runtime/spec/Prefix.lean` |
| 8-5 | `crates/runtime/spec/Handoff.lean` |
| 8-6 | `crates/runtime/spec/Turn.lean` |
| 8-7 | `crates/runtime/spec/Pipeline.lean` |
| 8-8 | `crates/runtime/spec/Offload.lean` |
| 8-9 | `crates/runtime/spec/Watchdog.lean` |
| 8-10 | `crates/runtime/spec/Clock.lean` |
| 8-53 | `crates/runtime/spec/Clock.lean` |
| 8-57 | `crates/runtime/spec/Clock.lean` |
| 8-58 | `crates/runtime/spec/Fork.lean` |
| 8-11 | `crates/runtime/spec/Catalog.lean` |
| 8-60 | `crates/runtime/spec/Catalog.lean` |
| 8-61 | `crates/runtime/spec/Catalog.lean` |
| 8-62 | `crates/runtime/spec/PolicyTake.lean` |
| 8-12 | `crates/runtime/spec/Mode.lean` |
| 8-12b | `crates/runtime/spec/Mode.lean` |
| 8-54 | `crates/runtime/spec/Mode.lean` |
| 8-55 | `crates/runtime/spec/Tools.lean` |
| 8-56 | `crates/runtime/spec/Run.lean` |
| 8-13 | `crates/runtime/spec/Sandbox.lean` |
| 8-13-2 | `crates/runtime/spec/Tools/Exec.lean` |
| 8-13-3 | `crates/runtime/spec/Tools/Exec.lean` |
| 8-14 | `crates/runtime/spec/Tools.lean` |
| 8-15 | `crates/runtime/spec/Run.lean` |
| 8-16 | `crates/runtime/spec/Digest.lean` |
| 8-17 | `crates/runtime/spec/Diagnostics.lean` |
| 8-15-1 | `crates/runtime/spec/Run.lean` |
| 8-18 | `crates/runtime/spec/Turn.lean` |
| 8-19 | `crates/runtime/spec/Bench.lean` |
| 8-20 | `crates/runtime/spec/Replay.lean` |
| 8-21 | `crates/runtime/spec/Prefix.lean` |
| 8-22 | `crates/runtime/spec/Tools.lean` |
| 8-23 | `crates/runtime/spec/Run.lean` |
| 8-24 | `crates/runtime/spec/Digest.lean` |
| 8-25 | `crates/runtime/spec/Sandbox.lean` |
| 8-26 | `crates/runtime/spec/Tools/Exec.lean` |
| 8-27 | `crates/runtime/spec/Sieve.lean` |
| 8-27-10 | `crates/runtime/spec/Pipeline.lean` |
| 8-28 | `crates/runtime/spec/Backlog.lean` |
| 8-29 | `crates/runtime/spec/Tools/Read.lean` |
| 8-30 | `crates/runtime/spec/Tools/Search.lean` |
| 8-30-1 | `crates/runtime/spec/Tools/ChosenPath.lean` |
| 8-30-2 | `crates/runtime/spec/Tools/Search.lean` |
| 8-59 | `crates/runtime/spec/Tools/BoundReader.lean` |
| 8-31 | `crates/runtime/spec/Tools/Exec.lean` |
| 8-32 | `crates/runtime/spec/Transcript.lean` |
| 8-33 | `crates/runtime/spec/Tools/Succeed.lean` |
| 8-34 | `crates/runtime/spec/Reminder.lean` |
| 8-48 | `crates/runtime/spec/Run.lean` |
| 8-37 | `crates/runtime/spec/Run.lean` |
| 8-36 | `crates/runtime/spec/Tools.lean` |
| 8-35 | `crates/runtime/spec/Bench.lean` |
| 8-38 | `crates/runtime/spec/Diagnostics.lean` |
| 8-39 | `crates/runtime/spec/Prefix.lean` |
| 8-40 | `crates/runtime/spec/Backlog.lean` |
| 8-41 | `crates/runtime/spec/Turn.lean` |
| 8-42 | `crates/runtime/spec/Elision.lean` |
| 8-49 | `crates/runtime/spec/Turn/Recovery.lean` |
| 8-50 | `crates/runtime/spec/Turn/Recovery.lean` |
| 8-51 | `crates/runtime/spec/Turn.lean` |
| 8-43 | `crates/runtime/spec/Watchdog.lean` |
| 8-44 | `crates/runtime/spec/Compaction.lean` |
| 8-45 | `crates/runtime/spec/Run/Checkpoint.lean` |
| 8-46 | `crates/runtime/spec/Bench.lean` |
| 8-47 | `crates/runtime/spec/Conversation.lean` |
| 8-47-1 | `crates/runtime/spec/Conversation.lean` |
| 8-47-2 | `crates/runtime/spec/Conversation.lean` |
| 8-52 | `crates/runtime/spec/Run.lean` |
-/

/-! ## 9 工作流程

`just replay <log>` → `sprawling replay` → `storage::audit_chain` → 全绿报行数与 tail seq，违规报 three-part。citysim 检查器与 A19 测试直接调 `verify_lines`／`prefix`。

回合内的流程是 `spec/Turn.lean` 的 `turn`：组装、调用、工具波、收尾，每一相先消费它的边界；run 的流程是 `spec/Run.lean` 的 `drive`：开篇两行、回合直到一个结局、收尾两行。
-/

/-! ## 10 实现逻辑

envelope 探查与全解共用 kernel 的解析（Value 探查仅取五键，不建第二记录类型）；行号从 1 计（人读）；错误 recovery 字段给「重放同一夹具于更新版本」或「检查介质」两句可执行建议。

### 两个设计

**turn 侧**：中断作相变入参（选中）vs 独立 `cancel()` 方法。后者表面更直观，但 cancel 方法可在任意持有点被调＝相内中断可表示，A9 退化成时序约定；选中方案把边界快照做成相变函数的形参，相内无入口，结构即断言。代价：调用方每相必须显式给 Interrupt（哪怕 None）——这个啰嗦是刎意的：它迫使执行器在每个边界问一次信号面。

**fork 侧：fork 消费 VerifiedLedger**（CLI 的 `fork` 与 `prefix` 仍如此；分支的对话重建只有 `inherited_indexed` 一扇门，理由见 8-2：账本唯一的写者打开账本时已验过，再验一次整链只为读一条 run 的几行，代价随历史长度增长）——分叉前必先验链，类型上把「从未验证的序列分叉」做成不可表示；分叉正确性与重放正确性因此是同一条断言。
**B（落选）：fork 直接吃原始行**（`prefix(lines: &[Vec<u8>], at_seq)`）——少一次验证成本，但打开「对损坏历史分叉」的路径，且 at_seq↔行号对应要自行重解 envelope＝第二解析权威。落选理由：验证成本 O(n) 在分叉频率下可忽略，而不变量 14（citysim 检查器）需要的正是 A 的类型保证。另 `verify_dir` 命名族落选：与 `verify_ledger_dir` 二选一，取后者（dir 一词泛滥易撞 S3 worktree 面）。

### 入窗的字节与它的代价

入窗字节大半由本 crate 决定：prefix 四段与断点（§8-4、§8-6）、catalog 的 Resident 行与二级披露（§8-11）、工具结果的信封与压缩（§8-7、§8-27）、上下文提醒（§8-34）与回合边界的压缩（§8-44）。replay/fork 是离线设施，其产物（分叉 Run 的入窗历史）经 prefix 组装间接入窗，自身不产生 prefix 字节。

### 工具的分段代价

每个工具在本 crate 里可测的有两段：经工具自己的面完成一次调用（路径判定、IO 与信封合为一段），以及结果入账前经 `redact` 的秘密扫描。仪表是 `redact::phases` 里的 `instrument_read_phases`、`instrument_write_and_edit_phases`、`instrument_exec_phases`、`instrument_search_phases`——放在 `redact` 旁，因为扫描是每个工具共有的那一段——由 `just bench` 以 `--lib` 跑，读数一行一工具、以微秒计；夹具约 64 KiB，形同城里工具结果的文本（哈希、oid、带凭据名的 hex 键、混合字母 token）。M2 的 gate、checkpoint 与屏障三段不在本 crate：它们在 accounting 的工具面与账本线程里，本 crate 的仪表读不到，那三段的仪表属于 accounting。三个平台上两段的含义相同；exec 在 Windows 上经 `cmd`，在 macOS 与 Linux 上经 `sh`。读数由版本中段的统一测量给出，下面每段只写剩余成本的来源、下界与下一步。

- **read**：剩余成本是读出所请求的字节与对这些字节的一次扫描。下界是两者各一遍：结果整段进账本，扫描必须看到结果的每个字节，不能按选区跳过。`redact` 接收载荷的所有权并原地改写（D37），零命中时扫描之外不再分配，剩下的就是这两遍。
- **write**（`edit` 以 `base_version` 为 `new` 新建文件）：剩余成本是写盘，加上结果回显的整份 diff 的扫描——新文件的 diff 就是全文，所以扫描量与文件同长。下界是写一遍、扫一遍回显。回显全文不能省：逐次的 diff 是还原一次改动的粒度（`tools::edit` 的模块说明），只回显长度与版本会让扫描量变成常数，却丢掉这次新建的还原依据。下一步因此在扫描器一侧，即 read 那一段说的不复制未命中的字符串。
- **edit**：剩余成本在调用一段：为核对 `base_version` 读整个文件并计算 blake3，再整份写回；回显只含改动的行，扫描几乎为零。下界是读一遍、哈希一遍、写一遍文件。下一步看中段读数里读与写哪个占大头；局部改写不改变「整份写回」的下界，因为版本是整份内容的哈希。
- **exec**：剩余成本是起一个进程与沙箱臂的准备，扫描只覆盖 stdout 与 stderr。下界是一次进程创建。下一步属于沙箱臂（`spec/Tools/Exec.lean`），不属于扫描。
- **search**：剩余成本是在读界内遍历楼的目录并逐文件匹配，扫描只覆盖命中行。下界是读界内每个文件读一遍。下一步是在多次搜索之间复用遍历结果；它需要一个随写入失效的索引，失效规则要先写进 `spec/Tools/Search.lean`。

扫描器本身是 kernel 的 `secret::scan`（`crates/kernel/spec/Secret.lean` §8-25）：形状表对字节只走一遍，hex 段先判标签再读熵，工作量随输入线性增长，由 kernel 的确定性计数测试把守。
-/

/-! ## 11 边界枚举

空序列（合法：VerifiedLedger 空，tail_seq=None；fork 于其上恒越界）；**目录存在但不含任何账本段**（在本模块合法且与空账本同形；人输入路径的拒绝在 CLI）；单行创世；`at_seq=FIRST`（前缀＝仅创世行）；`at_seq=tail_seq`（前缀＝全量）；ig:true 且 kind 已知（照常全解，ig 只授未知时的跳过权）；篡改中段一字节（链断于下一行报错）；两段夹具跨段验证（storage 读面已拼平）。
-/

/-! ## 12 错误处理

- 恢复层不造码（§8-49）：恢复段的失败恒是它拿到的那个类型化错误，可定义性随原码走；该层改变的只是「同样的请求再发一次」与「换一扇门再问一次」之间的选择。

- `E_INVALID_ARGS`（at_seq 越界）：不可定义掉——「从已冻结 Run 最后事件之后分叉」是用户可达输入；静默夹取是被明拒的替代。
- `E_LOG_VERSION_UNSUPPORTED`（v 判向＋未知 kind 无 ig）：不可定义掉——数据比二进制长寿。
- 链断/seq 洞/非规范字节：以 `E_CAS_CORRUPT` 报（存储完整性族；subject=行号与路径）——能否定义掉＝「介质位腐烂在设计边界外」，同 storage D4。

决定的条目与它们住的地方：

| 决定 | 标题 | 分部 |
|---|---|---|
| D1 | 定规：来源行不带摘要生产者指纹 | `crates/runtime/spec/Prefix.lean` |
| D2 | 定规：回合边界的压缩只在收尾边界换快照 | `crates/runtime/spec/Compaction.lean` |
| D3 | 定规：run 结束即终止它留下的后台命令 | `crates/runtime/spec/Backlog.lean` |
| D4 | 定规：一条路径算不算绝对路径，由本平台判定 | `crates/runtime/spec/Tools/ChosenPath.lean` |
| D5 | 一回合里等来的四个时刻各采各的，钟交给回合的账本门 | `crates/runtime/spec/Turn.lean` |
| D6 | 首个内容只数文字与推理，钟在回合里读 | `crates/runtime/spec/Turn/Recovery.lean` |
| D7 | 定规：run 的开篇与收尾归 `Charter`，harness run 不借 `RunPlan` | `crates/runtime/spec/Run.lean` |
| D8 | 戳从驱动最近的读数渲染，到秒，默认每分钟；回合在工具面打包之前读答复时刻 | `crates/runtime/spec/Clock.lean` |
| D9 | 城外书架上的 skill 由 catalog 携着正文交给 run | `crates/runtime/spec/Tools/Read.lean` |
| D10 | 沙箱副本按工具一份、每条命令前同步，而不是每条命令新建一份 | `crates/runtime/spec/Tools/Exec.lean` |
| D11 | 「只新建」下 exec 只在副本里跑，不开放 host | `crates/runtime/spec/Tools/Exec.lean` |
| D13 | 时刻只有 `iso` 写出的那一种拼法可以读回，时间段是半开的、两端在构造时核对 | `crates/runtime/spec/Clock.lean` |
| D14 | 开篇的写法记在 `run_started` 上，`FromJob` 的分支仍改写第一条消息 | `crates/runtime/spec/Fork.lean` |
| D15 | 连接器把声音块存进 CAS，交出的是 locator 而不是附件 | `crates/runtime/spec/Pipeline.lean` |
| D16 | 按字节读的门在 runtime，交出一个 `Read` 与字节的来处 | `crates/runtime/spec/Tools/BoundReader.lean` |
| D19 | 休眠索引整份封顶 1 KiB，按字节、不按件 | `crates/runtime/spec/Catalog.lean` |
| D20 | 索引按稳定序贪心装填，先带提示、再只有名字、最后 `+N more` | `crates/runtime/spec/Catalog.lean` |
| D21 | 工具表在 session 里恒不变，第三档经 `describe` 与 `call` 走会话 | `crates/runtime/spec/Catalog.lean` |
| D22 | 搜索是确定的关键词排序，skill 正文仍只经 `read` | `crates/runtime/spec/Catalog.lean` |
| D23 | 命令行程序不另立目录 | `crates/runtime/spec/Catalog.lean` |
| D24 | 只读工具不在执行前等落盘，写调用的意图先落盘，一波一道屏障 | `crates/runtime/spec/Turn.lean`（§8-3），性质在 `crates/runtime/spec/Turn/Durability.lean` |
| D25 | 会话中可改运行策略之后，常驻核心是各 mode 核心的并集 | `crates/runtime/spec/Catalog.lean` |
| D28 | 会话中换运行策略只由驱动循环在 `BeforeWave` 写进策略格，工具只读格 | `crates/runtime/spec/PolicyTake.lean` |
| D29 | 每个 run 的 job 按权重分 CPU、设作业级内存上限；子进程在 macOS 降到 utility，在 Linux 用 cgroup v2 或只用 nice | `crates/runtime/spec/Tools/Exec.lean` |
| D30 | shell 默认仍是平台的 shell，一栋楼可以换成 pwsh 7，exec 的失败按 shell 从账本折出 | `crates/runtime/spec/Tools/Exec.lean` |
| D32 | 沙箱臂的调研表（SB0）与按平台的缺省臂、可选臂 | `crates/runtime/spec/Tools/Exec.lean` |
| D34 | 对话窗口每个 run 有字节预算，已发出的消息超出时移出进程、按 `Locator` 从 CAS 读回 | `crates/runtime/spec/Conversation.lean`（§8-47-1） |
| D36 | 攒下的记录跨过回合：`HeldLines` 归 `Run<Active>`、每回合的 `Journal` 借它，回合收尾不付屏障，`TurnReport` 的 `model_returned` 是 `Entry`、ref 在下一道屏障换出，一回合 `1 + 写调用数`、run 末尾一道，三个平台相同 | `crates/runtime/spec/Turn/Durability.lean` |
| D37 | `redact` 接收并交回载荷的所有权：零命中原样交回同一块分配，有命中只换命中的字符串，三个平台相同 | `crates/runtime/spec/Tools.lean`（runtime::redact） |
-/

/-! ## 13 依赖选型

kernel、storage（读面与 cas）；serde_json（envelope 探查）。dev：proptest、tempfile、trybuild、insta（prefix golden）。
`wasmtime`／`wasmtime-wasi = "48.0.3"`（feature `wasm` 内藏，钉版理由见 §8-13；wat 为 dev 依赖供 A10 模块）；`similar`？否——unified diff 自写最小形（edit 回显只需逐行对照，不引第三方 diff 库；被否理由：依赖面换一处 80 行纯函数，不值）。其余无新第三方（分段哈希经 kernel `B3Hash::digest`，不直依 blake3）。

规格本身不加依赖：分部只 import 工具链的库与本 crate 的分部（ARCHITECTURE.md §3 的 `depmap` 允许 runtime 引 kernel、storage、gateway 的分部，今天一个都不需要）。
-/

/-! ## 14 硬编码声明

无（行号计法与 recovery 文句不构成行为常量）。

两处 pub(crate) 数据面（改须本 SPEC 同集）：信封附件封顶 `ENVELOPE_ATTACH_MAX_BYTES=1024`（§8-7：附件与负载分账的断言界）；net_notice／truncation／offload 提示句三定句（ASCII，住 pipeline／offload 实现内，改句＝改入窗字节＝过本 SPEC）。
截断锁的三个常量住 `catalog`（改值＝改入窗字节＝过本 SPEC）：`DORMANT_INDEX_CEILING=1024`（整份休眠索引的上限，D19）、`HINT_MAX_BYTES=64`（索引里一件的提示截到多长，§8-60）、`DESCRIBE_HITS=8`（`describe` 一次列出几件候选，D22）；常驻核心的名单住 `mode::core_tools`（§8-60）。
一项常量读取（值与理由住 `kernel::consts_policy`，本文件不复写）：`EXCHANGE_BUDGET_BYTES`——回合 exchange 的入窗预算，`compaction::exchange` 是唯一读者。
-/

/-! ## 15 影响面

citysim 链检查器复用 verify_lines；bin `replay` 子命令接线；prefix 重建器消费 VerifiedLedger。
citysim 是 `run::drive` 的调用方（§8-15），assemble 的签名变动波及它；kernel::model 的 canonical 类型波及 ScriptModel；ToolBench 持有全部门判；E_TOOL_OUTCOME_UNKNOWN 补写面（replay 的 dangling 检测）供 resume 路径消费。
-/

/-! ## 16 测试与约束

单测：五步各拒绝分支＋ig 跳过；fork 越界；fork_draft 载荷形。proptest：对任意合法 draft 序列（经内存 Ledger 物化）verify 恒过；任意单字节翻转恒拒。A2/A19 演示测试入 crates/runtime/tests/。约束：clippy 零告警。
A4 golden（build_prefix 重跑逐字节同）；A15（rebuild_prefix 对拍）；A7 往返四断言；A18 零字节；watchdog 分级序；A10 三断言（feature `wasm` 下真 wasmtime＋WAT）；L0×失败注入矩阵（三臂×（正常／工具错／拒收））；ToolBench 门路由（Deny 回流／门的提问回流／dedup 先于副作用）。
回合边界的压缩（§8-44）：`compaction::exchange::tests` 三例（预算内全保留；超预算回复按 `plan` 裁、结构化结果整块保留；份额随 exchange 大小走）；`turn/tests/compaction` 两例（收尾边界才换快照、波中达阈值按全波分组一次压）；`fork/tests` 一例（分支继承的是压缩后的 exchange）；citysim `compaction` 两例（波中达阈值账本全字节保留、同剧本逐字节重放）。

形式化的义务由证明清偿：`lake build crates.runtime.Spec`（`just models` 在 `just check` 里构建全部规格），不留 `sorry`、`admit` 与 `axiom`，`cargo xtask gates spec` 检查这一点。模型与生产实现的对应由这些 Rust 测试检查，它们是行为比对，不是精化证明：

- 回合的边界：`turn::tests::phases`（`cancel_at_the_call_boundary_stops_before_any_model_bytes`、`steer_at_a_boundary_records_and_advances`、`a_wave_halted_between_two_calls_does_not_make_the_second`）。
- run 的出口与 checkpoint：`crates/runtime/tests/run_driver.rs`（`a_checkpoint_runs_before_the_wave_and_carries_the_turns_stamp`、`a_run_that_calls_nothing_puts_up_no_checkpoint`、`a_read_only_wave_puts_up_no_checkpoint`）与 `run::lifecycle::tests`。
- 分叉：`fork::tests`（`a_cut_inside_a_wave_moves_back_to_the_safe_point`、`a_cut_the_history_does_not_hold_is_refused`）、`fork::request_tests` 与 `crates/runtime/tests/replay_fork.rs`。
- 打戳与时间段：`clock` 的测试（`off_emits_never_even_for_timestamped_tools`、`first_result_emits_once_then_timeless_deduplicates_within_a_bucket`、`timestamped_emits_every_result_with_its_own_reading`、`a_span_keeps_its_start_and_leaves_out_its_end`、`a_span_with_no_moment_in_it_is_refused`）。
- 准入：`mode` 的测试，每种要求、每种落地各自的拒与放。
- 截断锁：`catalog::tests`（休眠索引不超过上限、截断只在放不下时发生、没准入的件零字节、常驻核心以外的工具不进工具表、`resolve_call` 的三种拒绝与换出的调用沿用原 `id`）、`tools::describe` 与 `tools::call` 的测试，以及工具面换调用的 `turn::tests::concurrent`。
- 路径与按字节读：`tools::chosen_path` 的测试、`tools::read::tests::doors` 与 `tools::bound_reader` 的测试（`the_door_refuses_what_read_refuses_with_the_same_code`）。

只有节注释的分部，其要求由类型、trybuild 反例（`crates/runtime/tests/trybuild.rs`）与 `cargo nextest run -p sprawling-runtime` 的各模块测试守住。
-/

/-! ## 17 文档关系

模块登记在 ARCHITECTURE 的模块图（`xtask modmap`）；canonical 类型的改动与 `crates/kernel/Spec.lean` §8-23/§8-24 同一变更集；runtime 的公开面即 `lib.rs` 的 `pub mod` 与根重导出。

- ARCHITECTURE.md §11「Specifications in Lean」：本规格的布局；它改了，分部的路径与 `architecture.toml` 里 runtime 各行的 `spec` 锚点一起重看。
- `architecture.toml` 的模块图：runtime 每一行的 `spec` 指向规定它的分部，`cargo xtask gates specalign` 检查锚点在盘上。
- `docs/glossary.md`：本规格用的词，`cargo xtask gates lexicon` 检查。
- kernel 的规格（`crates/kernel/Spec.lean`）：事件表、`Effect`、`Opening`、`RunPolicy` 与 `ReadVerdict` 的权威；它们改了，这里的模型与 §8 相应各节一起重看。
- storage 的规格（`crates/storage/Spec.lean`）：逐行检查 `LineCheck`、块的来源 `Cas::origins` 与写目标；gateway 的规格（`crates/gateway/Spec.lean`）：模型口、`AudioType` 与 `Recording`。
- `crates/agent_protocols/spec/Harness/Session.lean`：harness run 的次序与结局表（§8-52 引它），runtime 不 import 它，因为 `depmap` 不让 runtime 依赖 agent_protocols。
- 引本规格的其他规格与 rustdoc 写 `crates/runtime/Spec.lean §8-n` 或 `runtime D<n>`；一节换了分部，它的标签不变，引用不必改。
-/
