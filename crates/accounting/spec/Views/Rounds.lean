-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# accounting::views::rounds：外壳读的字段

规定 `crates/accounting/src/views/rounds.rs`、`crates/accounting/src/views/commits.rs` 与 `crates/accounting/src/views/lines.rs` 里外壳读的那几个字段。本文件是 `crates/accounting/Spec.lean` 的一个分部；下面每一节保留它在 accounting 规格里的标签 §8-n，别处引作 `crates/accounting/Spec.lean §8-n`，决定引作 `accounting D<n>`。
-/

/-!
### 8-33 外壳读的字段从哪一处折出（`accounting::views::rounds`、`accounting::views::commits`、`accounting::views::lines`，形状 7 投影；`crates/wire/Spec.lean` §8-76–§8-78）

```rust
// accounting::views::commits
pub(crate) struct CommitFacts { /* …既有字段… */ b3: Option<B3Hash> }   // 折叠时算，进快照编码
// accounting::views::rounds：Call.exit_code、Opening.policy、RoundsAnswer.worktree 在 `turns`／`opening` 那一次遍历里读
// accounting::views::lines：ConfigAnswer.first = Some(CTX_REMINDER_FIRST_PERCENT)
```

- **提交的 B3 在折叠那一行时算。** `fold_commit` 手里有这一行，`canonical_line` 的 BLAKE3 就是它的身份；答问时只拷出。快照编码随之变，`VIEWS_FOLD_RULES` 由夹具摘要进位（§8-24）。
- **退出码经 `runtime::pipeline::exit_code_in` 读**，与配对的 `tool_result` 同一刻：答复写下 `outcome` 与 `output` 的那一处。
- **运行策略照录 `run_started` 的 `policy`**，与 `task`、`goal`、`dispatched_by` 同一次读；**工作树名照录这次 run 自己的 `worktree_opened`**，窗口里第一条为准。
- **缓存写照 `wire::used_in` 读**：读法在 wire，本模块不另读 `usage`。
- 验收：`views::rounds::tests`、`views::commits::tests`、`views::snapshot::tests` 里点名这几个字段的用例（D45）。
-/

/-! D45 外壳读的字段在折叠时读出，各自照录写下它的那一行，读不出即缺席

（§8-33）。(a) 提交的 B3 进 `CommitFacts`，在 `fold_commit` 时算。理由：那一刻规范字节就在手里；答问时再算要按 `seq` 回读账本，一页五百个提交就是五百次读。被否决的做法：答问时读——多一次读盘，只为省快照里每个提交 32 字节。(b) 退出码经 runtime 的一个读者读，不在读面另写一次 `"exit_code"`。理由：键是 `tools::exec::outcome` 写的，读法也该只有一处；被否决的做法：读面直接 `get("exit_code")`——这个键的第三处拼写。(c) 工作树名放在 `RoundsAnswer` 上、与 `opened_at` 并列，不放进 `Opening`。理由：它来自另一行（`worktree_opened`），而 `Opening` 是 `run_started` 一行的投影；放在一起就是一个投影读两种行。
-/

/-! ### 接口仍写在 sprawling 规格里的模块

下面这些模块的接口与取舍今天写在 `crates/sprawling/Spec.lean` 的这几节里，按标签列出；`architecture.toml` 里它们的行指向本分部，这张表把读者带到那一节。它们搬进本 crate 的规格是 D15 记下的下一步。

| sprawling 的标签 | 模块 |
|---|---|
| §8-50 | `accounting::views::rounds`、`accounting::views::rounds::tests`、`accounting::views::rounds::reading_tests` |
-/
