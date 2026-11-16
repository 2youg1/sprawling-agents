-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::bench

规定 `bench`、`bench::admit`、`bench::outside`（`crates/runtime/src/` 下同名的文件）。工作台：一条调用过哪扇门、以什么次序，去重答的是第一次的结果，外来内容进 run 的唯一入口。本文件是 `crates/runtime/Spec.lean` 的一个分部；下面每一节保留它在 runtime 规格里的标签 §8-n，别处引作 `crates/runtime/Spec.lean §8-n`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `runtime::bench::tests` 守住。
-/

/-!
### 8-19 runtime::bench 目录化


| 文件 | 管什么 |
|---|---|
| `bench.rs` | `ToolBench` 与 `BenchOutcome` 的定义、装配面（`new`／`for_job`／`with_checkpoint`／`register`／`taint_mut`／`meta_of`）、`invoke` 的路由次序，以及 `kernel_error_from_storage` |
| `bench/admit.rs` | 门：`admit` 按 `Effect` 分派到 Write／Connector／Egress／Spawn／Govern 各门，`settled`／`crossed` 把一次判定翻译成 `BenchOutcome`，`scanned` 为两扇朝外的门备好密钥扫描的字节 |
| `bench/tests.rs` | 去重、门、taint、checkpoint 与注册冲突的夹具 |
-/

/-!
### 8-35 去重答的是第一次的结果，而不是一句「你已经问过了」（形状 1 判定）


**「同一次调用做两遍」的正确答案是第一次的结果**：回一个错误，重试的模型学到的是「这件事失败了」，而它其实成功了——那是幂等只做了一半：副作用被挡住，答案没有被记住。所以 `ToolBench` 记住的是键与答，两个调用方（`accounting::worker::driving::lane`、`citysim::executor`）把 `Duplicate` 回成第一次的 `ToolOutcome`。

```rust
seen: BTreeMap<IdemKey, Result<ToolOutcome, AxError>>   // ToolBench 私有
pub enum BenchOutcome { …, Duplicate { outcome: ToolOutcome } }   // 调用方回第一次的 ToolOutcome，checkpointed 为空
```

- **写入点**：键与答在工具答过之后一起写入（`account`），失败的答也记下；被门拒的调用不入表，所以被门拒后的重试不算重放。
- **`Duplicate` 仍是一个独立变体而不是并进 `Ran`**：`checkpointed` 对重放恒为空，而 `Ran` 的调用方要按 `checkpointed` 决定波后清扫；把两者合并会让「这一波要不要扫」多出一个恒空的分支。
- **代价写在明处**：一次运行期间每个成功调用的结果都留在内存里。这与 `seen` 本来就要活到运行结束是同一条寿命，多出来的是 payload 的字节；一次运行的工具调用数以百计而非以百万计。
-/

/-!
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
-/
