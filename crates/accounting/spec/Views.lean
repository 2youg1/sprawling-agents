-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# accounting::views 与 accounting::lineage

规定 `crates/accounting/src/views.rs` 的公开面与 `crates/accounting/src/lineage.rs`。本文件是 `crates/accounting/Spec.lean` 的一个分部；下面每一节保留它在 accounting 规格里的标签 §8-n，别处引作 `crates/accounting/Spec.lean §8-n`，决定引作 `accounting D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `accounting::views::answering::stored::tests`、`accounting::views::versions::tests`、`accounting::views::tests`等 守住。
-/

/-!
### 8-10 accounting::views 与 accounting::lineage：页面问的每个问题，从折叠里答（形状 7 投影）

```rust
// accounting::views
pub struct Views { /* 折叠状态，私有 */ }
impl Views {
    pub fn new(city_root: &Path) -> Views;
    pub fn over(ledger_dir: &Path) -> Views;
    /// 先审计整条链，再从合适的快照起步、只折尾部（`crates/sprawling/Spec.lean` §8-91）。
    pub fn rebuild(ledger_dir: &Path) -> Result<Views, AxError>;
    pub fn apply(&mut self, record: &EventRecord) -> Result<(), AxError>;
    /// 锁内只取小数据；读盘、读库、出网在 `Prepared::finish` 里做（`crates/sprawling/Spec.lean` §8-100）。
    pub fn prepare(&self, query: &wire::Query) -> Prepared;
    pub fn twin(&self) -> Result<Views, AxError>;

    // 服务中的城从外面交进来的五样东西（`views::served`）。都不由记录折出，重建不碰它们，`twin` 把它们带到另一份。
    pub fn found_on_this_machine(&mut self, report: wire::DoctorAnswer);
    pub fn lend_the_vault(&mut self, vault: Arc<Mutex<gateway::Custodian>>);
    pub fn ask_the_registry_through(&mut self, newest: fn() -> wire::ReleaseAnswer);
    pub fn ask_upstream_through(&mut self, newest: fn(&str) -> wire::DoctorUpstream);
    pub fn look_for_harnesses_through(&mut self, find: fn(&str) -> Option<PathBuf>, place: fn(&agent_protocols::SetUpDir) -> Option<PathBuf>);
}
pub enum Prepared { /* 锁放开之后还要做的那一步 */ }
impl Prepared { pub fn finish(self) -> wire::Answer; }
pub struct Published { /* 私有 */ }
pub fn answer_outside_the_lock(views: &Published, query: &wire::Query) -> (Seq, Result<wire::Answer, AxError>);
/// 不 serve 一座城，只从它自己的历史答一问。
pub fn ask(city_root: &Path, query: &wire::Query) -> Result<wire::Answer, AxError>;
pub fn turns<'a>(records: impl IntoIterator<Item = &'a EventRecord>) -> Vec<wire::Turn>;
pub struct Governance { /* 读侧那一份治理折叠 */ }
pub fn pursued(record: &EventRecord) -> Result<Address, AxError>;

// accounting::lineage
pub struct Lineage { /* … */ }
pub struct RunLine { /* 公开字段不变 */ }
pub fn lineage_of(ledger_dir: &Path) -> Result<Lineage, AxError>;
```

- **读面对这台电脑只有五个入口，都经 `views::served` 交进来。** `machine` 是城启动后 doctor 看到的那一眼；`vault` 是 worker 打开的那一个；`registry` 与 `upstream` 各问一次网络；`programs` 回答「这台电脑的搜索路径上有没有这个程序」。五个都是服务中的城交的，所以一份没人 serve 的 `Views`（重建、`ask`、测试）对它们一律答 `Unavailable`，不去碰这台电脑。
- **harness 页经 `Views.programs` 找程序。** `Query::Harnesses` 在快照放开之后作答：`Some(find)` 时对 `agent_protocols::Harness::ALL` 里每一家的启动程序调一次 `find`，`found` 是它有没有交回一条路径；`None` 时答 `Unavailable { query: "Harnesses" }`。生产交的是 `bin::doctor::host::find_program`，它读的是 doctor 读的同一条搜索路径（`host::search_path` 加 `probe::on_search_path`），所以 harness 页与 doctor 对同一个程序给同一个答案。钉住它的测试是 `a_harness_is_looked_for_through_the_search_the_views_were_handed` 与 `a_harness_page_nobody_served_answers_unavailable`（`accounting::views::served::tests`）。
- **对 `sprawling` 公开的是这一节列出的面。** 模块在 `sprawling` 里时 `pub(crate)` 的条目，搬过来以后是 `pub`：装配根、服务面与二进制照原样读它们。`Views::answer` 仍只在本 crate 的测试里存在；`sprawling` 的测试写 `prepare(&query).finish()`，那是生产走的同一条路。
- **`lineage` 与 `views` 同住本 crate，因为读者跨两处。** `sprawling view` 的 run 列表在二进制里，playback 的共享投影在本 crate 的读面里；二进制够得到本 crate，本 crate 够不到二进制。
- **依赖**：`views` 折叠 `storage::HotView`、`storage::Attribution` 与 `storage::LedgerIndex`，快照起步经 `runtime::replay::fold_ledger_dir`，所以本 crate 依赖 `storage` 与 `runtime`（ARCHITECTURE.md §3 的 `depmap`，D14）。
-/

/-! D13 harness 页找程序经 `Views.programs` 这个 `fn` 指针，不经 `Machine`，也不在开城时算好

理由：这一问读的是此刻的搜索路径，与 `registry`、`upstream` 同形——没有状态、服务中的城交一次、`None` 就答 `Unavailable`（D10）；它不启动任何程序，所以不必等 `DoctorRefresh`。被否决的做法：给 `Machine` 加一个方法——`Machine` 属于 worker，读面拿不到它，而且 doctor 的逐项查法已经在 `bin::doctor::Machine::look` 里，再加一个方法就是第二条查法；在开城时把 harness 的有无算进 `DoctorAnswer`——那是一个线上的形状改动，而且人在 harness 页上装完一个程序，要等到下一次 `DoctorRefresh` 才看得到它。
-/

/-! D14 `views` 搬进来时，本 crate 加 `storage` 与 `runtime` 两条边

理由：`views` 折叠的就是 `storage` 的 `HotView`、`Attribution`、`LedgerIndex`，快照起步与 worker 的 `Standing` 共用 `runtime::replay::fold_ledger_dir` 的同一遍；worker 搬过来后本来也要这两条边（D11）。被否决的做法：把这两处读经端口交进来——端口会把一份折叠的权威分到两个 crate，与 D9 否决的是同一件事；把 `fold_ledger_dir` 挪进 `storage`——那改的是 `runtime` 的公开面，与这次迁移无关。
-/

/-! D16 `views` 的测试经 `worker::fixture::init_city` 造城，与 worker 的测试是同一个创世

理由：页面读到的城就是创世留下的城；测试用真正的 `worker::genesis::form`（带测试的手），市政厅的布局、创世两行与 `City.md` 改了，读面的测试跟着看到。被否决的做法：保留一份只写读面测试读到的东西的第二份创世——worker 搬进本 crate 之前确实这样做过，因为那时本 crate 够不到 `genesis`；它与真正的创世没有东西把两者拴在一起，一旦市政厅的布局变了，读面的测试就在一座不存在的城上判定。
-/

/-! ### 接口仍写在 sprawling 规格里的模块

下面这些模块的接口与取舍今天写在 `crates/sprawling/Spec.lean` 的这几节里，按标签列出；`architecture.toml` 里它们的行指向本分部，这张表把读者带到那一节。它们搬进本 crate 的规格是 D15 记下的下一步。

| sprawling 的标签 | 模块 |
|---|---|
| §8-17 | `accounting::views::governance` |
| §8-20 | `accounting::views::answered` |
| §8-37 | `accounting::views`、`accounting::views::holding`、`accounting::views::answering`、`accounting::views::published`、`accounting::views::asking`、`accounting::views::answering::history`、`accounting::views::archives`、`accounting::views::lines`、`accounting::views::tests`、`accounting::views::tests::folding`、`accounting::views::tests::roadmaps`、`accounting::views::tests::released`、`accounting::views::tests::released_ledger`、`accounting::views::governance_tests` |
| §8-39 | `accounting::views::building_page` |
| §8-50 | `accounting::views::evidence`、`accounting::views::cost_of` |
| §8-52 | `accounting::views::listing`、`accounting::views::hunks`、`accounting::views::standing_tests` |
| §8-53 | `accounting::views::commits` |
| §8-57 | `accounting::views::hearing` |
| §8-66 | `accounting::views::served`、`accounting::views::mcp_health` |
| §8-67 | `accounting::views::prefix`、`accounting::views::skills`、`accounting::views::git_status` |
| §8-100 | `accounting::views::city`、`accounting::views::prepared` |
| §8-105 | `accounting::lineage` |
-/
