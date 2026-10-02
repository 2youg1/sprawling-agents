-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# accounting::trace

规定 `crates/accounting/src/trace.rs`。本文件是 `crates/accounting/Spec.lean` 的一个分部；下面每一节保留它在 accounting 规格里的标签 §8-n，别处引作 `crates/accounting/Spec.lean §8-n`，决定引作 `accounting D<n>`。
-/

/-!
### 8-16 accounting::trace：一个提交倒推到它之前的那些调用（形状 7 投影）

`trace` 回答拿着一个提交 oid 的人在 `whose` 之后问的下一句：这个提交是哪几次调用的结果。CLI（`sprawling whose --trace`，sprawling-SPEC.md §8-136）是它的薄适配器；下一轮 playback 的调用归属与验收工具从坏提交归因到写它的居民，都读同一个值。

```rust
// accounting::trace
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trace {
    pub commit: wire::CommitAnswer,   // 与 `Query::Commit` 的答是同一个值：run、actor、previous、parents
    pub calls: Vec<wire::Call>,       // 这个 run 在区间里的调用，按 seq 升序
    pub nearby: Vec<Nearby>,          // 同一栋楼里别的 run 在区间里的调用，每个 run 一项，按 run id 升序
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nearby { pub run: RunId, pub actor: Address, pub calls: u64 }
/// 只读。这座城没写过这个提交时 `Ok(None)`。
pub fn trace(city_root: &Path, oid: GitOid) -> Result<Option<Trace>, AxError>;
/// 经 `index` 读 `commit` 的 run 的行与区间里的行；只读 seq 小于 `commit.seq` 的行。
pub(crate) fn trace_through(index: &LedgerIndex, ledger_dir: &Path, commit: wire::CommitAnswer) -> Result<Trace, AxError>;
// 逐行折到宣告行时答 `Query::Commit` 的 `History` 见 §8-25。
```

- **提交的事实从 `views::ask` 来。** `trace` 先问 `Query::Commit`，这一步审计整条链、折叠视图（sprawling-SPEC.md §8-41），所以「这座城写没写过它」与 `whose` 同一个答案；答不是 `Answer::Commit` 时就是 `Ok(None)`。
- **区间。** 上界是宣告这个提交的那一行（不含）；下界是 `commit.previous` 那一行（不含），没有 `previous`（这是这个 run 的第一个提交）时是这个 run 的第一行（含）。`previous` 是同一个 run 上一个宣告的提交（`views::commits` 的折叠）：别的 run 在中间的提交不是界，git 的父提交也不是，因为父提交可能出自别的 run 或人自己。
- **调用。** 这个 run 在区间里的 `tool_called`，经 `views::turns` 与各自的 `tool_result` 配对成 `wire::Call`：工具、subject、参数、结局、输出、`effect`、两个时刻与 `timing` 的读法只有 rounds 那一份。`turns` 只把一条 `model_called` 之后的调用归进回合，而区间的下界可以落在一个回合中间，所以往回读过下界之后，继续读到下界之前最近的一条 `model_called` 为止（含），折完再按 `Call.at` 只留区间里的调用。读的是本 run 的行（`LedgerIndex::run_seqs_before`），不读别的 run。
- **同楼的别人。** 区间里信封 `run` 不是这个 run、信封地址在 `city::Building::of(commit.actor)` 那栋楼里（`Address::is_within`）的 `tool_called`，按 run 计数；`actor` 是这个 run 在区间里第一条这样的行的地址。同一栋楼共用一棵工作树，这些 run 的写也可能落进这个提交，所以它们是本 run 之外的候选。只计数，不列调用。
- **第一次宣告。** 同一个 oid 可以被再次宣告（一次没有改动的检查点交回同一棵树），别的 run 也可以宣告它；视图为这个 oid 记的 run、地址与 seq 是最近那一次，`previous` 是第一次时的那个。`trace` 回答的是最近那一次。playback 要的是第一次宣告的那一行：它在视图折到那一行时问 `History::commit`，此刻视图里这个 oid 的 run、`actor`、`seq` 就是那一行的，再经 `trace_through` 按同一条区间规则求；所以它的答只取决于那一行及其之前的历史，以后的宣告与 cutoff 之后的行都改变不了它，playback 的复核才能逐字节重现（§8-17、§8-25）。
- **代价。** `ask` 之后再建一次 `LedgerIndex`，再按索引读两段：本 run 的行（倒着读到下界前最近的 `model_called`）与区间里的每一行（判同楼的别人）。读的行数与区间长度成正比；建索引与 `ask` 的审计各与账本字节数成正比，与一次性的 `whose` 同阶。playback 不走这条路：它不调用 `ask`、不重建索引，只按 walk 建的索引读这两段（§8-25）。
- **失败。** `ask` 的失败原样上抛；建索引与读行的 `StorageError` 经 `into_ax`；一行解析不了按 `EventRecord::parse_line` 的错误上抛，审过的链上出现它，说明账本在 `ask` 之后被改了；actor 落在保留子树里时 `Building::of` 的拒绝上抛（城写的提交不会落在那里）。
-/

/-! D28 `whose --trace` 的逻辑是读面上的一个模块 `accounting::trace`，从 `Query::Commit` 的答出发再读账本，不加线上查询；同楼的别人只计数

理由：区间的两端已经在 `CommitAnswer` 的 `seq` 与 `previous` 里，调用的读法已经在 `views::turns` 里；今天的读者是读盘的 CLI，下一轮的 playback 与验收工具都在本 crate 里或经本 crate 读。同楼别的 run 的写也可能落进这个提交，但把它们的调用与本 run 的并列，会把「候选」读成「原因」，所以只给条数与地址，要细看的人拿 `view --run` 去读。被否决的做法：①加 `Query::Trace`：线上多一个形状、`WIRE_V` 进一位、`wire.ts` 与 adversary 的门面都要跟着改，换来的只是把这几步搬到服务端，而 CLI 本来就读盘；②按 `Call.effect` 只留写调用：读调用决定了写什么，去掉它们就去掉了归因的一半证据，`effect` 留在每条调用上由读者判断；③区间以 git 的父提交或全城紧邻的上一个提交为界：两者都可能属于别的 run，会把别人的调用算成这个 run 的。重开参数：页面要显示一个提交的调用时（那时要一个线上查询，本模块搬到 `views` 后面作答）；或同一栋楼里几个 run 同写一棵树成为常态、条数不够区分时。
-/
