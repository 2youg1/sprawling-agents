-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import crates.sprawling.spec.Accounting.Effect
import crates.sprawling.spec.Accounting.PlanView
import crates.sprawling.spec.Accounting.Views
import crates.sprawling.spec.Accounting.Worker
import crates.sprawling.spec.Assembly
import crates.sprawling.spec.Assembly.ChainWatch
import crates.sprawling.spec.Assembly.Listening
import crates.sprawling.spec.BrowserBidi
import crates.sprawling.spec.Console
import crates.sprawling.spec.Doctor
import crates.sprawling.spec.Firstrun
import crates.sprawling.spec.Install
import crates.sprawling.spec.Keying
import crates.sprawling.spec.Main
import crates.sprawling.spec.Main.Exit
import crates.sprawling.spec.Main.Grammar
import crates.sprawling.spec.Monitor
import crates.sprawling.spec.Outside
import crates.sprawling.spec.Outside.Conduit
import crates.sprawling.spec.Serving
import crates.sprawling.spec.Serving.OutputRing
import crates.sprawling.spec.Serving.Memory
import crates.sprawling.spec.Serving.Placement
import crates.sprawling.spec.Serving.Placement.Plan
import crates.sprawling.spec.Serving.Standing
import crates.sprawling.spec.Supervising
import crates.sprawling.spec.WireClient

/-! # sprawling 的规格

`sprawling`（目录 `crates/sprawling`）是这座城唯一的二进制：命令行、装配根、serve 时的服务面、机器体检，以及嵌进二进制的客户端包。

本文件是 crate 的规格入口，分部在 `spec/` 下，布局见 ARCHITECTURE.md §11「Specifications in Lean」。接口一节一节写在规定它的那个模块的分部里，每一节保留它的标签 §8-n，别处引作 `crates/sprawling/Spec.lean §8-n`；本文件 §8 列出每个标签住在哪个分部。决定写作 `D<n>`，放在它所管主题的分部里，别处引作 `sprawling D<n>`；§12 末尾列出每条住在哪里。

能写成定理的规则在分部里证明：Lean 模型是「必须守住哪些性质」的权威，Rust 代码是「怎样守住」的权威（§2 列出每个模型证明了什么）。其余各节写接口的形状、取舍与被否的备选，由 Rust 的类型与各模块旁的测试守住（§16）。`spec/Accounting/` 下的分部写的是 `accounting` crate 的模块，为什么暂住这里见 D30。
-/

/-! ## 1 需求分解

`sprawling` 是这座城唯一的二进制，由五块组成：命令行（`bin::main`，动词表在 `main::verbs`，§8-89）；装配点（`bin::assembly`：造 `RunWorker`、把端口接上、墙钟的唯一采样点 `assembly::SystemClock`）；serve 时的服务面（`bin::serving`：门、进程日志、视图的折叠线程，§8-92）；读面（`accounting::views`）与机器体检（`bin::doctor`）；以及把客户端包嵌进二进制的 `build.rs`（§8-83）。

| 分部 | 它规定的模块 |
|---|---|
| `spec/Accounting/Effect.lean` | `accounting::effect` |
| `spec/Accounting/PlanView.lean` | `accounting::plan_view` |
| `spec/Accounting/Views.lean` | `accounting::views` |
| `spec/Accounting/Worker.lean` | `accounting::worker` |
| `spec/Assembly.lean` | `bin::assembly` |
| `spec/Assembly/ChainWatch.lean` | `bin::assembly::chain_watch` |
| `spec/Assembly/Listening.lean` | `bin::assembly::listening` |
| `spec/BrowserBidi.lean` | `bin::browser_bidi` |
| `spec/Console.lean` | `bin::console` |
| `spec/Doctor.lean` | `bin::doctor` |
| `spec/Firstrun.lean` | `bin::firstrun` |
| `spec/Install.lean` | `bin::install` |
| `spec/Keying.lean` | `bin::keying` |
| `spec/Main.lean` | `bin::main` |
| `spec/Main/Exit.lean` | `bin::main::exit` |
| `spec/Main/Grammar.lean` | `bin::main::grammar` |
| `spec/Monitor.lean` | `bin::monitor` |
| `spec/Outside.lean` | `bin::outside` |
| `spec/Outside/Conduit.lean` | `bin::outside::conduit` |
| `spec/Serving.lean` | `bin::serving` |
| `spec/Serving/Memory.lean` | 常驻内存：工作集清点、按字节计预算的缓存（`storage::resident`，`crates/storage/src/resident.rs`）、私有字节的平台读数 |
| `spec/Serving/OutputRing.lean` | `bin::serving::output_ring` |
| `spec/Serving/Placement.lean` | `bin::serving::placement` |
| `spec/Serving/Placement/Plan.lean` | `bin::serving::placement::plan` |
| `spec/Serving/Standing.lean` | `bin::serving::standing` |
| `spec/Supervising.lean` | `bin::supervising` |
| `spec/WireClient.lean` | `bin::wire_client` |
-/

/-! ## 2 验收标准

每节写出钉住它的测试。退出码是一张表（§8-103）：读不懂的命令行退 2，并给出最近的动词名。

分部里的定理是模型对性质的证明，每个模型都带 `example` 走到每一条分支，所以这些保证不是从一个无法满足的前提推出来的：

- `spec/Install.lean`：追加幂等、追加不遮挡、追加再移除回到原值、移除只动那一个目录（§8-9）。
- `spec/Main/Exit.lean`：五个退出码两两不同，`Unheard` 的三种原因各落到一个码（§8-103）。
- `spec/Main/Grammar.lean`：版本先于一切，帮助先于任何动词运行，`--` 之后的词不参与这两个判断，空行是首屏（§8-89）。
- `spec/Serving/Standing.lean`：降回是吸收态、窗口没关上不判、忙满一个窗口即判降回、设置为 `normal` 从不升档、平台拒绝之后不再试（§8-93）。
- `spec/Serving/Placement.lean`：任意的起动与退出序列上，一座一人、座位都在计划里、坐着的线程数不超过座位数；有空座位就给，拿不到座位只发生在每个座位都有人时；线程活着时座位不变，退出即交还（§8-93）。
- `spec/Serving/Placement/Plan.lean`：对每一张拓扑，放置计划只含本进程能用的处理器；只有一档（不论缓存）或拓扑不自洽时计划是空的；一个物理核至多一个座位；有几档时计划里没有最慢一档的处理器；同一张拓扑以任何顺序读进来，计划相同（§8-93）。
- `spec/Monitor.lean`：没人看不读也不留历史，看整页读整页，历史不超过 300 点、新的在最后（§8-94）。
- `spec/Supervising.lean`：人选的收口总是停下，重启带出的预算有界且都在窗口里，一分钟内第三次崩溃即 degraded（§8-109）。
- `spec/Serving/OutputRing.lean`：最新的一块总留着，每个 run 不超过上界或只剩一块，只从最旧的整块丢起（§8-115）。
- `spec/Serving/Memory.lean`：任意的放入、读、逐出与冻结序列上，常驻字节不超过预算，冻结的 run 名下没有项，每次读（包括读回被逐出的项）答出盘上的字节；按项数计的预算给出一条超过字节预算的序列（§8-173）。
- `spec/Assembly/ChainWatch.lean`：证明线程的四种结局各给出判定，只有完好才放行（§8-90）。
- `spec/Outside/Conduit.lean`：到城的只有放行的帧，只看的设备从不转发动手的动词，读不懂的帧与会话已结束时都不到城（§8-139）。
- `spec/Assembly/Listening.lean`：锁之前的拒绝一行不写，横幅只在 `listen` 成功之后印，第二个进程在写任何东西之前被拒（§8-88）。

生产实现与模型的对应由 §16 列出的 Rust 测试检查；一条 Lean 定理证明的是模型，不是 Rust。
-/

/-! ## 3 假设与歧义

评审楼的 worktree 按房间保留：`Site::place_tree` 以 `room-<地址 BLAKE3 摘要前 16 位十六进制>` 为名认领，同一房间的下一轮活取回上一轮留下的树（`crates/storage/Spec.lean` §8-9），不再每轮全量检出、再整目录删除；`RunWorker::over` 拿到账本写者后解开上一个写者留下的全部 worktree 锁。树按楼的 scope 领、按同一个 scope 立检查点和献出（`workbench::tree_scope` 是这一个 scope 的唯一定义）：再领一棵留着的树只检出 scope（`crates/storage/Spec.lean` §8-9），献出的 `Checkpoint::land` 只按 scope 暂存，于是合并进干线的提交只动 scope。一个房间的第一次放置先接管城的备树，只改名、不检出（`crates/storage/Spec.lean` §8-35，§8-145）；没有备树可接管时才全量检出，因为 `Worktree::add` 总做一次全量检出，git2 0.21 没有把 `git_worktree_add_options.checkout_options` 暴露成安全接口，而 `storage` 禁 `unsafe`。城的第一次放置总在这一类：那之前城没有提交，也就没有干线可备（读数在 §8-161）。上限只称一次检出、不称留着的树之和，定在 `crates/storage/Spec.lean` §8-9。树的放置与 MCP 缺表时的那次连接，连同 `lay_out_workbench`、`freeze_plan`，在驾驶这个 run 的 lane 里做（§8-113）。

不钉的构建（从 crates.io 的 `.crate` 构建、找不到三份工具链钉子文件）该不该在 develop 层列 `lean` 与 `zig` 两行，未定：从 crates.io 装这个二进制的人多半不开发这份代码，而 `Need` 不按构建来源分（§8-58）。今天两行照列，钉子为空时 `lean` 装 `stable`、`zig` 装 winget 给的那一版（§8-162）。判定它的证据是一次从 `.crate` 构建出的二进制在 Windows 上按页面的「安装」跑这两行。
-/

/-! D30 写 `accounting` 模块的节暂住 `spec/Accounting/` 下，等它们搬进 accounting 的分部

一百多个模块从 `sprawling` 搬进 `accounting` 时，写它们的节留在本 crate 的规格里，只把模块路径改成新的拼写。本 crate 迁到 Lean 时，这些节按它们写的那个 accounting 模块的顶层收进 `spec/Accounting/Worker.lean`、`spec/Accounting/Views.lean`、`spec/Accounting/Effect.lean` 与 `spec/Accounting/PlanView.lean`，标签不变，所以 accounting 分部末尾那几张「标签 → 模块」的表与别处的引用都仍然找得到它们。理由：两个 crate 的标签在 §8-17、§8-24 这些号上撞车，把节搬进 accounting 的分部就要给它们重新编号，并改写每一处引用；那是 accounting D15 记下的那一步，与格式转换分开做，转换就不夹带语义变化。被否的做法：转换时直接搬进 accounting 的分部（一次改动里既换格式又重编号）；把它们留在一份 Markdown 里（一个 crate 就有两份生效的规格）。重开的条件：accounting D15 那一步做完，`spec/Accounting/` 就删掉。
-/

/-! D34 车道不设上限：准备好的 run 立即得到一条 lane，放行只看内存与端点的并发名额

**决定**：`accounting::worker::pool` 不持有车道数，也没有配置键给它一个；一个准备好的 run 立即得到一条 lane（一个线程）。放行只问两件事：内存（`RESERVE_SHARE`，§8-46-3 的内存闸）与这次模型调用要去的端点的并发名额（`crates/gateway/Spec.lean` D17）。排队于是只发生在 provider 一处，在那里计数并显示；lane 大多阻塞在网络上，每条只多占一个线程栈。三个平台相同：线程栈用标准库的缺省大小；内存读数经 `bin::monitor::memory`（`sysinfo`），Windows 读 `GlobalMemoryStatusEx`，Linux 读 `/proc/meminfo` 的 `MemAvailable`，macOS 读 Mach 的 `vm_statistics64`，可用量的口径在 macOS 上较宽（§8-46-3）。

**理由**：User 定了车道不设上限（roadmap TP2）。写死的四条车道说自己等于一个 provider 准入模块的上限，而那个模块已经不在，于是第五个准备好的 run 在等一个与任何 provider 都无关的名额。

**被否**：①把车道数改成可配置：仍是一个与 provider 无关的闸，只是把选数的事交给人；②按 CPU 核数定车道：lane 的时间几乎都在等网络，核数不是它的约束。

**重开参数**：吞吐台（roadmap TP1）在 N = 64 时量到线程栈或调度本身成了等待的来源时，重议 lane 是否改为任务而不是线程。
-/

/-! ## 4 现状分析

各节的「本节接口的当前状态」写该接口还没做完的部分，本节不另列。
-/

/-! ## 5 权威信源

bin 子命令面；装配层是 Main；ARCHITECTURE.md §2（客户端嵌入链）与 `architecture.toml`（模块图里 bin 段那些条目）。
-/

/-! ## 6 命名统一

**跨 crate 类型住处**：本 crate 引用别的 crate 的类型，用那个 crate 公开的拼写：它的顶层重导出（如 `kernel::AxError`），或它声明为 `pub mod` 的模块路径（如 `kernel::layout::CityLayout`）；定义实际住在哪个更深的子模块（如 `AxError` 住 `error::shape`）是那个 crate 的内政，搬家不改这里的任何一行。`gateway` 与 `wire` 同理。
-/

/-! ## 7 模块边界

`main`：CLI 分发与呈现；`assembly`：唯一知情点，句柄/时钟/种子/spawn 注入处。
**本 crate 不做什么**：不含任何判定（判定住 kernel）；账本与内容仓库的写盘住 storage。
-/

/-! ## 8 接口先行

```rust
// bin::assembly
pub struct SystemClock;   // 墙钟的唯一采样点（clippy.toml 的 disallowed-methods 只在这里放行），生产的 accounting::Clock
```

每一节的标签与它住的分部：

| 标签 | 分部 |
|---|---|
| 8-2 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-2b | `crates/sprawling/spec/Main.lean` |
| 8-3 | `crates/sprawling/spec/Accounting/Views.lean` |
| 8-4 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-4b | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-4c | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-4d | `crates/sprawling/spec/Main.lean` |
| 8-4e | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-5 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-6 | `crates/sprawling/spec/Accounting/Views.lean` |
| 8-7 | `crates/sprawling/spec/Assembly.lean` |
| 8-8 | `crates/sprawling/spec/Firstrun.lean` |
| 8-9 | `crates/sprawling/spec/Install.lean` |
| 8-10 | `crates/sprawling/spec/WireClient.lean` |
| 8-11 | `crates/sprawling/spec/Console.lean` |
| 8-12 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-13 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-14 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-15 | `crates/sprawling/spec/Assembly.lean` |
| 8-16 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-17 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-18 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-19 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-20 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-21 | `crates/sprawling/spec/Console.lean` |
| 8-22 | `crates/sprawling/spec/Keying.lean` |
| 8-24 | `crates/sprawling/spec/Accounting/Effect.lean` |
| 8-25 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-26 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-27 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-28 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-29 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-30 | `crates/sprawling/spec/Accounting/Effect.lean` |
| 8-31 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-32 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-33 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-34 | `crates/sprawling/spec/Accounting/PlanView.lean` |
| 8-35 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-36 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-37 | `crates/sprawling/spec/Accounting/Views.lean` |
| 8-38 | `crates/sprawling/spec/Serving.lean` |
| 8-39 | `crates/sprawling/spec/Assembly.lean` |
| 8-40 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-41 | `crates/sprawling/spec/WireClient.lean` |
| 8-42 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-42-1 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-42-2 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-42-3 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-42-4 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-42-5 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-42-6 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-42-7 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-42-8 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-43 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-44 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-45 | `crates/sprawling/spec/BrowserBidi.lean` |
| 8-45-1 | `crates/sprawling/spec/BrowserBidi.lean` |
| 8-45-2 | `crates/sprawling/spec/BrowserBidi.lean` |
| 8-45-3 | `crates/sprawling/spec/BrowserBidi.lean` |
| 8-45-4 | `crates/sprawling/spec/BrowserBidi.lean` |
| 8-45-5 | `crates/sprawling/spec/BrowserBidi.lean` |
| 8-46 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-46-1 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-46-2 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-46-3 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-46-4 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-46-5 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-46-6 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-46-7 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-46-8 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-46-9 | `crates/sprawling/spec/Accounting/Views.lean` |
| 8-46-10 | `crates/sprawling/spec/Accounting/Views.lean` |
| 8-46-11 | `crates/sprawling/spec/Accounting/Views.lean` |
| 8-46-12 | `crates/sprawling/spec/Accounting/Views.lean` |
| 8-46-13 | `crates/sprawling/spec/Accounting/Views.lean` |
| 8-47 | `crates/sprawling/spec/Doctor.lean` |
| 8-48 | `crates/sprawling/spec/Doctor.lean` |
| 8-49 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-50 | `crates/sprawling/spec/Accounting/Views.lean` |
| 8-50-1 | `crates/sprawling/spec/Accounting/Views.lean` |
| 8-50-2 | `crates/sprawling/spec/Accounting/Views.lean` |
| 8-50-3 | `crates/sprawling/spec/Accounting/Views.lean` |
| 8-50-4 | `crates/sprawling/spec/Accounting/Views.lean` |
| 8-51 | `crates/sprawling/spec/Assembly.lean` |
| 8-52 | `crates/sprawling/spec/Accounting/Views.lean` |
| 8-53 | `crates/sprawling/spec/Accounting/Views.lean` |
| 8-54 | `crates/sprawling/spec/Doctor.lean` |
| 8-55 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-56 | `crates/sprawling/spec/Assembly.lean` |
| 8-57 | `crates/sprawling/spec/Doctor.lean` |
| 8-58 | `crates/sprawling/spec/Doctor.lean` |
| 8-59 | `crates/sprawling/spec/Doctor.lean` |
| 8-60 | `crates/sprawling/spec/Doctor.lean` |
| 8-61 | `crates/sprawling/spec/Doctor.lean` |
| 8-62 | `crates/sprawling/spec/Doctor.lean` |
| 8-63 | `crates/sprawling/spec/Doctor.lean` |
| 8-64 | `crates/sprawling/spec/Doctor.lean` |
| 8-65 | `crates/sprawling/spec/Doctor.lean` |
| 8-66 | `crates/sprawling/spec/Doctor.lean` |
| 8-67 | `crates/sprawling/spec/Doctor.lean` |
| 8-68 | `crates/sprawling/spec/Doctor.lean` |
| 8-69 | `crates/sprawling/spec/Doctor.lean` |
| 8-71 | `crates/sprawling/spec/Doctor.lean` |
| 8-73 | `crates/sprawling/spec/Doctor.lean` |
| 8-74 | `crates/sprawling/spec/Doctor.lean` |
| 8-76 | `crates/sprawling/spec/Doctor.lean` |
| 8-78 | `crates/sprawling/spec/Doctor.lean` |
| 8-79 | `crates/sprawling/spec/Doctor.lean` |
| 8-80 | `crates/sprawling/spec/Doctor.lean` |
| 8-81 | `crates/sprawling/spec/Doctor.lean` |
| 8-82 | `crates/sprawling/spec/Doctor.lean` |
| 8-83 | `crates/sprawling/spec/Doctor.lean` |
| 8-84 | `crates/sprawling/spec/Doctor.lean` |
| 8-85 | `crates/sprawling/spec/Doctor.lean` |
| 8-86 | `crates/sprawling/spec/Doctor.lean` |
| 8-87 | `crates/sprawling/spec/Doctor.lean` |
| 8-88 | `crates/sprawling/spec/Assembly/Listening.lean` |
| 8-89 | `crates/sprawling/spec/Main/Grammar.lean` |
| 8-90 | `crates/sprawling/spec/Assembly/Listening.lean` |
| 8-91 | `crates/sprawling/spec/Assembly/Listening.lean` |
| 8-92 | `crates/sprawling/spec/Supervising.lean` |
| 8-93 | `crates/sprawling/spec/Serving/Standing.lean`，放置一半在 `crates/sprawling/spec/Serving/Placement.lean` |
| 8-94 | `crates/sprawling/spec/Monitor.lean` |
| 8-95 | `crates/sprawling/spec/Monitor.lean` |
| 8-96 | `crates/sprawling/spec/Monitor.lean` |
| 8-97 | `crates/sprawling/spec/WireClient.lean` |
| 8-98 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-99 | `crates/sprawling/spec/Assembly/Listening.lean` |
| 8-100 | `crates/sprawling/spec/Assembly/Listening.lean` |
| 8-101 | `crates/sprawling/spec/Assembly/Listening.lean` |
| 8-102 | `crates/sprawling/spec/Assembly.lean` |
| 8-103 | `crates/sprawling/spec/Main/Exit.lean` |
| 8-104 | `crates/sprawling/spec/Main.lean` |
| 8-105 | `crates/sprawling/spec/Main.lean` |
| 8-106 | `crates/sprawling/spec/Accounting/Views.lean` |
| 8-107 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-108 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-109 | `crates/sprawling/spec/Supervising.lean` |
| 8-110 | `crates/sprawling/spec/Supervising.lean` |
| 8-111 | `crates/sprawling/spec/Supervising.lean` |
| 8-112 | `crates/sprawling/spec/Supervising.lean` |
| 8-113 | `crates/sprawling/spec/Supervising.lean` |
| 8-114 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-115 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-116 | `crates/sprawling/spec/Monitor.lean` |
| 8-117 | `crates/sprawling/spec/Main.lean` |
| 8-118 | `crates/sprawling/spec/Install.lean` |
| 8-119 | `crates/sprawling/spec/Assembly.lean` |
| 8-120 | `crates/sprawling/spec/Doctor.lean` |
| 8-121 | `crates/sprawling/spec/Assembly/Listening.lean` |
| 8-122 | `crates/sprawling/spec/Assembly/Listening.lean` |
| 8-123 | `crates/sprawling/spec/Serving.lean` |
| 8-124 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-125 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-126 | `crates/sprawling/spec/Main.lean` |
| 8-127 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-128 | `crates/sprawling/spec/Accounting/Views.lean` |
| 8-129 | `crates/sprawling/spec/Main.lean` |
| 8-129-1 | `crates/sprawling/spec/Main.lean` |
| 8-129-2 | `crates/sprawling/spec/Main.lean` |
| 8-129-3 | `crates/sprawling/spec/Main.lean` |
| 8-129-4 | `crates/sprawling/spec/Main.lean` |
| 8-129-5 | `crates/sprawling/spec/Main.lean` |
| 8-129-6 | `crates/sprawling/spec/Main.lean` |
| 8-130 | `crates/sprawling/spec/Doctor.lean` |
| 8-131 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-132 | `crates/sprawling/spec/Main.lean` |
| 8-133 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-134 | `crates/sprawling/spec/Assembly.lean` |
| 8-136 | `crates/sprawling/spec/Main.lean` |
| 8-137 | `crates/sprawling/spec/Main.lean` |
| 8-139 | `crates/sprawling/spec/Outside/Conduit.lean` |
| 8-140 | `crates/sprawling/spec/Outside.lean` |
| 8-141 | `crates/sprawling/spec/Doctor.lean` |
| 8-142 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-143 | `crates/sprawling/spec/Main.lean` |
| 8-144 | `crates/sprawling/spec/Assembly/Listening.lean` |
| 8-145 | `crates/sprawling/spec/Supervising.lean` |
| 8-146 | `crates/sprawling/spec/Doctor.lean` |
| 8-151 | `crates/sprawling/spec/Assembly.lean` |
| 8-152 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-154 | `crates/sprawling/spec/Assembly/Listening.lean` |
| 8-155 | `crates/sprawling/spec/Supervising.lean` |
| 8-157 | `crates/sprawling/spec/Doctor.lean` |
| 8-161 | `crates/sprawling/spec/Supervising.lean` |
| 8-162 | `crates/sprawling/spec/Doctor.lean` |
| 8-163 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-164 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-165 | `crates/sprawling/spec/Assembly.lean` |
| 8-166 | `crates/sprawling/spec/Doctor.lean` |
| 8-167 | `crates/sprawling/spec/Accounting/Views.lean` |
| 8-168 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-169 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-170 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-171 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-172 | `crates/sprawling/spec/Accounting/Worker.lean` |
| 8-173 | `crates/sprawling/spec/Serving/Memory.lean` |

§8-163 到 §8-172 是迁移时给重号的节新编的号：这几节在 Markdown 里与另一节同号（§8-27、§8-28、§8-29、§8-40、§8-41、§8-42、§8-50、§8-60 各有两到三节），保留原号的是先出现、被引用的那一节。
-/

/-! ## 9 工作流程

`main` 经 `main::grammar` 把 argv 读成一次调用（§8-89），`main::router` 分派给动词；serve 的次序见 §8-88，一次派活见 §8-2 与 §8-113。
-/

/-! ## 10 实现逻辑

`build.rs` 的失败经 `cargo::error=` 显性报出（cargo ≥ 1.84 语法），而不是留下一份过期或缺失的资产让 `include_bytes!` 去撞。

**两个设计。** **A（选中）**：`build.rs` 把 `just build-web` 留在本包 `web-dist`（`BUNDLE_DIR`，§8-83）的客户端包压缩进 OUT_DIR，再由 `include_bytes!` 嵌入——嵌入只有一处。**B（落选）**：`include_bytes!` 直指源码树里的一个目录——少一步拷贝，但把「产物在哪」写死进源码路径，换产物就要改代码，且没有 `rerun-if-changed` 的粒度。

**模型读到的东西。** 模型读到的前缀由 `accounting::worker::freezing` 组装，四段全部入内容仓库（§8-12、§8-67、§8-85）。
-/

/-! ## 11 边界枚举

客户端包缺失在构建期即红，不是运行期的意外（§8-83）。命令行读不懂退 2（§8-103）。
-/

/-! ## 12 错误处理

失败对命令行的人与 agent 都落成一个退出码与一行 `AxError`：退出码是一张表（§8-103，`spec/Main/Exit.lean`），错误的失败行与恢复行由 `bin::main::refusal` 写出；各节写出自己的失败码与失败之后留下什么。

决定住在它所管主题的分部里：

| 决定 | 住处 |
|---|---|
| D1 | `crates/sprawling/spec/Main.lean` |
| D2 | `crates/sprawling/spec/Main.lean` |
| D3 | `crates/sprawling/spec/Assembly/Listening.lean` |
| D4 | `crates/sprawling/spec/Assembly/Listening.lean` |
| D5 | `crates/sprawling/spec/Assembly/ChainWatch.lean` |
| D6 | `crates/sprawling/spec/Doctor.lean` |
| D7 | `crates/sprawling/spec/Accounting/Worker.lean` |
| D8 | `crates/sprawling/spec/Accounting/Worker.lean` |
| D9 | `crates/sprawling/spec/Doctor.lean` |
| D10 | `crates/sprawling/spec/Doctor.lean` |
| D11 | `crates/sprawling/spec/Doctor.lean` |
| D12 | `crates/sprawling/spec/Doctor.lean` |
| D13 | `crates/sprawling/spec/Accounting/Worker.lean` |
| D14 | `crates/sprawling/spec/Accounting/Worker.lean` |
| D15 | `crates/sprawling/Spec.lean` |
| D16 | `crates/sprawling/spec/Assembly.lean` |
| D17 | `crates/sprawling/spec/Assembly.lean` |
| D18 | `crates/sprawling/spec/Assembly.lean` |
| D19 | `crates/sprawling/spec/Accounting/Worker.lean` |
| D20 | `crates/sprawling/spec/Accounting/Worker.lean` |
| D21 | `crates/sprawling/spec/Supervising.lean` |
| D22 | `crates/sprawling/spec/Supervising.lean` |
| D23 | `crates/sprawling/spec/Main.lean` |
| D24 | `crates/sprawling/spec/Main.lean` |
| D25 | `crates/sprawling/spec/Main.lean` |
| D26 | `crates/sprawling/spec/Accounting/Worker.lean` |
| D27 | `crates/sprawling/spec/Main.lean` |
| D28 | `crates/sprawling/spec/Main.lean` |
| D29 | `crates/sprawling/spec/Main.lean` |
| D30 | `crates/sprawling/Spec.lean` |
| D31 | `crates/sprawling/spec/Doctor.lean` |
| D32 | `crates/sprawling/spec/Doctor.lean` |
| D33 | `crates/sprawling/spec/Doctor.lean` |
| D34 | `crates/sprawling/Spec.lean` |
| D37 | `crates/sprawling/spec/Accounting/Landing.lean` |
| D40 | `crates/sprawling/spec/Serving/Standing.lean` |
| D41 | `crates/sprawling/spec/Serving/Placement.lean` |
| D42 | `crates/sprawling/spec/Serving/Memory.lean` |
| D43 | `crates/sprawling/spec/Serving/Memory.lean` |
| D44 | `crates/sprawling/spec/Console.lean` |
| D45 | `crates/sprawling/spec/Serving/Placement.lean` |
| D46 | `crates/sprawling/spec/Serving/Placement.lean` |
| D47 | `crates/sprawling/spec/Serving/Placement.lean` |
| D48 | `crates/sprawling/spec/Monitor.lean` |
| D49 | `crates/sprawling/spec/Doctor.lean` |
| D50 | `crates/sprawling/spec/Install.lean` |
-/

/-! D15 不从别的工具的配置里读 provider 表（人的决定）

sprawling 里那个已删的 `import` 模块 的五个文件读 Codex 的 `~/.codex/config.toml` 与 pi 的 `models.json`，把其中的 provider 折成本城的词汇；但没有任何 `mod` 声明过它们，所以它们从没被编译，clippy 与测试也从没看过它们，模块图却把它们记为 built。本城删去这五个文件，首次上手的第 1 步由人手填端点，或选一个已知主机。理由：没有调用点的代码是一份没人维护的第二文法，它记下的 Codex 与 pi 键名会随上游改版静默过时。**被否**：接上它，作为首次上手第 1 步「从别的工具已写好的配置读入」的候选来源——那要在 wire 上加一条 Query、在设置页加一行，属于线协议与页面的改动，本版不做。**重开参数**：首次上手要给出「别的工具已配置的 provider」这一步时，从 git 历史取回这两种文法，先在 `lib.rs` 声明模块，让测试与 clippy 看见它们，再接 Query。
-/
/-! ## 13 依赖选型

依赖以 `crates/sprawling/Cargo.toml` 为准；每个依赖旁的注释写它为哪一节而在。
-/

/-! ## 14 硬编码声明

客户端包的位置 `web-dist`，相对本包目录，由 `build.rs` 的 `BUNDLE_DIR` 一处声明（§8-83）。三份工具链钉子文件的位置（`rust-toolchain.toml`、`lean-toolchain`、`crates/desktop/ffi/zig-version`，相对检出的根）由 `build.rs` 的 `PINS` 一处声明（§8-157）。

`bin::install` 引入四处，全部是外部世界的事实而非我们的选择，故各自注明出处：`%LOCALAPPDATA%\Programs\<app>` 是 Windows 用户级程序目录的约定；`~/.local` 下的 `bin` 是 XDG 用户级可执行目录的约定；`HKCU\Environment` 是用户级环境变量在注册表里的位置；`WM_SETTINGCHANGE=0x1A`／`HWND_BROADCAST=0xffff`／`SMTO_ABORTIFHUNG=2` 是 Win32 的常量值。这四处一旦被平台改掉，改点各只有一个。
-/

/-! ## 15 影响面

`just build-web` 产出客户端包，`build.rs` 读它（§8-83）；交付形态见 §17。
-/

/-! ## 16 测试与约束

workspace lints 全量适用（含 build.rs）；各节写出钉住它的测试。

`lake build Spec`（`just models` 的一步）证明 §2 列出的每个模型，没有 `sorry`、`admit` 或 `axiom`。
-/

/-! ## 17 文档关系

每加一个动词：拥有它的分部增一节，`architecture.toml` 登记它的模块。

交付形态入册：`just package` 的产物名、`QUICKSTART.md`、README 与 `docs/getting-started.md` 的首次运行段、`release.yml` 的附件清单，五处同改。
-/
