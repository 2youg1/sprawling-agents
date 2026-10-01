-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::replay

规定 `replay`、`replay::resume`、`replay::shape`（`crates/runtime/src/` 下同名的文件）。离线重演：只重演不重执行，验链、重建记录序列，以及崩溃恢复时哪些工具调用没有结局。本文件是 `crates/runtime/Spec.lean` 的一个分部；下面每一节保留它在 runtime 规格里的标签 §8-n，别处引作 `crates/runtime/Spec.lean §8-n`。
-/

/-!
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
-/

/-!
### 8-20 runtime::replay 目录化


**三份文件，各答一个问题。** 崩溃恢复与链验证是两件事：链验证读的是字节与哈希，崩溃恢复读的是已验证行之间的配对关系（`tool_called` 有没有后继的 `tool_result`）。两者同处一个文件时，`replay.rs` 越过了 400 行上限。

| 文件 | 管什么 |
|---|---|
| `replay.rs` | `VerifiedLine`／`VerifiedLedger`／`Envelope`，以及 `verify_lines`／`verify_ledger_dir`／`fold_ledger_dir`／`rebuild_prefix`。它同时是子模块的父模块，声明 `mod resume;` 并 `pub use resume::{dangling_tool_calls, outcome_unknown_draft};`，因此 crate 内外的 `use` 一行未改 |
| `replay/resume.rs` | 崩溃恢复这一条规则的两次读法：`dangling_tool_calls` 认出结果未知的调用，`outcome_unknown_draft` 写下关掉它的那一行（`E_TOOL_OUTCOME_UNKNOWN`）。什么算悬空决定关帐行说什么，故同一个文件 |
| `replay/tests.rs` | 离线验证拒绝什么：更高的 `v`、无 `ig:true` 的未知 kind、漂移的 prefix 源文档、悬空的 tool_called |
-/
