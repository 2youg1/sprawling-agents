-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# accounting::playback

规定 `crates/accounting/src/playback.rs` 与它的 bundle：`playback::select`、`playback::reader`、`playback::walk`、`playback::project`、`playback::links`、`playback::document`、`playback::encode`、`playback::consistency`。必须守住的性质在 `crates/accounting/spec/Playback/Select.lean` 与 `crates/accounting/spec/Playback/Project.lean`。本文件是 `crates/accounting/Spec.lean` 的一个分部；下面每一节保留它在 accounting 规格里的标签 §8-n，别处引作 `crates/accounting/Spec.lean §8-n`，决定引作 `accounting D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；必须守住的性质证在上面两个分部，其余由 Rust 的类型与 `accounting::playback::tests` 守住（`crates/accounting/Spec.lean` §16）。

**平台**：导出只读账本、楼的规则文件与检查点仓库，不读墙钟，不写盘（落盘见 §8-13）；下面每一句在 Windows、macOS 与 Linux 上相同。
-/

/-!
### 8-12 accounting::playback：一段历史成为一份可以重算的 playback bundle（形状 7 投影）

`playback` 把一座城 Ledger 的一段折成一份 **playback bundle**：带来源的、字节确定的 JSON，人或 agent 拿它回看一段工作流。它是账务读面上的一个共享投影，CLI（`sprawling playback export/check`，`crates/sprawling/Spec.lean` §8-126）与以后的居民工具都是它的薄适配器。必须守住的性质的权威是 `crates/accounting/spec/Playback/Select.lean`（选择、次序与去重、cutoff 不读未来）与 `crates/accounting/spec/Playback/Project.lean`（读不到的行不流进派生表、真实关闭的单调性）；本节是接口与做法。

```rust
// accounting::playback
pub const SCHEMA: &str = "sprawling.playback/3";
pub const PROJECTION_RULES: u32 = 4;
pub const BUNDLE_MAX_BYTES: usize = 32 * 1024 * 1024;
pub enum Cutoff { Latest, At(Seq) }
pub struct Request { pub selection: Selection, pub reader: Reader, pub cutoff: Cutoff }
pub struct Bundle { /* 规范字节，私有 */ }
impl Bundle {
    pub fn bytes(&self) -> &[u8];
    pub fn digest(&self) -> B3Hash;
    pub fn events(&self) -> usize;
}
/// 只读：严格校验从创世到 cutoff 的每一行，再投影。
pub fn export(city_root: &Path, request: &Request) -> Result<Bundle, AxError>;
// check、Report、Verdict 与页面见 §8-13。

// accounting::playback::select（形状 1 决策）
pub struct Selection { /* 私有：first、last、run、building、span（时间条件，§8-17） */ }
impl Selection {
    pub fn everything() -> Selection;
    /// seq 闭区间 [first, last]；first > last 以 E_INVALID_ARGS 拒绝。
    pub fn new(first: Option<Seq>, last: Option<Seq>, run: Option<RunId>, building: Option<Address>) -> Result<Selection, AxError>;
}

// accounting::playback::reader（形状 1 决策）
pub enum Confidential { Withheld, Included }
pub enum Reader { Person(Confidential), Resident(Address) }
```

模块：`playback`（索引、`export` 与 `check` 的入口）、`playback::select`、`playback::reader`、`playback::walk`（严格校验一遍、固定 cutoff）、`playback::project`（折叠）、`playback::links`（关键时刻与消息的两端）、`playback::document`（bundle 的 schema，形状 6 数据）、`playback::encode`（规范序列化、安全嵌入编码、摘要）、`playback::consistency`（一份 bundle 自洽的判定）、`playback::check`。

**快照与选择。**

- **cutoff 在导出开始时固定。** `walk` 经 `storage::LedgerIndex::folding` 读段、经 `storage::LineCheck::advance` 逐行判定，从创世连续走到最后一个完整行（`Cutoff::Latest`）或到给定 seq（`Cutoff::At`，`check --city` 用）；被选择条件排除的行照样过 `LineCheck`，缺行、坏链、重复 seq、版本超前都在这一遍上报错，整个导出失败，不产出任何字节。尾部半行按 storage 既有的读取规则不算一行，导出不修账。本模块不自己数段文件，也不信索引判断缺行。
- **范围内事件**恰好是 seq ≤ cutoff 与选择的交集（`Select.lean` 的 `mem_selected`）：`--from`/`--through` 是含端点的 seq 闭区间，`--run` 比 `record.run()`，`--building` 用 `Address::is_within` 比信封地址（没有地址的行不属于任何一栋楼，`lab` 不匹配 `laboratory`），时间条件比信封的 `t`（§8-17）。区间越过 cutoff 时只读到 cutoff，`source.selection` 照原样记下给的条件。合法的空选择输出带范围信息的空 bundle。

**读界与隐去。**

- **一行碰到的楼**：信封地址的楼；它所在 run 的房间的楼（`run_started`/`run_forked` 的信封地址）；它关闭的那一对的打开行碰到的楼（`approval_resolved` 继承 `approval_requested`，`signal_consumed` 继承 `signal_enqueued`，`pr_merged`/`pr_rejected` 继承 `pr_opened`，`run_frozen` 继承 `run_started`）；载荷里任一字符串，只要它是一个 `Address`、头一段是这本账到此为止立过的楼（`building_created` 的 `addr`，或出现过的信封地址的头一段）。楼由 `city::Building::of` 求。
- **判定复用 `kernel::ReadVerdict` 的三臂。** `Reader::Resident(b)` 调 `kernel::address::may_read(b, 楼, 规则)`；`Reader::Person(Confidential::Withheld)` 对每栋楼问同一份规则：`confidential = false` 为 `Open`，`true` 为 `Confidential`，读不了为 `RulesUnreadable`；`Reader::Person(Confidential::Included)` 全部 `Open`。规则经 `city::RulesCache` 读，按楼缓存一次导出。碰到的楼全是 `Open` 的行才可见；另外两臂关闭。规则是导出时刻的规则。
- **凭据扫描仍生效。** 可见的行再过 `kernel::secret::scan`（`storage::hunks` 用的同一个扫描）；命中的行按凭据隐去，只计数，不回显字节。
- **派生表只从可见行求。** 事件、上下文、run、关键时刻、消息、费用与 checkpoint 全由可见行求出；被隐去的行只进 `withheld`：条数、种类计数、关闭的楼名与原因（`confidential`、`rules_unreadable`）、凭据隐去的条数。这些是明示的元数据披露，不宣称隐藏机密活动的存在；自由文字里转述的秘密，地址规则认不出来。
- **不认识的可忽略行**（更新的写者、`ig:true`）只给 seq，列进 `unknown`，载荷一字不出；它们没有可读的 run 与地址，所以 seq 在区间内就列出，不论 `--run`/`--building`。

**bundle 的内容**（`playback::document`，字段按下面的次序写出）：

| 段 | 内容 |
|---|---|
| `schema` | `SCHEMA` |
| `source` | `city`（`storage::Provenance::city_of`，创世行的链哈希）、`selection`（给的条件：`from`、`through`、`run`、`building`、`since`、`until`）、`cutoff`（seq 与该行的 `chain_hash`）、`rules`（`PROJECTION_RULES`）、`reader`（`{"person":"withheld"}`、`{"person":"included"}` 或 `{"resident":"<楼>"}`） |
| `events` | 范围内的可见行，按 seq 升序、各一次：`seq`、`moment`（`EventRecord::moment`，账本版本给出的逐行时刻依据；`null` 是没量过，不是零耗时）、`line`（账本原行，逐字节） |
| `context` | 范围外、cutoff 以内、可见、被引用的行（run 的首行、关键时刻与消息范围外的那一端），同形 |
| `unknown` | 区间内不认识的可忽略行的 seq |
| `runs` | 有范围内事件的 run，取 `accounting::lineage::Lineage` 折到 cutoff 的那一行：`run`、`addr`、`session`、`parent`、`forked_at`、`predecessor`、`first_seq`、`last_seq`、`state`、`unanswered`、`policy`（§8-17）；`parent`/`predecessor` 是 `{"run":id}`、`"withheld"`（那个 run 的房间读者读不到）或 `"missing"`（本账到 cutoff 没有它） |
| `moments` | 关键时刻：`family`（`run`、`approval`、`pr`）、稳定键（run id、approval id、`<branch>@<pr_opened 的 seq>`）、`opened`、`closed`、`seqs`（范围内可见的成员） |
| `messages` | 每封信（`signal_enqueued` 的 id）：`from`、`room`、`sent`、`consumed` |
| `calls` | 每次工具调用（run 与调用 id）与每次模型调用（run 与它的 `model_called` 的 seq）：`run`、`callee`、`called`、`answered`、`took`（§8-17、§8-25） |
| `checkpoints` | 范围内可见、点名一个提交或钉住一份 job 的行，按 seq 升序：`checkpoint_committed` 的 `JobPinned` 写 `{"pinned":{"job":…}}`，`Committed` 写 `{"committed":{"oid","scope","files","base","diff","trace"}}`（后三项见 §8-17）；`pr_merged` 写 `{"merged":{"oid"}}`，`oid` 是落地的提交。哪些行点名提交、点名的是哪个 oid，由 `accounting::views::commits::commit_facts` 一处回答 |
| `costs` | 范围内可见行上折的 `storage::Attribution`：`billed_usd_micros`、`by_run`、`unpriced_calls`、`unpriced_tokens`；只涵盖所选可见范围 |
| `withheld` | 见上 |

一端的状态分五种，互不混同：`{"at":seq}`（在范围内）、`{"outside":seq}`（cutoff 以内、范围外，行在 `context`）、`"withheld"`、`"pending"`（关闭端到 cutoff 还没有出现）、`"missing"`（打开端不在本账到 cutoff 的历史里）。关键时刻、消息与调用只在至少一个成员在范围内可见时出现。闭合只看真实的关闭事件（`run_frozen`、`approval_resolved`、`pr_merged`/`pr_rejected`、`signal_consumed`），窗口的右端不是关闭（`Project.lean` 的 `closedAt` 不读选择）。PR 按 `branch` 与打开它的那一行识别，所以同一分支重开是另一个关键时刻。

**规范字节与安全嵌入。** `playback::encode` 是唯一的序列化：serde 按结构体字段次序写紧凑 JSON，再把字符串里的 `<`、`>`、`&`、U+2028、U+2029 写成 `\u` 转义，所以同一份字节原样放进 HTML 的 `<script type="application/json">` 也不会提前结束那个块。所有 u64（seq、时刻、金额、计数）写成十进制字符串，JS 的 `Number` 不经手它们；`rules` 是小整数。摘要是这份字节的 BLAKE3（`B3Hash::digest`）。读回时先比尺寸上限，再按 `deny_unknown_fields` 解析，再重新编码并与原字节逐字节比较：重复键、多余空白、字段次序、未知字段、非规范的十进制都在这一步被拒。

**自洽与来源复核。** 一份 bundle 自洽，是指它按上一段读得回，并且：`schema` 是 `SCHEMA`；`events` 与 `context` 各自 seq 严格递增、互不相交；每条 `line` 过 `storage::read_line` 且 seq 与条目一致；每个 `{"at":seq}` 指向 `events`，每个 `{"outside":seq}` 指向 `context`；`runs` 的每个 run 在 `events` 里出现过。自洽只说这份文件内部不矛盾，不说它没被改过。复核有两种：

- **与另一份 bundle 比**：两份都自洽时逐字节比较，不同时给出第一个不同的段。
- **与它的城比**：读者由调用入口给出，不信 bundle 自述。`source.rules` 不是本构建的 `PROJECTION_RULES`、`source.reader` 与入口的读者不同、城在 cutoff 之前就结束，都是「复核不了」，并说明要用哪个版本或哪个读者重新导出；否则按 `source.selection` 与 `Cutoff::At(source.cutoff.seq)` 用同一个投影重算并逐字节比较。改摘要、改费用、删事件而保留 `source`，重算的内容就不同。
- 链与摘要不提供签名，也不证明现实世界里的陈述为真：整本账与 bundle 一起被换掉时没有外部信任根。

两种复核与页面的检查怎样分项报出，见 §8-13。

**失败。** 全部是 `AxError`：`Selection::new` 的矛盾区间、bundle 读不懂或不规范是 `E_INVALID_ARGS`（action `select a playback range` / `read a playback bundle`）；链上的行按 `LineFault::into_ax` 报（`E_CAS_CORRUPT`、`E_LOG_VERSION_UNSUPPORTED`），recovery 指向 `sprawling replay`；序列化后超过 `BUNDLE_MAX_BYTES` 是 `E_INVALID_ARGS`，recovery 是用 `--from`/`--through`、`--run` 或 `--building` 收窄；规则、payload 读不了按各自的 `AxError` 原样上抛。任何失败都不交回部分的 bundle。错误文字不回显被隐去的内容。

**资源。** 一遍读，一个段的字节常驻；另外常驻的是 `Lineage`（每个 run 一行）、关键时刻与消息的两端（每个键一项）、可能成为上下文的范围外可见行（run 的首行与各对的两端），范围内的投影，以及折到当前行的视图（§8-25）。`BUNDLE_MAX_BYTES` 只限最终字节，不限这些常驻量。32 MiB 是待测的初值：多日夹具上的导出峰值与读取成本量出来之前，不把它当作内存上界。

**版本。** bundle 的内容或某张表的求法每变一次，`PROJECTION_RULES` 进一位，于是旧构建导出、本构建读得开的 bundle 复核时报「复核不了」，而不是报「不同」。字段增减是形状的变化，`SCHEMA` 随之进一位：读回时先只读 `schema` 一个键，不是本构建的 `SCHEMA` 就在结构一项报出两个版本、要求用写它的那个版本复核，而不报一条字段缺失。

**模型与实现的比较。** `Select.lean` 的 `scenes` 是一张场景表（选择、cutoff、应选中的 seq），`scenes_agree` 证明模型对表里每一项给出那组 seq；`playback::tests::model` 从同一个 `.lean` 文件读出这张表，对每一项跑生产的 `export` 并比较 seq。表只在 Lean 里写一次。这是行为比较，不是 Rust 实现的精化证明。
-/

/-! D24 playback 是账务读面上的一个投影，按整行判定可见，逐字节携带账本行，只读一遍严格校验过的字节，复核靠重算

(a) 一行可见，当且仅当它碰到的每一栋楼对读者都是 `Open`；碰到的楼由信封地址、run 的房间、关闭的那一对的打开行与载荷里以已知楼开头的地址求出，一条规则管所有事件种类。理由：读界要对未来新加的种类也关着，一张按种类列可公开字段的表，每加一个种类就要加一行，漏一行就漏字段；整行判定漏不了。代价是一行只要碰到一栋关闭的楼就整行隐去，连同它本可公开的字段。被否决的做法：按种类逐字段投影（维护面随种类增长，缺行时无声地开或关）；只按信封 `addr` 删行（`approval_resolved` 记在 city run 上、`addr` 为空，handback 的内容来自子 run）。

(b) `events` 里每条是账本原行的字符串，外加十进制字符串的 `seq` 与 `moment`。理由：原行就是账本的权威字节，读者可以对它重算 `chain_hash`；把记录展开成 JSON 对象会让 seq、`t` 与金额在 JS 的 `Number` 里丢精度，也等于第二种写法。被否决的做法：展开成对象、u64 写成数字。

(c) 严格校验自己走一遍 `LedgerIndex::folding` 加 `LineCheck::advance`，不用 `runtime::replay::fold_ledger_dir`。理由：导出要每一行的原字节（cutoff 行的链哈希、逐字节的 `line`、凭据扫描）和不认识的可忽略行的 seq，`fold_ledger_dir` 两样都不交出；事后按索引重读原行，会把审过的字节和重读而未审的字节混在一起。被否决的做法：`fold_ledger_dir` 加按索引重读。

(d) `check --city` 用同一个投影重算并逐字节比较，读者取入口给的。理由：只比 `source` 与末行链哈希时，保留 `source` 而改摘要、删事件都比不出来；信 bundle 自述的读者，就能用一份伪造的 `{"person":"included"}` 扩大权限。被否决的做法：比末行链哈希；按 bundle 的 `reader` 重算。

(e) PR 的关键时刻键是 `<branch>@<pr_opened 的 seq>`，关闭行关掉同一分支最近打开的那一个。理由：请求在账本上的身份就是分支（`collab::OpenRequest`），同一分支会重开；只用分支作键会把两次请求并成一个。被否决的做法：只用分支。

(f) `checkpoints` 里哪些行点名一个提交、点名哪个 oid，问 `accounting::views::commits::commit_facts`（它对本 crate 可见）；`JobPinned` 不点名提交，按 `CheckpointCommitted` 自己的类型读出，与 `Committed`、`pr_merged` 分开写。理由：识别提交的权威只能有一个，`views` 的提交页与 playback 的表必须对同一行给出同一个答案；`pr_merged` 的 oid 写在手写键里，抄一份读法，两处就会在那个键改名时分开。被否决的做法：在 playback 里再写一份识别提交的匹配。

(g) 人的入口在 `Confidential::Withheld` 时按楼的规则取三臂，而不调 `may_read`。理由：`may_read` 要一个读者所在的楼，人不住在任何一栋楼里；为了调它而编一栋楼，会让「人的楼」成为一个不存在的地址。三臂的类型仍是 `kernel::ReadVerdict`，居民入口仍调 `may_read`。被否决的做法：给人编一个地址。
-/
