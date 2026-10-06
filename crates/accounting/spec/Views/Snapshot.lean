-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# accounting::views::snapshot

规定 `crates/accounting/src/views/snapshot.rs`、`crates/accounting/src/views/snapshot/start.rs` 与 `crates/accounting/src/views/sessions.rs`。本文件是 `crates/accounting/Spec.lean` 的一个分部；下面每一节保留它在 accounting 规格里的标签 §8-n，别处引作 `crates/accounting/Spec.lean §8-n`，决定引作 `accounting D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `accounting::views::snapshot::tests`、`accounting::views::tests::sessions` 守住。
-/

/-!
### 8-19 视图的第二份是克隆；一次性重建每行只核对一遍；房间的各段 session（`accounting::views::snapshot`、`accounting::views::snapshot::start`、`accounting::views::sessions`，形状 7 投影；`crates/sprawling/Spec.lean` §8-144，`crates/wire/Spec.lean` §8-71）

```rust
// accounting::views
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Views { /* 折叠状态，私有 */ }
impl Views {
    pub fn twin(&self) -> Result<Views, AxError>;   // Ok(self.clone())
}
// accounting::views::snapshot::start
/// 一次从已证明的历史起步，和这一路逐行核对过的行数（证明的与折叠的合计）。
pub struct Audited<F> { pub started: Started<F>, pub lines_checked: u64 }
pub fn start_audited<F: SnapshotFold>(ledger_dir: &Path) -> Result<Audited<F>, AxError>;
// accounting::views::sessions
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub(crate) struct RoomSessions { /* 每个地址一列 wire::SessionLine */ }
impl RoomSessions {
    pub(crate) fn absorb(&mut self, record: &EventRecord) -> Result<(), AxError>; // 读不出 session_opened 的载荷即拒
    pub(crate) fn answer(&self, room: &Address) -> wire::SessionsAnswer;
}
```

(a) **第二份视图是克隆。** 折叠线程轮换两份视图（`crates/sprawling/Spec.lean` §8-99），第二份原先经快照编码复制：编码再解码，再把共享的句柄接回去。`Views` 的每个字段都能 `Clone`：折出来的字段深拷贝；账本索引、计划缓存、金库与停机值都在 `Arc` 里，克隆之后仍与原份共享；`machine`、`registry`、`upstream`、`programs` 照值带过去。所以 `#[derive(Clone)]` 给出的正是 `twin` 原先的结果，字段清单仍只写在 `Views` 的定义里。40 万行夹具城上，编码再解码 350 ms，克隆 22 ms（`crates/sprawling/Spec.lean` §8-144）。为此 `storage::HotView`、`storage::Attribution`、`Governance` 与 `CommitFacts` 派生 `Clone`。

(b) **一次性重建每行只核对一遍。** `start_audited` 先判起点，再决定要不要先证明：
- 快照合身（`SnapshotStart::Resume`）：先 `storage::prove_chain`（只读记录），`Whole` 才解码快照、折尾部；快照之前的行起步时不看，只有这次证明看。
- 不能用快照（`SnapshotStart::Whole`）：直接从创世全量折叠。全量折叠经 `runtime::replay::fold_ledger_dir` 让每一行过同一个 `LineCheck`，它的判定就是不带记录的证明的判定，先证明一遍只是同一批行多核对一次。

`Audited.lines_checked` 是证明逐行核对的行数加上折叠核对的行数（尾部行数，或从创世折到的最后一行的 `seq + 1`）。没有快照、没有记录时它等于账本行数，不是两倍；快照合身、记录写满时它等于记录之后长出的行数加尾部行数，与历史长度无关。两条都在 `worker::folds::views_start::tests` 以两种规模断言，这是整城重建的回退门；墙钟只进 `budgets.toml`。

(c) **每个地址的各段 session 是视图里的一张表。** `accounting::views::sessions::RoomSessions` 为每个地址存一列 `wire::SessionLine`，`Views::apply` 每行调一次 `RoomSessions::absorb`：`session_opened` 在它的地址下开新的一段；`run_started` 在一个还没有任何一段的地址下开一段 `Dispatched`，否则给当前这一段的 `runs` 加一；任何带地址的记录都把这个地址当前这一段的 `last`、`at` 挪到自己。没有地址的记录不碰这张表。`Query::Sessions { room }` 由 `RoomSessions::answer` 在锁内作答（`crates/wire/Spec.lean` §8-71）。表随视图快照存取，所以视图快照的编码变了，`VIEWS_FOLD_RULES` 随之重取（`crates/sprawling/Spec.lean` §8-91）；旧快照解不开就按 `WholeFold::Damaged` 从创世折一次，不报错。
-/

/-!
### 8-24 快照格式的进位由夹具摘要钉住，夹具里有每一种出现在快照里的摘要（`accounting::views::snapshot`、`accounting::worker::folds::standing_start`，形状 7 投影；`crates/kernel/Spec.lean` §8-84）

```rust
// accounting::views::snapshot
const VIEWS_FOLD_RULES: &str = "views-fold-<16 位十六进制>";       // 视图夹具编码的 blake3 前缀
// accounting::worker::folds::standing_start
const STANDING_FOLD_RULES: &str = "standing-fold-<16 位十六进制>"; // Standing 夹具编码的 blake3 前缀
```

- **门只看得见夹具里有的类型。** 两个常量的后缀是一份固定夹具经快照编码之后的 blake3 前缀，编码一变，`the_fold_rules_name_carries_the_digest_of_the_views_encoding` 与 Standing 的同名测试给出新值并失败（`crates/sprawling/Spec.lean` §8-91）。夹具里没有 `GitOid`，摘要的编码从十六进制改成字节时摘要不动，旧快照就会以同一个 `fold_version` 交给新的解码：postcard 把一个长度前缀加 40 个十六进制字符当成 20 个字节读，读错的视图可能照样解得开。
- **所以夹具折进每一种出现在快照里的摘要。** 视图的夹具多折一条 `checkpoint_committed`（`commits`、`commit_seqs`、`last_commit` 各存一个 `GitOid`）与一条 `rules_changed`（`governance.rules` 存一个 `B3Hash`）；Standing 的夹具多折同一条 `rules_changed`（它的 `governance` 是同一个类型）。kernel 里摘要的编码再变，两道门都变红，旧快照按 `WholeFold::OtherFoldVersion` 从创世折一次。
- **EndpointBook 同时保存成功 Session 亲和与未成功调用的非秘密账号身份**，实时与重放共用 `crates/gateway/spec/Router.lean` 的绑定规则；格式更改必须重取两个 fold rules 摘要，旧编码从 genesis 重建。
- **两个格式夹具都登记 Provider**：旧凭据登记与显式有序账号一起进入 EndpointBook，夹具编码覆盖缺席的账号字段与带账号的字段；实际 Views 快照往返比较完整端点簿，检测字段省略造成的 postcard 解码错位。
- **摘要在快照里是字节。** `views::snapshot::tests` 断言一个 `GitOid` 与一个 `B3Hash` 的快照编码就是它们的 20 与 32 个字节、读回相等，JSON 拼写仍是十六进制（`crates/kernel/Spec.lean` §8-84）。
- **被否：夹具不动，改编码的人记得改常量。** 这正是两个常量带摘要后缀要免掉的那种记忆；而这次的改动在 kernel，改它的人看不见 accounting 的常量。
-/

/-! D31 视图的第二份、一次性重建与房间的各段 session

**(a) 视图的第二份按值克隆，不经快照编码。** 理由：两份视图要的是同一个折叠状态加上同一组共享句柄，派生的 `Clone` 正好如此，而编码再解码在 40 万行城上要 350 ms，是开城最长的一段之一（§8-19）。被否决的做法：①留在编码路径上，把复制挪到视图线程——首字节不再等它，但视图线程开头的每一批照样等 350 ms，而且要改装配根起线程的次序；②手写逐字段复制——字段清单的第二份拼写，加一个字段就要改两处。**(b) 重建从创世时不先证明。** 理由：全量折叠逐行核对每一行，判定与不带记录的证明相同；先证明再全量折叠是同一批行核对两遍（§8-19）。被否决的做法：照旧先证明，把证明的结果交给全量折叠跳过核对——折叠要的是每一行解析出的记录，跳过核对仍要解析，省下的只是规范回显的比较，却让「这一行核对过」有了两处来源。 **(c) 房间的各段 session 由视图折叠，表按地址存在 `views::sessions`。** 理由：作答不读盘，表的大小与 session 数同阶（wire D9）。被否决的做法：把这张表并进 `worker::folds::SessionOrigins`——那张表回答的是派活要问的「这一段还欠不欠一段对话」，只留当前一段，worker 的 `Standing` 也不由页面读；把各段 session 并进 `lineage`——`lineage` 由读盘的 CLI 每次重建，服务中的城不持有它。
-/

/-! D36 快照的格式门用夹具摘要，夹具里放进每一种出现在快照里的 kernel 摘要类型（§8-24）

理由：`fold_version` 只在夹具的编码变了时才动，夹具缺哪一种类型，哪一种类型的编码就能悄悄改变，而旧快照照样被当成新格式读。被否决的做法：①为每个出现在快照里的类型各写一条字节数断言——那是编码的第二份拼写，加一个类型就要多写一条；②把快照格式的版本号写成手改的常量——改 kernel 的人看不见它。重开参数：快照换掉 postcard、或快照的编码有了自己的模式描述（schema）可以直接取摘要时，改由模式描述钉住版本。
-/

/-! ### 接口仍写在 sprawling 规格里的模块

下面这些模块的接口与取舍今天写在 `crates/sprawling/Spec.lean` 的这几节里，按标签列出；`architecture.toml` 里它们的行指向本分部，这张表把读者带到那一节。它们搬进本 crate 的规格是 D15 记下的下一步。

| sprawling 的标签 | 模块 |
|---|---|
| §8-91 | `accounting::views::snapshot`、`accounting::views::snapshot::start`、`accounting::views::snapshot::tests` |
| §8-122 | `accounting::views::snapshot::start::both` |
-/
