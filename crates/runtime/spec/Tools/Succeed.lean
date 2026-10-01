-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::tools::succeed

规定 `tools::succeed`（`crates/runtime/src/` 下同名的文件）。succession 给模型看的那一面。本文件是 `crates/runtime/Spec.lean` 的一个分部；下面每一节保留它在 runtime 规格里的标签 §8-n，别处引作 `crates/runtime/Spec.lean §8-n`。
-/

/-!
### 8-33 runtime::tools::succeed 与 succession（形状 4 适配器）


> 三件事一起落地，缺一件就是一个洞。

**动词**：`succeed {reason}`。一跑请求由继任者接替自己：**同地址、同深度、同工具表**，无需人在环内。它答的是「继任者将在你冻结后于 `<addr>` 启动；先把 `Handoff.md` 写在你的房间里」，而不是一个结果——与 `delegate` 同理，工具不能从一个 run 的工具台里驱动另一个 run。桌面 `SuccessionDesk` 至多持一份请求（第二次调用覆盖第一次，理由随之更新）；装配层在 `conclude` 里读它，**被取消的跑不接替**（与 delegate 的第四安全点同一条规则）。

**深度守恒**：succession 与 `delegate` 是两个动词。`delegate` 让深度加一；succession 不加。`Assignment::depth()` 从 `parent` 推出，继任者**继承前任的 `parent`**（而不是以前任为 parent），所以深度按构造守恒，工具表因而与前任逐名相同——`delegate` 在内。红测试：继任者的工具表与前任逐名相等。

**账本**：`Assignment`／`RunPlan` 增 `predecessor: Option<RunId>`，写进 `run_started` 的 `predecessor` 键；`Provenance` 增同一指针（`crates/storage/Spec.lean` §8-17：第六条 trailer `Sprawling-Predecessor`，仅在有前任时出现；`model_fields` 同时写 `predecessor`）。`accounting::views` 从 `run_started` 折出 `predecessors: BTreeMap<RunId, RunId>`，`Query::Commit` 的答 `CommitAnswer` 增 `lineage: Vec<RunId>`——本跑在前，逐级向前到第一任（`crates/wire/Spec.lean` §8-18）。红测试：三次接替后 lineage 有四个 run。

**Handoff 住房间**：`city::handoff(city_root, room)`／`handoff_path(city_root, room)` 读写 `<city>/<room>/Handoff.md`；模板在 `city::open_room` 打开房间时铺下，楼级 `lay_out` 不铺它。理由是同楼并发：一栋楼一份 Handoff，两个房间同时冻结就是两份内容抢一个文件。

**质量防线（不可选）**：`accounting::worker::probing::probe` 的 handoff 探针在每次 succession 真的跑。`handoff_probe()` 给出固定四问（版本 1）；装配层 `accounting::worker::probing` 在 `conclude` 读到接替请求时，用前任的 adapter 对前任的 transcript 问一遍（before），在继任者 `freeze_plan` 之后、第一回合之前，用继任者的 prefix 问一遍（after），`compare` 后记一条 `eval_run`（`probe`／`version`／`kept`／`lost`／两份答案）。探针答案不是判定，`lost` 报的是位置，人自己去读两份答案——这正是 sprawling-SPEC §8-39 交接探针一节定的口径。每次 succession 两次模型调用，这是这道防线的价钱，写在明处。
-/
