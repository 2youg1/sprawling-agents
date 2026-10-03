-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::answer::automation

规定 `answer::automation`（`crates/wire/src/` 下同名的文件）。页面读到的日程表与 watch 表，与读不出的那一份。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的帧形状由 Rust 的类型与 `crates/wire/tests/wire_contract.rs` 钉住的 wire schema（`wire::schema_hash`）守住。
-/

/-!
### 8-62 自动化只读组：`Query::Automation`

```rust
// Query
Automation,                                  // → Answer::Automation(Box<AutomationAnswer>)
pub struct AutomationAnswer {
    pub jobs: Vec<ScheduledJob>,             // SCHEDULE.toml，文件里的顺序
    pub sources: Vec<WatchedSource>,         // WATCH.toml，文件里的顺序
    pub unreadable: Vec<String>,             // 读不出的那份文件：文件名与拒因；它的行不列
}
pub struct ScheduledJob { pub name: String, pub addr: Address, pub task: String, pub goal: String, pub cadence: Cadence }
pub enum Cadence { EveryMinutes { minutes: u64 }, DailyAt { minute: u64 }, WeeklyAt { minute: u64 } }   // minute：UTC 的一天／一周里的第几分钟
pub struct WatchedSource { pub name: String, pub matches: String, pub addr: Address, pub starts_work: bool }
```

- **只读，读的是文件此刻。** 两份文件由人在城根上写，城每一拍读一次 `SCHEDULE.toml`，起服务时读一次 `WATCH.toml`；本查询在问的那一刻经 `city::Schedule::load` 与 `city::Watch::load` 各读一次，答的正是下一拍会读到的东西。页面上不写这两份文件（S06 Q4 (b)）：写它们要一套 cron 与路由的编辑器，而 CLI 与编辑器已经够得到。
- **一份读不出不挡另一份。** 读不出的文件进 `unreadable`，拒因就是派活那一拍会给的那一句；另一份照常列出。没有文件即空列表，那是没设自动化的城的常态。
- **`Cadence` 是线上自己的拼法。** `city::Cadence` 不派生 serde（它是城判定的值），本 crate 定一份线上形状，装配处一一映射，三个值一一对应、穷尽匹配，不另存规则。
- 验收：accounting 的 `the_automation_query_lists_both_tables_and_names_the_one_that_does_not_read`。

**(b) 从检查点取回单个文件：`Command::RestoreFile`**（S07 Q2 (c)）

```rust
RestoreFile { at: Address, point: GitOid, idem: IdemKey }
```

- **把城自己工作树里的一个文件换回检查点里的那一份。** `at` 是文件在城里的地址（`Address` 爬不出城、点不到保留子树），`point` 是一个检查点的 oid（页面从 `checkpoint_committed` 或 `Query::Commits` 读到）。检查点里有这个文件：工作树里这一处的字节换成检查点里的那一份（原子替换，取被替换文件的权限）；检查点里没有：工作树里这一处的文件删去。不回到过去开一棵树（S07 Q2 (c) 只做取回单个文件）。
- **有 run 在这栋楼里干活就拒。** 拒 `E_BUSY`，点名房间与 run：取回会改掉 run 正在写的那棵树，与 `RemoveBuilding` 同一条理由。
- **一步一行。** 写成之后记一行 `file_restored { name: "", path, point }`；`name` 为空串指城自己的工作树，非空时仍是一棵 run 的工作树的名字（`crates/kernel/Spec.lean` §8-4，`crates/storage/Spec.lean` §8-33）。
- 验收：storage 的 `taking_a_file_back_replaces_what_the_tree_holds_and_removes_what_the_point_did_not_hold`。
-/
