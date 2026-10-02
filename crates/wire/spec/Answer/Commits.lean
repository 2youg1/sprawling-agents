-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::answer::commits

规定 `answer::commits`（`crates/wire/src/` 下同名的文件）。城对它做过的一个提交说什么：按 oid 一个，或倒序一页。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。
-/

/-!
### 8-17 `Query::Commit`：一次提交出自哪次运行

```rust
Commit { oid: GitOid },        // → Answer::Commit(CommitAnswer)，或 Answer::Unavailable

pub struct CommitAnswer {
    pub oid: GitOid,
    pub run: RunId,
    pub actor: Address,                // Sprawling-Actor：这次运行工作的那个地址
    pub model: String,                 // Sprawling-Model；空串＝那条记录没说
    pub effort: Option<kernel::Effort>,// Sprawling-Effort
    pub seq: Seq,                      // 宣告这次提交的那一行在账本里的位置
    pub at: TimeMs,                    // 那一行写下的时刻
    pub session: Option<SessionName>,  // 房间那一段：人给这条活起的名字
}
```

- **四个字段与 trailers 逐一对上，少 `Sprawling-City`**：问这个问题的人手里已经拿着城，
  把城的身份再答一遍是在回答它自己的问题。`seq` 是 trailers 携不了的那一个：
  从哪一行接着读去。
- **`session` 不是 `actor` 的复述**：派到楼根的运行没有房间，故它是 `Option`；
  有房间时它就是 `city::open_room` 当初拿这个名字开的那一段（重名时带 `-2` 后缀）。
- **一条这座城没写过的 oid 答 `Unavailable`**，与 `Changes` 同口径：「没有变化」与
  「我看不了」是两个答案，而读的人对它们的下一步不同。
- **答案从账本来，不从 git 来**（`crates/storage/Spec.lean` §8-18）。一座导出后在别处恢复、
  `.git` 不在身边的城，照样答得出自己的历史。
- **客户端欠的（前端冻结，此处不画界面）**：`client/src/wire.ts` 需重新生成
  （`cargo xtask wire-ts --write`）；只把新变体接进
  `mount::frame` 那条「有答案而暂无页面问它」的臂，使其仍能编译。
-/

/-!
### 8-18 `CommitAnswer.lineage`：一次提交背后的接替链

`CommitAnswer` 增 `lineage: Vec<RunId>`：本跑在前，逐级向前到第一任；没接替过谁的跑是长度 1 的链。名字表没动，语法换了形——正是 `WIRE_V` 存在的那种情形，于是 14→15，golden 由 `730e9d0b…` 变为 `24b7e8ff3727cad505653c951a6733b748cb294f9eec418adefb3c4a7e7223b9`。服务端从 `run_started` 的 `predecessor` 键折出 `predecessors` 表（`accounting::views::commits`），答时沿表走链。客户端读它的页尚未画，`crates/web` 只需编译通过；新客户端欠一行「replaced <run>」。
-/

/-!
### 8-24 `Query::Commits`：一座楼做过的提交，倒序分页

```rust
// Query 第 23 条（声明序，QUERY_NAMES 同序追加）
Commits { building: Option<Address>, before: Option<Seq>, limit: u32 },   // → Answer::Commits(CommitsAnswer)

pub struct CommitsAnswer {
    pub building: Option<Address>,   // 问题里的那个，原样回带
    pub before: Option<Seq>,         // 同上
    pub commits: Vec<CommitAnswer>,  // seq 递减
    pub more: bool,                  // 末条之前还有没有符合条件的提交
}
```

- **答案复用 `CommitAnswer`**：一个提交是什么只有一处权威，列举与反查因而不可能给出两套字段；`lineage` 逐条照答，楼页据此画接替标记。
- **倒序分页同 `History`**：`before` 是独占上界（`None` 从尾读起），`limit` 夹到 `1..=HISTORY_MAX`。分页游标是 `commits.last().seq`：下一页问 `before: Some(那个 seq)`。`more` 而不是 `earlier: Option<Seq>`——`History` 按 seq 连续扫描，所以「从哪接着读」是它自然算出的量；提交在账本里是稀疏的，下一条在哪只有再走一步才知道，而客户端手里已经有末条的 seq，答一个游标就是把它已有的东西再给一遍。
- **`building` 按 `actor` 地址前缀过滤，不按 session**：`actor == building` 或 `actor` 以 `<building>/` 起头；run 的 actor 是权威，session 是它的投影（`session_of`），反过来过滤会丢掉派到楼根、没开过会话的 run。`None` 列全城。
- **答案回带 `building` 与 `before`**：线上没有请求 id，客户端按内容把答案配回问题（`ChangesAnswer` 回带 `base`／`head` 是同一个理由）；缺了这两个字段，两座楼的两页同时在飞时无法分辨谁是谁的。
- **失联如实**：只列城自己写过的提交；人 rebase／squash 之后 trunk 上的 oid 不在其中，`Commit` 对它仍答 `Unavailable`。不读提交体的 trailer 回填——那会让投影成为第二权威（`views/commits.rs` 模块头）。
- **服务端**：`views::holding` 给按 oid 键的 `commits` 表加一条按 `seq` 的索引（`commit_seqs: BTreeMap<Seq, GitOid>`），`fold_commit` 两表同写；sprawling-SPEC §8-53。
- **`CommitAnswer` 长出 `at: TimeMs`**（同版内，第二条提交）：宣告这次提交的那条记录自己的 `t`。理由来自第一张截图——一列 seq 没法扫读，而一列时间可以。与 `Opening.at`／`Closing.at` 同源、同型。
- **客户端**：`cargo xtask wire-ts --write` 重生。
-/

/-!
### 8-54 一次提交带出同一次 run 的上一个提交，与它在 git 里的父提交

```rust
pub struct CommitAnswer {
    // …既有字段…
    pub previous: Option<CommitAt>,     // 同一次 run 在它之前宣告的最后一个提交；这是那次 run 的第一个时为 None
    pub parents: Option<Vec<GitOid>>,   // 提交对象自己记的父提交，按 git 的次序；读不到时为 None
}
pub struct CommitAt { pub oid: GitOid, pub seq: Seq }
```

- **`previous` 由账本折出。** 两条宣告提交的记录（`checkpoint_committed` 的提交一支与 `pr_merged`）按 `seq` 折进视图时，同一次 run 上一次宣告的那个提交就是它的 `previous`（sprawling-SPEC 8-128）。它与本提交围出一段：`Query::Changes { base: previous.oid, head: Some(oid) }` 答这次提交相对上一个检查点改了哪些文件，`Query::RunHistory { run, before: Some(seq) }` 往回读到 `previous.seq` 为止，答这一段里这次 run 发出的调用。这一段是候选，不是原因：同一栋楼里别的 run 与人也可能在这一段里写过文件。
- **`parents` 读自 git，在答问时读。** 提交对象自己记着它的父提交，账本记下的 oid 就是这个对象（连同父提交）的哈希，所以这里读的是权威本身，不是投影；五条 trailer 才是投影，本节不读它们。`Some(vec![])` 是根提交；`None` 是这座城没有仓库、仓库里没有这个对象，或者读失败——一座导出后在别处恢复、身边没有 `.git` 的城，其余各字段照答，只是画不出这一格。读 git 在快照的锁放开之后做（sprawling-SPEC 8-100），一页提交只开一次仓库。
- **`message: Option<String>` 读自 git，与 `parents` 同一刻读。** 提交对象自己记着说明，账本从未记过它，所以它与父提交同属「答问时读 git」那一类（D3(b)）：`Some` 是提交对象里的说明原文，五条 trailer 在内；`None` 的情形与 `parents` 相同，另加说明不是 UTF-8。页面画提交行时取第一行，要读全文时读这一格。名字不变而形状变了，与本批其余改形共用 `WIRE_V` 45（D1）。
-/

/-!
### 8-78 一次提交带出宣告它的那一行的 B3

```rust
pub struct CommitAnswer {
    // …既有字段…
    #[serde(default)] pub b3: Option<B3Hash>,   // 宣告这次提交的那一行（`checkpoint_committed` 或 `pr_merged`）的规范字节的 BLAKE3
}
```

- **是那一行自己的摘要，不是它的 `prev`**：账本的每一行带着上一行的摘要（`EventRecord::prev`）；一行自己的摘要是对它的规范字节（`EventRecord::canonical_line`）求 BLAKE3，也就是下一行的 `prev`。核对链的那一处（`storage::jsonl::verify`）算的就是这个数，所以人拿它可以在账本里找到这一行、并确认它没被改过。
- **在折叠时算**：`views::commits` 折这一行时就有它的规范字节，答问时不再读账本。
- **`None`**：一行在折叠时序列化不出规范字节（读回来的行不会这样，那是校验过的字节），或一座旧城答出的帧（D13）。
- **ISO 时刻就是 `at`**：见 D13。
-/
