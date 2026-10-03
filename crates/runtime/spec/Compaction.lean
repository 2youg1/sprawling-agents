-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::compaction

规定 `compaction`、`compaction::exchange`（`crates/runtime/src/` 下同名的文件）。按内容分类的缩短判定，以及回合边界的压缩。本文件是 `crates/runtime/Spec.lean` 的一个分部；下面每一节保留它在 runtime 规格里的标签 §8-n，别处引作 `crates/runtime/Spec.lean §8-n`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `runtime::turn::tests::compaction`、`runtime::compaction::exchange::tests`、`runtime::compaction::tests` 守住。
-/

/-!
### 8-44 runtime::compaction::exchange（形状 2 值＋形状 1 判定）：回合边界的压缩


一回合加进 window 的东西是一对值：发出调用的助手回复与那波的答案——少了任何一半都不是对话，所以它们是一个值 `Exchange`（形状 2），由 `Turn<ToolWave>` 在波落地时收集、由 `runtime::fork` 从同一批记录重建。压缩只发生在 `Turn<Recording>::record` 的收尾边界（工具波全落地之后），这扇门一个回合只开一次（形状 1 判定住 `compact_texts`，调用的是 `compaction::plan`）。

- **判定并入 `compaction::plan`，`turn.rs` 不另写阈值**：exchange 的总字节对着预算 `EXCHANGE_BUDGET_BYTES`（住 `kernel::consts_policy`）触发；触发后每段文本按**均分份额**过 `plan`：`Keep` 与 `MustOffload` 原样，`Cut(Strategy)` 走 `shorten`。除 `plan` 外这条路上没有第二次「装不装得下」的比较。
- **波中永不换快照**：半落地的波会让压缩按半截分组，而 `fork` 是一回合一回合同一值重建的——两边分组不同就是同一历史两种字节，分支从此与母亲分叉。故收集期不压、只在收尾边界对全波一次压；红测用「全波分组与半波分组产出不同字节」钉住这一点。
- **`MustOffload` 在这扇门上不动文本**：它要的是整块离窗留引用，而这扇门没有 store（tee 在落地管线那一侧），城里也没有工具解析得了引用——所以文本留原样。回路不是窗口：进这扇门的文本恒等于账上 `model_returned.data.content` 与 `tool_result` 里的那一份（`turn/wave.rs` 只打印一次、`fork` 由同一条记录重建），而 `exec` 结果在落地时已被管线裁过或 tee 过——原件在 CAS、账上有 `result_offloaded` 指过去。两条路都没有「只存在于窗口里」的字节。
- **thinking 与 redacted thinking 永不碰**：thinking 块带覆盖其字节的签名，redacted 形态是封好的密文。
- **`runtime::fork` 同边界重放**：`fold_run` 的 `Wave` 持同一个 `Exchange`，在波收齐（或整波丢弃）的同一点 `compact()`——live 折叠与离线重建因此同源同字节，C16 的承诺由这条对拍承接。
- **唯一的失败**：`Exchange::compact` 在一段文本计不进 `u64` 时以 `E_INVALID_ARGS` 报（动作＝压缩这一回合的 exchange，主体＝那段文本，recovery 指向本模块）——这是「没有人解析得了的窗口字节」，不是可恢复的压缩结果。live 路径（`record`）与回放路径（`fold_run`）在同一个值上走同一次判定，故同一段文本两边同样拒，回放不会因为压缩而少一条分支。
- **数字一个家**：预算只住 `consts_policy::EXCHANGE_BUDGET_BYTES`，本文件不复写它的值。
-/

/-! D2 定规：回合边界的压缩只在收尾边界换快照

- **决定**：一回合的 exchange 只允许在 `Turn<Recording>::record` 的收尾边界被压缩替换，时机是工具波全落地之后、一回合一次；判定由 `compaction::plan` 一处给出（8-44），`turn.rs` 不写任何阈值比较；`runtime::fork` 在同一边界重放同一判定。
- **理由**：压缩若在波中换快照，它看见的是半截波，分组与 `fork` 的逐回合重建不同，同一历史会折出两种字节，分支与母亲分叉——这正是确定性回放（ARCHITECTURE §10）要排除的失败。收尾边界是这一波的唯一完整分组点。
- **击败的备选**：①逐结果在波中压缩——分组随落地次序漂移，重放无法复现；②在 `turn.rs` 写一份阈值判断——「装不装得下」已经有一个家（`compaction::plan`），第二个家会各自漂移；③在边界把 `MustOffload` 的文本换成引用——这扇门没有 store，且城里没有工具解析得了引用，模型拿到的将是一条跟不上的指针，故该类文本原样保留、以 Ledger 为回路。
-/
