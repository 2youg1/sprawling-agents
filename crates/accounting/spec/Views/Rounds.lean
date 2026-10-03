-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# accounting::views::rounds：外壳读的字段

规定 `crates/accounting/src/views/rounds.rs`、`crates/accounting/src/views/commits.rs` 与 `crates/accounting/src/views/lines.rs` 里外壳读的那几个字段。本文件是 `crates/accounting/Spec.lean` 的一个分部；下面每一节保留它在 accounting 规格里的标签 §8-n，别处引作 `crates/accounting/Spec.lean §8-n`，决定引作 `accounting D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `accounting::views::rounds::tests` 守住。
-/

/-!
### 8-33 外壳读的字段从哪一处折出（`accounting::views::rounds`、`accounting::views::commits`、`accounting::views::lines`，形状 7 投影；`crates/wire/Spec.lean` §8-76–§8-78）

```rust
// accounting::views::commits
pub(crate) struct CommitFacts { /* …既有字段… */ b3: Option<B3Hash> }   // 折叠时算，进快照编码
// accounting::views::rounds：Call.exit_code、Opening.policy、RoundsAnswer.worktree 在 `turns`／`opening` 那一次遍历里读
// accounting::views::lines：ConfigAnswer.first = Some(CTX_REMINDER_FIRST_PERCENT)
// accounting::views::rounds：Turn.returned 取回复那一行自己的时刻
// accounting::views::rounds::opening：Opening.effort 照录 run_started；Opening.names 按 run_started 的 naming 版本从内容库读回
```

- **提交的 B3 在折叠那一行时算。** `fold_commit` 手里有这一行，`canonical_line` 的 BLAKE3 就是它的身份；答问时只拷出。快照编码随之变，`VIEWS_FOLD_RULES` 由夹具摘要进位（§8-24）。
- **退出码经 `runtime::pipeline::exit_code_in` 读**，与配对的 `tool_result` 同一刻：答复写下 `outcome` 与 `output` 的那一处。
- **运行策略照录 `run_started` 的 `policy`**，与 `task`、`goal`、`dispatched_by` 同一次读；**工作树名照录这次 run 自己的 `worktree_opened`**，窗口里第一条为准。
- **一轮的返回时刻取回复那一行自己的时刻**（`EventRecord::moment`）。版本一的行没有这个时刻，答 `None`，不拿这一轮的时间戳顶替。
- **开场的 effort 照录 `run_started` 的 `effort`。** `runtime::run::charter` 从计划的调用形状抄下这次请求冻结的 effort，所以第一行就带着它；还没有提交的 run 也有页面能读的 effort。
- **开场的名字按 `run_started` 的 `naming` 版本从内容库读回**（`crates/wire/Spec.lean` §8-79、D17）。内容库打不开、库里已经没有这个版本、字节读不成冻结的命名时，`names` 为 `None`：页面退回地址与角色名，绝不读今天的名字。
- **缓存写照 `wire::used_in` 读**：读法在 wire，本模块不另读 `usage`。
- 验收：`views::rounds::tests`、`views::commits::tests`、`views::snapshot::tests` 里点名这几个字段的用例；`views::rounds::carried_tests::a_turn_reports_when_its_reply_returned`；`views::rounds::opening_tests::the_opening_carries_the_effort_and_the_names_the_run_froze`、`a_naming_the_store_lost_answers_no_names`（D45）。
-/

/-! D45 外壳读的字段在折叠时读出，各自照录写下它的那一行，读不出即缺席

（§8-33）。(a) 提交的 B3 进 `CommitFacts`，在 `fold_commit` 时算。理由：那一刻规范字节就在手里；答问时再算要按 `seq` 回读账本，一页五百个提交就是五百次读。被否决的做法：答问时读——多一次读盘，只为省快照里每个提交 32 字节。(b) 退出码经 runtime 的一个读者读，不在读面另写一次 `"exit_code"`。理由：键是 `tools::exec::outcome` 写的，读法也该只有一处；被否决的做法：读面直接 `get("exit_code")`——这个键的第三处拼写。(c) 工作树名放在 `RoundsAnswer` 上、与 `opened_at` 并列，不放进 `Opening`。理由：它来自另一行（`worktree_opened`），而 `Opening` 是 `run_started` 一行的投影；放在一起就是一个投影读两种行。(d) 开场的名字存的是版本，答问时按版本从内容库读回，读不出即缺席。理由：一次会话冻结的是那一刻的名字，城改名不该追到已经开始的会话；`run_started` 已经带着冻结那份命名的版本，字节在内容库里只有一个家。被否决的做法：①读今天的命名——改名会改写过去的会话；②把名字原文抄进 `run_started`——同一份命名有了第二个家，旧行仍要第二种读法。重开参数：内容库开始回收会话仍引用的版本时，这条缺席就不再罕见，要重新论证。
-/

/-! D48 跨行配对的 note 归 `accounting::views::rounds::paired`

`turns` 只读一个会话自己的行；三种 note 的另一半在别处，配对放在 `turns` 之后的一遍里，`rounds_answer` 依次调用：(a) `Note::Waiting.answered`：`approval_resolved` 记在城自己的 run 下，按 approval id 配（`crates/sprawling/Spec.lean` §8-50-1）；(b) `Note::Arrived` 的 `from`、`said` 与 `handback`：`signal_enqueued` 记在发信者的 run 下，从 `signal_consumed` 那一行往前、在整本账里按 `SignalId` 找，最多 `wire::HISTORY_MAX` 行（wire D36）；找到的那一条 signal 经 `collab::Handback::from_signal` 读成交接时填 `handback`，`session` 是那一行的 run（wire D38）；(c) `Note::AwaitingReply.ended`：`signal_wait_ended` 与它的开始记在同一会话里，按 `signal` 配（wire D37）。配不上一律留 `None`，不猜。理由：`rounds.rs` 是一次遍历的折叠，配对要读别的 run，二者分开，各自一个改动的理由。被否决的做法：在 `turns` 里一边折一边配——`turns` 是纯函数，测试与回放直接喂它记录；让它读账本就失去了这一点。验收：`views::rounds::tests` 里 `a_signal_pulled_from_another_session_says_who_sent_it_and_what`、`a_reply_wait_says_how_it_ended`、`a_handback_says_whether_the_child_finished`；`views::tests::folding` 的收件箱用例点名 `first_line`（wire D39）。
-/

/-! ### 接口仍写在 sprawling 规格里的模块

下面这些模块的接口与取舍今天写在 `crates/sprawling/Spec.lean` 的这几节里，按标签列出；`architecture.toml` 里它们的行指向本分部，这张表把读者带到那一节。它们搬进本 crate 的规格是 D15 记下的下一步。

| sprawling 的标签 | 模块 |
|---|---|
| §8-50 | `accounting::views::rounds`、`accounting::views::rounds::tests`、`accounting::views::rounds::reading_tests` |
-/
