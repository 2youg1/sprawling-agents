-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# accounting::playback::traced

规定 playback 的时间选择、调用的耗时、运行策略与提交的证据：`playback::traced`、`playback::diff`，以及它们在 `playback::select`、`playback::links`、`playback::project` 与 `views::rounds::Attempts` 里的那一部分。本文件是 `crates/accounting/Spec.lean` 的一个分部；下面每一节保留它在 accounting 规格里的标签 §8-n，别处引作 `crates/accounting/Spec.lean §8-n`，决定引作 `accounting D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它要守住的性质证在 `crates/accounting/spec/Playback/Select.lean` 与 `crates/accounting/spec/Playback/Project.lean`，其余由 Rust 的类型与 `accounting::playback::tests` 守住（`crates/accounting/Spec.lean` §16）。

**平台**：时间条件只读每行信封的 `t`，不读墙钟；提交的证据只读 git 对象。下面每一句在 Windows、macOS 与 Linux 上相同。
-/

/-!
### 8-17 accounting::playback 的时间选择、调用的耗时、运行策略与提交的证据（形状 7 投影；`diff`、`traced` 为形状 4 适配器）

回看一段工作流的人还要四样东西：按 UTC 时间选一段；每次工具调用花了多久，以及这个数什么时候是量出来的；一次 run 要求了什么准入证据；一个提交改了什么、出自哪几次调用。本节定这四样在 bundle 里的形状与求法；泳道与播放怎样画是页面的事（`skills/playback/SKILL.md`）。CLI 与城工具读同一组条件（`crates/sprawling/Spec.lean` §8-143）。时间条件的性质在 `crates/accounting/spec/Playback/Select.lean`。

```rust
// accounting::playback
pub const DIFF_MAX_BYTES: usize = 64 * 1024;
impl Selection {
    /// 加上时间条件：信封 `t` 落在 `span` 里的行。
    #[must_use]
    pub fn during(self, span: runtime::clock::UtcSpan) -> Selection;
}
/// 一次导出给的时间条件，原文。
pub struct Window<'a> { pub since: Option<&'a str>, pub until: Option<&'a str>, pub day: Option<&'a str> }
impl Window<'_> {
    /// 三项取交集，成为一个 `UtcSpan`；没给任何一项时是不限的区间。
    pub fn span(&self) -> Result<runtime::clock::UtcSpan, AxError>;
}
```

模块：`playback::select`（时间条件与 `Window`）、`playback::links`（调用这一对）、`playback::project`（运行策略、提交证据的接入）、`playback::traced`（一行 checkpoint 持有什么；`Committed` 那一行的调用归属与基准，按读者能看到的写）、`playback::diff`（一个提交从基准到它、逐文件的 diff）。

**时间选择。**

- 一行在选择里，另须它信封的 `t` 在 `[since, until)` 里（`UtcSpan::contains`）。`t` 不随 seq 单调（kernel D10：四种等来的行各记各的时刻，墙钟也会回拨），所以逐行判断，走到 cutoff 为止，不在第一条越过 `until` 的行处停下（`Select.lean` 的 `a_line_whose_time_steps_back_is_judged_on_its_own`）。与 seq 区间、run、楼取交集。不认识的可忽略行仍只按 seq 判（§8-12）。
- `Window::span`：`since`、`until` 经 `runtime::clock::parse_iso` 读，只收 `iso` 写出的形状。`day` 是 `YYYY-MM-DD`，展开成 `[那一天 00:00:00Z, 次日 00:00:00Z)`：拼成 `<day>T00:00:00Z` 经 `parse_iso` 读，所以历法与校验仍是 `runtime::clock` 那一份，2 月 30 日照样被拒；上界是下界加一个 UTC 日的毫秒数（checked）。给了几项就交几项：下界取最晚的，上界取最早的，再交给 `UtcSpan::new`，它拒绝上界不晚于下界的区间。所以 `--day` 与 `--since`/`--until` 一起给时是交集，交出空区间就是矛盾的范围，以 `E_INVALID_ARGS` 拒绝；合法而一行都没选中的区间输出带范围信息的空 bundle。
- `source.selection` 多记 `since`、`until`（毫秒的十进制字符串，没给为 `null`）。`day` 只记成它展开的两端，因为复核按区间重算，同一个区间不该有两种写法。

**调用与耗时（`calls`）。**

- 一次工具调用是 links 里的一对（模型调用是另一种对，见 §8-25）：`tool_called` 的 `id` 打开、同一 run 里 `tool_result` 的 `tool_use_id` 关闭，与 `views::rounds` 配对用的是同一对键。两端的状态与关键时刻、消息一样是五种之一（§8-12）。关闭行继承打开行碰到的楼，所以一条调用碰到机密楼时，它的答复同样隐去。至少一端在范围内可见的调用才出现，按它在范围内的第一行的 seq 升序。
- 行：`run`、`callee`（工具调用写 `{"tool":{"id","name"}}`，`name` 在打开行可见时取它载荷的 `name`，否则 `null`；模型调用见 §8-25）、`called`、`answered`、`took`。
- `took` 是 `{"measured":"<毫秒>"}`，当且仅当两端都可见、`views::rounds::answered_timing` 判这一对的时刻是量出来的（两行都有 `EventRecord::moment`，答复不是城在重启后补写的 `E_TOOL_OUTCOME_UNKNOWN`），并且答复的 `t` 不早于调用的 `t`；其余一律是 `"unknown"`。所以版本早于逐行时刻的账本、混合区间里旧版本的那几次调用、补写的答复、还没答、有一端被隐去，都不给耗时，页面也就不会画出零耗时。判定量没量的规则只在 `views::rounds` 一处；逐行的精度仍是每条 `events` 的 `moment`，混合的区间逐行、逐调用各自保留。
- 一条两端都落在范围外、范围内又没有成员的调用，在它关闭时就从「可能成为上下文的范围外行」里删掉：调用占账本行数的大头，留着它们会让一次窄选择的常驻量与整本账同阶；删掉以后常驻的只有还没关闭的调用。

**运行策略（`runs[].policy`）。** 每个 run 行带这个 run 的 `run_started.policy`（`crates/kernel/Spec.lean` §8-77：`mode`、`write`、`admit`、`landing`），早于策略入账的行写 `null`。`admit` 是这次派活要求的准入证据；它的结果是这个 run 打开的 PR 怎样关闭：合并时准入不过写成 `pr_rejected`，`by` 与 `why` 是拒绝它的一方与理由（`crates/runtime/Spec.lean` §8-54），合并写成 `pr_merged`，带 `reviewed_commit` 与 `verified_by`。这些行在 `events` 里，关键时刻 `pr` 一项指向它们。要求与结局各是记下的事实，playback 不从一次合并推断测试跑过没有。

**提交的证据。** 范围内可见的每个 `Committed` checkpoint 多三项；读这三项要读账本之外的输入（git 对象与 `accounting::trace` 对整本账的折叠），确定性的条件是这些输入相同，而 git 对象按 oid 不可变：

- **`trace`**（`playback::traced`）：这一行是 cutoff 以内第一次宣告这个 oid 的行时，是 `accounting::trace` 按这一行求出的答（8-25：视图折到这一行时的 `Query::Commit`，调用与同楼的别人经 walk 的索引读），写 `{"traced":{"calls","nearby"}}`：`calls` 是这个 run 在区间里的每次调用，各是 `{"at":seq}`（在 `events` 里）、`{"elsewhere":seq}`（读者看得到，在选择之外，行不随 bundle 携带，要看它就把选择放宽到它）或 `"withheld"`（读者看不到那条 `tool_called`）；`nearby` 是同一栋楼里别的 run 的调用数，各是 `{"run","actor","calls"}`，`run` 照 `runs` 的写法是 `{"run":id}`、`"withheld"` 或 `"missing"`，`actor` 只在 `run` 读得到时写出。这一行之前已经宣告过同一个 oid（同一棵树再次宣告，它的调用归在第一次宣告上），或视图答「没有这个提交」时，写 `"untraced"`；`trace` 失败（视图拒绝折 cutoff 以内的某一行，或这一段的行读不了）写 `{"unread":"<错误码>"}`，导出不因此失败。这两种情形下没有 `trace` 的答，`base` 是 `"none"`，`diff` 为空表。
- **`base`**：`trace` 答的 `commit.previous`（同一 run 的上一个提交），写 `{"previous":oid}`；没有时，`commit.parents` 恰好一个，写 `{"parent":oid}`；否则 `"none"`。全城紧邻的上一个提交不是基准：它可能属于别的 run。
- **`diff`**（`playback::diff`）：`base` 是 `"none"` 时为空表；否则按 `files` 的次序，每个路径一项 `{"path","change"}`。`change` 是六种之一，互不混同：
  - `{"patch":{"lines","credential"}}`：经 `storage::of_file(city_root, base, Head::Commit(oid), path)`（`storage::hunks`）读出的整段 patch。它只比较两个不可变的 oid，从不读工作区；`lines` 是 `{"number","text"}`，`credential` 是被凭据扫描隐去的行的 `{"number","reason"}`，与 `storage::hunks` 同一个扫描，不回显字节。
  - `{"truncated":{"lines","credential","cut"}}`：这一个提交显示的 patch 文字超过 `DIFF_MAX_BYTES`，这个文件只给出预算之内的头几行，`cut` 是没给出的行数；第一行放不下之后，这个文件余下的行都不给出，即使其中有更短的行。之后的文件照样读、照样分类，只是它们的行都在预算之外。
  - `"empty"`：两个提交之间这个文件没有动。
  - `"binary"`：patch 没有文字行，而提交（文件被删时是基准）里的这个 blob 是二进制。
  - `"missing"`：仓库不在城根、它没有这两个对象之一，或 patch 读不出（`of_file` 的任何失败）。
  - `"withheld"`：这个路径所在的楼对读者关闭，或这个路径读不成一个地址（说不出它在哪栋楼，就当作关闭），文件不读。
  预算按 UTF-8 字节计，只限 bundle 里 diff 的文字，不限 git 读取本身。

**失败。** `Window::span` 的失败是 `E_INVALID_ARGS`：`since`/`until` 是 `parse_iso` 的拒绝；`day` 读不了时 action 是 `select a playback day`，subject 是原文，recovery 给出 `2026-05-14` 的写法；交集为空时是 `UtcSpan::new` 的拒绝。`diff` 与 `trace` 从不让导出失败：账本之外的输入缺了、读不了，各有一个写明的状态。

**资源。** 时间条件只多读每行信封的 `t`，walk 已经解析了它。调用表常驻每个有成员的调用一项，加上还没关闭的范围外调用。每个范围内、第一次宣告的 `Committed` 提交问一次折到它的视图、按 walk 的索引读一段（代价见 §8-25），再对 `files` 里每个路径读一次 git：一份 patch 一次物化，显示的部分不超过 `DIFF_MAX_BYTES`。

**本节测试**：`accounting::playback::tests::span`：`t` 回退的行按它自己的时刻取舍；`day` 与 `since` 取交集，交出空区间时拒绝，`day` 读不了时拒绝；不重叠的合法区间给出空 bundle 且 `source.selection` 记着两端。`accounting::playback::tests::model`：`Select.lean` 场景表里带时间的几项在生产的 `export` 上给出同样的 seq。`accounting::playback::tests::evidence`：旧版本的调用与新版本的调用在同一区间里，前者 `took` 为 `"unknown"`、后者是量出来的毫秒，补写的答复为 `"unknown"`；run 行带它的策略，PR 的拒绝行在 `events` 里；一个提交的 `diff` 把文字、凭据行、空、二进制、缺失、隐去分开，基准是同一 run 的上一个提交；它的 `trace` 把区间里的调用按读者能看到的写出；导出之后别的 run 再宣告同一个提交，复核仍逐字节相同。
-/

/-!
### 8-25 accounting::playback 的模型调用耗时，与只读到 cutoff 的提交证据（形状 7 投影；`views::rounds::Attempts` 为形状 1 决策）

§8-17 给工具调用配了耗时，给提交配了调用归属。这一节补两样：模型调用（一条 `model_called` 到答它的 `model_returned`）的耗时；提交的证据只读到 cutoff——cutoff 之后的行写了什么、审不审得过，都改变不了 bundle，所以 `check --city` 在一段 cutoff 之后才坏掉的历史上照样逐字节重现。必须守住的性质在 `crates/accounting/spec/Playback/Project.lean`：`took_is_measured`、`an_unmeasured_end_is_unknown`、`evidence_ignores_lines_after_the_cutoff`。

```rust
// accounting::views::rounds（形状 1 决策）
/// 每条 `model_returned` 答的是它的 run 在它之前最近的一条 `model_called`。
#[derive(Debug, Default)]
pub(crate) struct Attempts { /* 私有：每个 run 最近一次尝试的 seq */ }
impl Attempts {
    pub(crate) fn note(&mut self, record: &EventRecord);
    pub(crate) fn answered_by(&self, reply: &EventRecord) -> Option<Seq>;
}

// accounting::trace
/// 一座城的历史，按严格校验走过的次序一行一行折进视图。
pub(crate) struct History { /* 私有：折到当前行的视图，或第一次拒绝 */ }
impl History {
    pub(crate) fn new(city_root: &Path) -> History;
    pub(crate) fn absorb(&mut self, record: &EventRecord);
    /// 折到当前行为止的视图答 `Query::Commit` 的那个值；没有一行宣告过它时 `Ok(None)`。
    pub(crate) fn commit(&self, oid: GitOid) -> Result<Option<wire::CommitAnswer>, AxError>;
}
pub(crate) fn trace_through(index: &LedgerIndex, ledger_dir: &Path, commit: wire::CommitAnswer) -> Result<Trace, AxError>;
```

模块：`views::rounds`（`Attempts`）、`playback::links`（调用的第二种对）、`playback::document`（`Callee`）、`trace`（`History`、`trace_through`）、`playback::walk`（交回它建的索引）、`playback::traced`（`Evidence`：在 walk 里折历史，记下每个第一次宣告的答）。

**模型调用（`calls`）。**

- 一次模型调用是 links 里调用的第二种对：键是 run 与那条 `model_called` 的 seq；关闭它的是同一 run 的 `model_returned`，它答哪一条由 `views::rounds::Attempts` 判：这个 run 在它之前最近的那条 `model_called`。rounds 的 `turns` 把答复放进回合时也经 `Attempts`，所以页面的回合与 bundle 的调用用同一条配对规则。每次尝试都入账，修复后的重发是第二条 `model_called`（`crates/runtime/spec/Turn/Recovery.lean` §8-50），被它替下的那次尝试不会有答复，写 `"pending"`，`took` 为 `"unknown"`。
- 行：`run`、`callee`、`called`、`answered`、`took`。`callee` 是两种之一：`{"tool":{"id","name"}}`（工具调用，§8-17）或 `{"model":{"name"}}`（模型调用：打开行可见时是 `ModelCalled.model`，请求里写的端点自己的模型 id，否则 `null`）。
- `took` 与工具调用同一条规则（§8-17）：两端都可见，`views::rounds::answered_timing` 判两端量过（两行都有 `EventRecord::moment`），答复的 `t` 不早于调用的 `t`，才写 `{"measured":"<毫秒>"}`，其余一律 `"unknown"`。模型的答复不带 `error`，所以「城在重启后补写的答复」这一条对它不成立；规则仍只在 rounds 一处。`ModelReturned.first_at` 是首个内容到达的时刻，不进 `took`：它答的是另一个问题（首字延迟），那一行在 `events` 里，页面要时自己读。
- 两端都在范围外、范围内没有成员的模型调用，与工具调用一样在关闭时从候选里删掉（§8-17）。

**只读到 cutoff 的提交证据。**

- `playback::walk` 交回它经 `storage::LedgerIndex::folding` 建出的索引，`project` 把它交给证据那一步。cutoff 只有一处定义，是 walk 停下的那一行（§8-12）；证据那一步不另读一遍账本，也不另建索引。
- `playback::traced::Evidence` 在 walk 里收下每一条核对过的行（`Walked::Known`）：先折进 `trace::History`，再记下这一行是不是第一次宣告它点名的提交（`views::commits::commit_facts`）。范围内可见的 `Committed` checkpoint 是第一次宣告时，在折完这一行的那一刻问 `History::commit(oid)`：此刻的视图恰好折到这一行，`previous` 是这个 run 在它之前宣告的最近一个提交，run、`actor`、`seq` 就是这一行自己的，`parents` 与 `message` 由 git 按 oid 给出。答只取决于这一行与它之前的历史，以后的宣告与 cutoff 之后的行都改变不了它（`evidence_ignores_lines_after_the_cutoff`）。
- walk 结束后，每个这样的答交给 `trace::trace_through`，经 walk 的索引读这个 run 在这一行之前的行与区间里的行，区间规则是 §8-16 那一条；读到的 seq 全都小于这一行，所以不越过 cutoff。
- `History::absorb` 遇到视图拒绝折的第一行时停下，记住那条拒绝；之后每次 `commit` 都答它，这些提交写 `{"unread":"<错误码>"}`，导出不因此失败（§8-17）。视图折的是 walk 已核对过的同一批记录，被拒绝的行在 cutoff 以内，所以一份 bundle 里的 `unread` 只取决于 cutoff 以内的历史，复核时照样重现。
- `whose --trace`（`crates/sprawling/Spec.lean` §8-136）不变：`trace(city_root, oid)` 照旧经 `views::ask` 求这个 oid 最近一次的宣告、重建一次索引，再经 `trace_through` 按同一条区间规则读。

**代价。** 一份 bundle 只折一遍视图，与 walk 同一遍、同一批记录；它从不调用 `views::ask`，也不重建索引。每个范围内第一次宣告的提交另有一次视图查询（在内存里），它经 `Prepared::finish` 两次打开仓库：读父提交（`storage::parents_of`）与提交消息（`views::commits`，`crates/sprawling/Spec.lean` §8-128），再按索引读它的 run 的行（倒着读到下界前最近的 `model_called`）与区间里的行。判同楼的别人时从区间的下界起读索引（`LedgerIndex::seqs_from`，`crates/storage/Spec.lean` §8-38），读到区间之后的第一行为止：一个提交读过的索引项是它的区间长度加一，与它之前的账本有多长无关，一次导出在这一项上的代价是各区间长度之和，不是提交数乘行数。确定性计数：`accounting::playback::tests::tracing` 在 N 与 2N 个提交的历史上各导出一次，数这次导出开始了几次视图折叠，两种规模下都是 1；再数一个提交判同楼的别人时最多读过几条索引项，两种规模下都是 6（第一个提交的区间从它的 run 的第一行起，五行，加上区间之后那一行）。两个数都只在测试里编译（`trace::counted`），理由与 D37 (c) 相同：要挡住的是按 N 增长的代价，墙钟在小夹具上看不出它。毫秒读数由同一文件的仪表 `instrument_evidence_cost` 给出（`cargo nextest run -p sprawling-accounting --release --run-ignored only -E 'test(instrument_evidence_cost)' --no-capture`）：50、100、200 个提交（1,003、2,003、4,003 行）的一次导出依次约 100–115、160–170、365–430 ms，随提交数线性增长。

**在 40 万行的城上。** 读数取自 40 万行夹具城（`bench_startup first-byte` 的 `l400k`：400,003 行、64,000 个提交；windows-x86_64、16 线程的笔记本处理器、release，`opt-level = 3`）：`sprawling playback export` 选 `--from 2 --through 8`（范围里没有提交）26.8–27.2 s，选 `--from 399000`（1,003 行、约 160 个提交）26.9–27.0 s，选 `--from 380000`（20,003 行、约 3,200 个提交）31.2 s，各三次。同一座城上分段计时（在 `opt-level = "z"` 下量，那时同一次导出是 34 s）：只走 walk（逐行核对）5.5 s；walk 加视图折叠 6.6–7.3 s，所以视图折叠约 1.1–1.8 s，是一次导出的 4% 上下；walk 加整个投影 33–37 s，其中 credential 扫描约 25 s、`named_in_payload` 2.3 s、视图折叠 1.4 s、`links` 0.9 s。导出的秒数因此不在视图折叠里，而在投影对每一行的扫描（§3），范围里没有提交时不折视图最多省下这 4%（D37 (b)）。范围里每多一个提交，另付约 1.3 ms（读父提交、区间里的行与同楼的别人）。

**失败。** `History::commit` 的失败是视图拒绝折某一行的那条 `AxError`；`trace_through` 的失败是读行的 `StorageError`（经 `into_ax`）、一行解析不了、actor 落在保留子树里（同 §8-16）。两者在 playback 里都写成 `unread`。

**本节测试**：`accounting::playback::tests::tracing`：旧版本的一次模型调用、被重发替下的一次尝试、量过的一次与没有答复的一次在同一区间里，`took` 依次是 `"unknown"`、`"unknown"`（答复端 `"pending"`）、量出来的毫秒、`"unknown"`；导出之后在 cutoff 之后追加两行、改坏其中第一行，`check --city` 的来源一项仍通过；N 与 2N 个提交时一次导出都只开始一次视图折叠，一个提交判同楼的别人时读过的索引项也不随 N 变。`accounting::playback::tests::evidence` 的调用表照 `callee` 的形状写。
-/

/-! D29 playback 的时间条件比信封 `t`、逐行判断，`--day` 展开成同一个区间；调用的耗时只在 rounds 判为量出来时给出；提交的证据经 `trace` 与 `storage::hunks` 读，读不到就写明读不到

(a) 时间条件读每一行信封的 `t`，与 `view --since` 与 `--until`（`crates/sprawling/Spec.lean` §8-137）同一个 `UtcSpan`，不按种类去载荷里挑时间字段，也不靠 `t` 有序提前停或二分。理由：`t` 就是这一行记下的那一刻，任何种类都有；它不随 seq 单调，在第一条越过 `until` 的行处停下会漏掉回退的行。被否决的做法：只对四种记时刻的行判时间、其余行跟着它所在的回合走（同一件事两个家，且旧账本里根本分不出回合的边界）；在索引里存时间列再二分（要 `t` 有序）。

(b) `day` 拼成 `<day>T00:00:00Z` 交给 `parse_iso`，加一个 UTC 日的毫秒数作上界；与 `since`/`until` 同给时取交集。理由：历法与校验只有 `runtime::clock` 那一份，日期另写一个解析器就是第二份；一天在 UTC 里总是 86 400 秒（Unix 时间不计闰秒）。交集而不拒绝同给，是因为「这一天里九点以后」本来就是一个合法的问题。被否决的做法：`day` 与 `since`/`until` 互斥（把一个合法的问题拒掉）；在本模块写一个 `YYYY-MM-DD` 解析器。

(c) 工具调用作为 links 的第四种对，键是 run 与调用 id；耗时由 `views::rounds::answered_timing` 判量没量，playback 只做减法。理由：配对的两端状态（在范围内、范围外、隐去、未答、缺）与关键时刻、消息是同一套，放进 links 就不必为调用另写一份；「补写的答复不算量过」这条规则在 rounds 里，把它搬成一个函数让两处共用，就不会在 rounds 改它时分开。代价：关闭行继承打开行碰到的楼，一条碰过机密楼的调用，它的答复也隐去，比以前多隐去一些行。被否决的做法：把 `views::turns` 用在选中的行上（回合的开头可能在范围外，turns 会丢掉没有回合的调用，也不交出答复那一行的 seq）；让页面拿两条 `moment` 自己相减（读者看不出补写的答复，旧行与真的零毫秒也分不开）。

(d) 提交的调用归属只经 `accounting::trace`，问的是第一次宣告的那一行：区间按这一行求；再次宣告的行写 `untraced`；范围外的调用只给 seq（`elsewhere`），行不进 `context`。理由：归因的权威是 `accounting::trace`，playback 再写一份区间规则就是两个家；`trace` 按视图里最近一次宣告求区间，而没有改动的检查点会把同一棵树再宣告一次，别的 run 也会，所以导出之后的一次宣告就能让复核与原 bundle 不同——两名居民的验收先撞上了这一点。按第一次宣告那一行求，答只取决于那一行之前的历史。范围外的调用要进 `context`，就得在整遍读里把所有调用行留着，常驻量与整本账同阶。被否决的做法：直接用 `trace`（导出之后的宣告改变复核）；在 playback 的折叠里按「同一 run 上一个提交」自己求区间（第二个家）；把归属里的调用行都放进 `context`。这一行的答怎样只读到它自己、不越过 cutoff，见 D37。

(e) diff 的基准是同一 run 的上一个提交，没有时才用唯一的父提交；文件的六种状态分开写，凭据扫描是 `storage::hunks` 那一个；读不到 git 不让导出失败。理由：全城紧邻的提交可能属于别的 run，git 的父提交也可能出自别人或人自己（与 D28 同一条理由）；缺失、二进制、空、截断、隐去对读者是不同的事，混成一个「没有 diff」就让读者把「没动」读成「读不到」。账本是 bundle 的核心，git 是附件：附件缺了，核心那一段照样能回看。被否决的做法：对比工作区（不是历史）；用全城紧邻的 checkpoint 作基准；git 读不到就整个导出失败。

(f) 字段一变，`SCHEMA` 与 `PROJECTION_RULES` 一起进位；读回时先只读 `schema`。理由：多了字段是形状变了，旧构建读不懂新 bundle，新构建也读不懂旧的；先看 `schema`，结构一项报的是「这是第几版、用哪一版复核」，而不是一条字段缺失。被否决的做法：只进 `PROJECTION_RULES`（旧 bundle 在解析时就失败，到不了「复核不了」那一步，报出来的是一条难懂的字段错误）。
-/

/-! D37 模型调用的耗时按 rounds 的那一条配对规则求；提交的证据在 walk 里折到宣告行时问视图，再经 walk 的索引读行，所以只读到 cutoff（§8-25）

(a) 模型调用作 links 里调用的第二种对，答复由 `views::rounds::Attempts` 配到它的 run 最近一条 `model_called`，`turns` 也经它配；`calls` 的行换成 `callee` 两种之一，`SCHEMA` 进到 `sprawling.playback/3`，`PROJECTION_RULES` 进到 4。理由：「一条答复答哪次尝试」原来只写在 `turns` 的 `folded.last_mut()` 里，playback 再写一份就是第二个家；做成两处共用的一个小值，rounds 改这条规则时 bundle 跟着改。模型调用没有 id，键用 `model_called` 的 seq：它在一本账里唯一，复核时也不变。用 `callee` 而不是在原来的行上加一个可空的 `model` 字段，因为工具调用的 `id` 对模型调用没有意义，可空字段会让读者分不清「没有」与「读不到」。被否决的做法：①让 `wire::Turn` 交出答复行的 seq（线上形状变，`WIRE_V` 进位，换来的只是 playback 少写几行）；②页面拿 `events` 里两条 `moment` 自己相减（看不出哪条答复配哪次尝试，一次重发会被读成一次很长的调用）；③把 `first_at` 当耗时（那是首字延迟，另一个问题）。

(b) 提交的证据从 walk 里逐行折的 `trace::History` 读：范围内第一次宣告的 `Committed` 行，在折完它的那一刻问 `Query::Commit`；调用与同楼的别人经 walk 建的索引读。理由：cutoff 的唯一定义是 walk 停下的那一行；视图折的是 walk 核对过的同一批记录，问的是折到宣告行时的视图，所以答只取决于那一行之前的历史，cutoff 之后的行写了什么、审不审得过都碰不到它。一遍折叠代替了每个提交一次 `views::ask`（每次审一遍整条链、折一遍视图）与一次 `LedgerIndex::rebuild`，一份 bundle 的代价不再是提交数乘账本字节数。被否决的做法：①给 `trace` 一个停在某个 seq 上的 `views::ask`（要 storage 的整链审计与快照起点都能停在一个 seq 上，那是 storage 的公开契约；而且每个提交仍各折一遍）；②walk 之后从创世再折一遍到 cutoff（多读一遍账本，读到的字节不是这一遍核对过的）；③在 playback 里只折提交，按「同一 run 上一个提交」自己求 `previous`（那是 `views::commits` 那条规则的第二个家，D29 (d) 已否决）。代价：没有提交的窄选择也要折一遍视图；在 40 万行的城上这一遍约 1.1–1.8 s，是一次导出的 4% 上下（§8-25），所以照现在这样折。被否决的还有④范围里没有提交时不折：walk 读到范围之后才知道范围里有没有提交，要么从创世再读一遍到第一个提交（同②，读到的字节不是这一遍核对过的），要么先把行留在内存里（40 万行的城要几百 MB）。重开参数：视图折叠在导出里占到一半以上时（例如投影的 credential 扫描变快之后，§3），再看「遇到第一个范围内的提交才开始折」。

(c) 「一次导出开始了几次视图折叠」由一个只在测试里编译的计数读出：`trace` 里开始一次折叠的两处（`trace` 的 `views::ask` 与 `History::new`）各数一次，`playback::tests::tracing` 在 N 与 2N 个提交上比较。理由：要挡住的回退是「每个提交又把整本账折一遍」，墙钟读数在小夹具上看不出它，计数与机器快慢无关。被否决的做法：只记墙钟读数（看不出按 N 增长的代价）；经 storage 的 `Vfs` 缝数读了几遍段（那是 storage 的接口，这里不改它）。
-/
