-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import crates.city.spec.Archive
import crates.city.spec.Building
import crates.city.spec.Check
import crates.city.spec.CityTool
import crates.city.spec.ConfigLayers
import crates.city.spec.ConfigLayers.Ladder
import crates.city.spec.ConfigLayers.Remote
import crates.city.spec.Document
import crates.city.spec.Gitignore
import crates.city.spec.Governed
import crates.city.spec.History
import crates.city.spec.Identity
import crates.city.spec.Library
import crates.city.spec.Library.Audit
import crates.city.spec.Library.Install
import crates.city.spec.Neighbourhood
import crates.city.spec.NeighboursTool
import crates.city.spec.Policy
import crates.city.spec.Resident
import crates.city.spec.Room
import crates.city.spec.RulesTool
import crates.city.spec.Schedule
import crates.city.spec.Session
import crates.city.spec.SpineFiles
import crates.city.spec.Vocation
import crates.city.spec.Watch
import crates.city.spec.Wizard

/-! # city 的规格

`sprawling-city`（库名 `city`，目录 `crates/city`）是城的空间与身份面：楼与房间、居民的身份、楼的规则、三层配置、楼里的文档与书架，以及城写下的第一批字节。

本文件是 crate 的规格入口，分部在 `spec/` 下，布局见 ARCHITECTURE.md §11「Specifications in Lean」。接口一节一节写在规定它的那个模块的分部里，每一节保留它的标签 §8-n，别处引作 `crates/city/Spec.lean §8-n`；本文件 §8 列出每个标签住在哪个分部。标签被别的规格与 rustdoc 的引用锚住，所以不重排。决定写作 `D<n>`，放在它所管的声明正上方，或它所管主题的那个分部里，别处引作 `city D<n>`；D7、D12、D14、D15 是空号，§12 末尾列出其余每条住在哪里。

能写成定理的规则在分部里证明，Lean 模型是「必须守住哪些性质」的权威，Rust 代码是「怎样守住」的权威：配置梯子（`spec/ConfigLayers/Ladder.lean`）、写配置先过读者（`spec/ConfigLayers.lean`）、一个地址归哪栋楼管与建楼的三道拒（`spec/Building.lean`）、楼规的求值与写域（`spec/Policy.lean`）、读—判—换的锁（`spec/Document.lean`）、近的书架盖远的（`spec/Library.lean`）、审核绑在内容摘要上（`spec/Library/Audit.lean`）、自带 skill 只在成形时放一次（`spec/Library/Install.lean`）、治理文件在一切写域之外（`spec/Governed.lean`、`spec/Policy.lean`）、谁规划（`spec/Vocation.lean`）。其余分部只有节注释：它们写的是接口的形状、取舍与被否的备选，由 Rust 的类型与各模块旁的测试守住（§16）。
-/

/-! ## 1 需求分解

本 crate 是城的空间与身份面：

| 模块 | 这个模块回答的问题 | 分部 |
|---|---|---|
| `resident` | 谁在跑这个 Run（身份从哪来、给 prefix 贡献什么、做过什么） | `spec/Resident.lean` |
| `identity` | 城怎么称呼人与主 Agent，一个 session 冻下哪一版 | `spec/Identity.lean` |
| `policy` | 这栋楼里允许什么（confidential 四条、写域、出网） | `spec/Policy.lean` |
| `building` | 一栋楼怎么被建出来，以及一个地址归哪栋楼管 | `spec/Building.lean` |
| `config_layers` | 三层配置住哪三个文件，又怎么求成一份 `FrozenConfig` | `spec/ConfigLayers.lean`、`spec/ConfigLayers/Ladder.lean`、`spec/ConfigLayers/Remote.lean` |
| `spine_files` | 一栋楼开局有哪几份文档，一件活的 JOB.md 落在哪 | `spec/SpineFiles.lean` |
| `schedule` | 到点发车：谁在什么节奏上自己开始 | `spec/Schedule.lean` |
| `watch` | 盘上的文件变了，谁该知道 | `spec/Watch.lean` |
| `library`、`archive` | 书架上有什么、怎么装上去、审过没有；一条记录存哪里、怎么找回来 | `spec/Library.lean`、`spec/Library/Install.lean`、`spec/Library/Audit.lean`、`spec/Archive.lean` |
| `wizard` | 建城向导 | `spec/Wizard.lean` |
| `room` | 一个地址是不是房间，会话没指名时开哪一间 | `spec/Room.lean` |
| `session` | 一段会话开始时清掉什么 | `spec/Session.lean` |
| `neighbourhood`、`neighbours_tool` | 这座城有哪些地方，我身边站着谁，我该跟谁说话 | `spec/Neighbourhood.lean`、`spec/NeighboursTool.lean` |
| `rules_tool` | 居民怎么读写自己楼的规则 | `spec/RulesTool.lean` |
| `gitignore` | 一栋楼的哪些字节进历史 | `spec/Gitignore.lean` |
| `vocation` | 一个地址上的居民是来建造的还是来规划的 | `spec/Vocation.lean` |
| `city_tool` | 市政厅对城市本身的那一扇门 | `spec/CityTool.lean` |
| `governed` | 治理这座城的三份文件 | `spec/Governed.lean` |
| `document` | 一份文档整个换上去，或者旧的留着 | `spec/Document.lean` |
| `handoff_form` | 交接表单的四节怎么读 | `spec/SpineFiles.lean` |
| `check` | 一座城的每份 TOML，错处落到行列 | `spec/Check.lean` |
| `history` | 这个目录有没有城的历史 | `spec/History.lean` |
-/

/-! ## 2 验收标准

每个模块的验收是它分部里那几节的规则，各由模块旁的 `tests.rs` 经生产入口断言。身份这一面的验收：一个 Resident 跨两个 Run 存活，且**两次 Run 的 resident 段字节相同**（`a_resident_crosses_two_runs_with_the_same_identity_segment`，bin 侧从 `model_called` 的 segments 哈希取证）；无 `URBANITE.md` 的地址落为 Ephemeral 且段文本明说这一点。

分部里的定理是模型对性质的证明：

- `spec/ConfigLayers/Ladder.lean`：设置页答的值就是 run 被治理的值（`the_setting_page_and_the_run_read_one_value`）；最近说话的那一级胜并说出它是哪一级（`the_nearest_rung_that_speaks_wins`）；地址就是楼时读两级、房间读三级（`an_address_that_is_its_building_reads_two_rungs`、`a_room_reads_three_rungs`）；写得比够得到的那一级更近的表即拒、够得到的照读、城那一级够得到每一张（`a_table_written_too_near_is_refused`、`a_rung_reads_the_tables_it_reaches`、`the_city_states_its_own_tables`）；梯子只给拒词加上文件（`the_ladder_keeps_the_parsers_recovery`，D8 (a)）；会话记录压过梯子上的 harness（`a_session_that_opened_on_a_model_keeps_it`、`without_a_record_the_nearest_harness_runs`，D6）。
- `spec/ConfigLayers.lean`：写面交出的文本读者一定收下，被拒的写不动文件，读得懂的配置写多少次都读得懂（`a_change_lands_only_what_the_reader_accepts`、`a_refused_change_leaves_the_file`、`the_layer_on_disk_stays_readable`）；`[search]` 的 `Custom` 只接它选的那一家、选的不在列表里时不回落、在列表里时一定接得到（`a_custom_choice_reaches_only_its_selection`、`a_custom_choice_never_falls_back`、`a_listed_selection_is_reached`，D24）。
- `spec/Building.lean`：一个地址被它的楼管着，楼的楼是它自己（`a_building_holds_every_address_it_governs`、`a_building_is_its_own_building`）；保留子树不属于任何楼（`the_reserved_subtree_belongs_to_no_building`）；建起的楼恰是它自己的楼，房间与二次出生各拒（`a_created_building_governs_itself`、`a_room_is_not_a_building`、`a_second_birth_is_refused`）；移走的楼在一切写域之外（`a_removed_building_is_out_of_every_write_domain`）。
- `spec/Policy.lean`：不说 `confidential` 或 `write` 即拒（`a_rules_file_that_does_not_say_confidential_is_refused`）；机密楼没有出路（`a_confidential_building_has_no_way_out`）；机密楼的写域止于本楼，没写前缀就是整栋楼（`a_confidential_domain_stays_in_its_building`、`no_prefix_means_the_building_alone`）；楼规与桌面白名单在一切写域之外（`the_rules_are_out_of_every_write_domain`）；留着的规则就是现读的规则（`kept_rules_answer_what_a_read_would`，D3）。
- `spec/Document.lean`：不在的文档读作空（`a_missing_document_reads_as_no_bytes`）；文件动过即拒且不动（`a_moved_document_is_refused_and_left`）；同一版出发的两次保存只落先到的（`two_saves_from_one_version_land_once`）；锁里的 `n` 次加一恰好加了 `n`，锁外有反例（`increments_under_the_lock_add_up`、`without_the_lock_an_update_is_lost`，D17）。
- `spec/Library.lean`：一个名字留下最近一格书架上的那一件（`the_nearest_shelf_keeps_the_name`、`the_building_shelf_beats_the_city_and_the_outside`）。
- `spec/Library/Audit.lean`：显示为已审时轨迹里有对此刻这份摘要的审核（`audited_only_what_was_audited`），内容改成没审过的字节之后不显示为已审（`changed_content_is_never_shown_audited`）；取审核失败不改变书架（`failed_fetches_change_nothing`），也拦不住上架（`a_failed_fetch_never_blocks_an_install`，D19）。
- `spec/Library/Install.lean`：自带的每一件 skill 至多被放上书架一次（`a_builtin_is_placed_at_most_once`），成形之后被 User 拿下的一件不再回来（`a_removed_builtin_is_not_put_back`，D20）。
- `spec/Governed.lean`：三份治理文件在一切写域之外（`the_governed_documents_are_out_of_every_write_domain`）。
- `spec/Vocation.lean`：规划的恰是 City Hall（`only_the_hall_plans`）。

每个模型都带一个可实现的正常路径（一栋建得起来的普通楼、一栋列出网域名的普通楼、一次落下的保存、城那一层照读的表），所以这些保证不是从一个无法满足的前提推出来的。生产实现与模型的对应由 §16 列出的 Rust 测试检查；一条 Lean 定理证明的是模型，不是 Rust。
-/

/-! ## 3 假设与歧义

「Resident 住哪」取**地址即目录**：`<city>/<addr>/URBANITE.md`。Building 模板实例化后若改变目录形状，改的是 `urbanite_path` 一处。

**书架文档由页面写入（`PutShelved`）尚未实现，规则已定**：页面写的书架文档只经 `library::install`（§8-28，上架唯一的门），作为单文档包安装；写进已安装的包时生成新版本——暂存后整体替换（§8-27），不在原地改；每次写入在账本记一行 `skill_shelved`（kernel D23，`source = Page`；这个种类已在 kernel 的种类表里，今天只有新城成形放自带 skill 时写它）。wire 的 `PutShelved` 分支此刻仍答 `not_built`，客户端没有控件。

**升级后的模板过期检查尚未实现，规则已定**：`init` 布置的城文件（`crates/city/templates/` 下的模板，`City.md` 在内）带上布置时所用模板的标记（模板内容哈希）；新二进制打开旧城的路径上挂一个检查，标记与二进制所带模板不符即如实说出哪份文件过期，并给出更新命令。标记与检查的挂点都还没有代码。

**`kernel::LayeredValue::resolve` 在这里有一份 Lean 读法**：kernel 的规格把「居民格、楼格、城格依次」写成 `crates/kernel/spec/Config.lean` 里的散文，没有它的模型，所以 `spec/ConfigLayers/Ladder.lean` 定义了 `LayeredValue.resolve` 来陈述梯子的性质。它是那句散文的读法，不是第二条规则；kernel 的 `Config` 分部长出模型的那一次改动里，这个定义搬过去，梯子的分部改为 import 它。

模型自己的假设写在各分部的定理假设里，不写成公理：受保护的名字是参数 `protected_name`，它的唯一的家是 `kernel::address`（kernel D8）；`kept_rules_answer_what_a_read_would` 假设同一个戳读出同一份规则，这就是 D3 的重开参数；段的字符级文法、TOML 的文法与 serde 的拒词在模型之外，由 `Address::parse` 的判例表与各模块的测试守住。
-/

/-! ## 4 现状分析

二十四个顶层模块（§1）。生产消费者是 `crates/sprawling` 与 `crates/accounting`：装配层在派活时读身份、规则与配置梯子，在建楼、开房间、写会话时调本 crate 的写面；`city_tool`、`rules_tool`、`neighbours_tool` 注册进 bench。
-/

/-! ## 5 权威信源

「空间、身份、历史」的语义（Resident 是身份、活跃 Run 才是开销；一个地址决定三件事）；`crates/city/templates/URBANITE.md`（这份文件长什么样）；`architecture.toml` 里 city 那些条目。地址、保留子树与布局的模型是 kernel 的（`crates/kernel/spec/Address.lean`、`crates/kernel/spec/Layout.lean`），本 crate 的分部 import 它们而不重述。
-/

/-! ## 6 命名统一

Identity（两态）｜Resident｜Ephemeral｜Dossier｜URBANITE.md。**不引入「persona」「角色」「档案」**——概念名一律英文原词，一个概念一个名字。

Lean 里的名字与 Rust 的对应：

- `City.ConfigLayers.Ladder.Layer`／`Layer.ALL`／`read`／`tagged`／`resolve`／`file` ↔ `config_layers::ladder` 的 `Layer`、`Layer::ALL`、`Ladder::read`、`Ladder::tagged`、`Ladder::resolve`、`Layer::file`；`statedAt` ↔ `ladder::stated` 加上它调的 `in_file`；`Confined`／`Confined.nearest`／`reaches` ↔ `refuse::Confined` 与它的 `nearest`、`reached_from`（`Layer.depth` 是 Rust `Layer` 由远及近派生的 `Ord`），`statedAt` 的 `confined` ↔ `ConfigLayer::confined`、`tooNear` ↔ `refuse::too_near`；`settledHarness` ↔ `settled_harness`；`LayeredValue` ↔ `kernel::LayeredValue`。
- `City.ConfigLayers.change`／`land` ↔ `config_layers::write::change_at` 与它之后盘上的那份文件。
- `City.ConfigLayers.Search.Configuration`／`supplier` ↔ `SearchConfiguration`（kernel 的 `config::search`）与 `config_layers::search::search_supplier`。
- `City.Building.of`／`holds`／`create`／`head` ↔ `Building::of`、`Building::holds`、`building::create`，`head` 是 `of` 取首段的那一步；`removed` ↔ `building::removal` 的落点。
- `City.Policy.evaluate`／`writeDomain`／`load` ↔ `policy::evaluate`、`BuildingRules::write_domain`、`RulesCache::load`；`Written`、`Granted`、`DomainReach`、`BuildingRules` 与 Rust 同名。
- `City.Document.revise`／`againstBase` ↔ `document::revise`、`edit_against`。
- `City.Library.scan`／`shelve` ↔ `Library::scan` 与 `reading::shelve`、`shelve_external` 的插入。
- `City.Governed.Governed`／`place` ↔ `Governed` 与 `Governed::path`；`City.Vocation.vocation_of` ↔ `vocation_of`。
-/

/-! ## 7 模块边界

**三件邻居的活，及它们各自的主人**（写「X 归 Y」而非「不做 X」）：

- 落盘与历史归 `storage`：本模块**读** `URBANITE.md`，写入与备份归 storage 与 checkpoint。
- Building 规则（confidential、写域、阅览室准入）归 `city::policy`：本模块只答「谁」，不答「他能做什么」。
- 身份的**呈现**归客户端（`client/`）：Dossier 是数值，界面怎么画它是客户端的事。

workspace 内只依赖 `kernel`（ARCHITECTURE.md §3 的 `depmap`），规格也只 import kernel 的分部与本 crate 的分部。
-/

/-! ## 8 接口先行

每一节的接口写在规定它的模块的分部里。标签照旧，按标签找分部：

| 标签 | 分部 |
|---|---|
| 8-1 | `crates/city/spec/Resident.lean` |
| 8-2 | `crates/city/spec/Policy.lean` |
| 8-2b | `crates/city/spec/RulesTool.lean` |
| 8-36 | `crates/city/spec/RulesTool.lean` |
| 8-3 | `crates/city/spec/Building.lean` |
| 8-4 | `crates/city/spec/ConfigLayers.lean` |
| 8-4b | `crates/city/spec/ConfigLayers.lean` |
| 8-4c | `crates/city/spec/ConfigLayers.lean` |
| 8-5 | `crates/city/spec/SpineFiles.lean` |
| 8-6 | `crates/city/spec/Schedule.lean` |
| 8-7 | `crates/city/spec/Watch.lean` |
| 8-8 | `crates/city/spec/Library.lean` |
| 8-9 | `crates/city/spec/Archive.lean` |
| 8-10 | `crates/city/spec/Wizard.lean` |
| 8-11 | `crates/city/spec/Policy.lean` |
| 8-12 | `crates/city/spec/Library.lean` |
| 8-12b | `crates/city/spec/Library.lean` |
| 8-13 | `crates/city/spec/Room.lean` |
| 8-14 | `crates/city/spec/ConfigLayers.lean` |
| 8-14b | `crates/city/spec/Session.lean` |
| 8-15 | `crates/city/spec/Neighbourhood.lean` |
| 8-15b | `crates/city/spec/NeighboursTool.lean` |
| 8-20 | `crates/city/spec/Building.lean` |
| 8-21 | `crates/city/spec/Gitignore.lean` |
| 8-22 | `crates/city/spec/Vocation.lean` |
| 8-23 | `crates/city/spec/CityTool.lean` |
| 8-24 | `crates/city/spec/SpineFiles.lean` |
| 8-24b | `crates/city/spec/Governed.lean` |
| 8-25 | `crates/city/spec/Policy.lean` |
| 8-26 | `crates/city/spec/Policy.lean` |
| 8-27 | `crates/city/spec/Document.lean` |
| 8-28 | `crates/city/spec/Library/Install.lean` |
| 8-28b | `crates/city/spec/Library/Audit.lean` |
| 8-28c | `crates/city/spec/Library/Install.lean` |
| 8-29 | `crates/city/spec/Check.lean` |
| 8-30 | `crates/city/spec/History.lean` |
| 8-31 | `crates/city/spec/ConfigLayers.lean` |
| 8-32 | `crates/city/spec/Policy.lean` |
| 8-33 | `crates/city/spec/Identity.lean` |
| 8-34 | `crates/city/spec/Policy.lean` |
| 8-39 | `crates/city/spec/ConfigLayers/Remote.lean` |
| 8-40 | `crates/city/spec/Document.lean` |
| 8-41 | `crates/city/spec/SpineFiles.lean` |

`spec/ConfigLayers/Ladder.lean` 没有标签：它是 §8-4 那条梯子的模型。`spec/SpineFiles.lean` 末尾的「模板的写法」一节讲 `crates/city/templates/` 下每份模板的格式，同样没有标签。

**门面上换名**：`lib.rs` 按能力组织，不按文件组织，所以一个函数对外的名字可能与它所在模块里的名字不同（`config_layers::load` 对外是 `city::load_config`，`room::open` 对外是 `city::open_room`）；每一处换名写在规定它的那一节里。
-/

/-! D4 没有生产调用者的公开面不留

**决定**：本 crate 的每个公开函数都有一个生产调用者。所以没有单写强度的门（派活与会话都经 `write_session` 写强度），也没有搬家的判定（全仓没有一个动作会搬家）。

**理由**：一个只有测试在调的公开函数，是一条没有人在生产里执行的规则：它的测试证明的是一段不会跑的代码，而它的签名让读者以为那扇门是开的。单写强度的门还会让「强度写在哪一张表里」有第二个写者，与 `write_session` 各自拼 `[model] effort`。

**被否**：为将来的功能先立判定——搬家真的上线时，判定连同它的四条规矩（不是改名、历史留在原址、不落楼根、不碰保留子树）从 git 取回，那时它有第一个调用者，也就有第一个能判它对不对的测试。

**重开参数**：出现一个会搬家的动作，或一个没有会话记录却要写强度的生产调用点。
-/

/-! ## 9 工作流程

装配层派活（`accounting::worker::workbench::standing`；唤醒时 `accounting::worker::waking`）→ `Identity::load(city_root, addr)` → `segment_bytes()` 进 `FrozenPrefix` 的 resident 槽 → `who()` 成为 Ledger 的 actor。
-/

/-! ## 10 实现逻辑

纯 std：一次 `fs::read` 与一次 `B3Hash::digest`。`urbanite_path` 按地址分段 push，故不做字符串拼接，Windows 上也无分隔符问题。

### 选中与落选的设计

**A（选中）：`Identity` 两态枚举**。调用方拿到的东西自己会说自己是谁，段字节由它给出。
**B（落选）：`Option<Resident>`**。少一个类型，但每个调用方都要自己决定「None 时该给 prefix 什么」——那是一条散落在每个调用点的策略，且第一个忘记写的人会得到一个空的 resident 段。翻案条件：出现第三种身份（例如代表某人的临时身份），届时枚举照常扩，Option 则无法扩。

**安装的决定与动作（`library::install`，`crates/city/spec/Library/Install.lean` §8-28）**

**A（选中）：`plan_install` → `PlannedInstall::apply` 拆两步，`install` 是两者的合成。** 全部拒绝发生在决定里，动作只能从决定里拿到（`storage::worktree` 的 `plan_merge` 同形）。TOCTOU 复查问的正是「决定之后、落位之前，世界动了没有」——这道缝不打开，复查就无处可查；批准面也在这道缝上：人按 `PlannedInstall::hash()` 批准一个具体哈希再让它落地。
**B（落选）：单一同步 `install` 四步全吞。** 入口最窄，但「安装中途目录被换」在单扇门内无法被测试模拟，除非往产品代码塞一个测试钩子；而钩子进产品代码正是本仓库明拒的东西。翻案条件：落位改由携带代际计数的文件系统原语完成（复查不再需要人为制造的间隙），届时合成回单扇门。

**配置文件名（`crates/city/spec/ConfigLayers.lean` §8-4）**

**A（选中）：三层同名 `CONFIG.toml`，层级由位置决定**。读者记一个名字；把一份配置放错层是一个**位置**错误，而位置在目录树里看得见。
**B（落选）：每层各起一名**（`CITY.toml`／`BUILDING.toml`／`RESIDENT.toml`）。文件名自带层级信息，代价是三个名字要同时被记住，且放错层变成一个**拼写**错误——拼写错误要靠逐字比对才看得出。翻案条件：出现同一目录下共存两级配置的需求（例如一栋楼的默认与它自己作为房间的默认同处），届时位置不再能区分层级。

### 入窗的字节与它的代价

三层配置入窗零字节：`CONFIG.toml` 改的是请求字段（effort），不是模型读到的文本；新楼的 `RULES.toml` 经 `city::policy` 进判定面，其字节另经 building 段整份入窗——判定与阅读读的是同一串字节。

resident 段是模型每回合都读到的四段之一。`URBANITE.md` 建议 30 行以内：长的描述不会让 Resident 更能干，只会让每个回合更贵——这句话写在模板里，因为模板在场即教学。

邻里名册的常驻代价是一件工具的 disclosure 与 schema，名册本身**不进 prefix**：一栋楼的住户数会长，而每回合都付的字节不该随人口增长。模型读到的常驻新增只有 `status` 的一行 `neighbours: N`——它回答的是「值不值得问」，问出来的详情由工具在需要时交付，与 catalog 对 skill 用的是同一条渐进披露。
-/

/-! ## 11 边界枚举

无文件（→Ephemeral）｜文件存在但无读权限（→报错）｜空文件（→Resident，段为空字节；空描述是作者的选择，不是缺陷）｜地址含多段（`lab/room1`，逐段 push）。
-/

/-! ## 12 错误处理

`E_STORAGE_FATAL`（读不动一个存在的描述）：不可定义掉——文件系统权限是外部世界的事实，而静默降级是被明拒的替代。

`E_INVALID_ARGS`（`[context] second_threshold` 域外）：不可定义掉——值是人写的输入，类型把「构造后非法」定义掉了，「构造时非法」必须留码；钳位是被明拒的替代。

每个模块的拒绝、它的码与恢复语写在规定它的那一节里；失败之后什么保持不变是接口的一部分：`change` 被拒时文件一字不动（§8-4b，`spec/ConfigLayers.lean`），`edit_against` 与 `revise` 的判定拒了文档不动（§8-27、§8-40，`spec/Document.lean`），`create` 被拒的楼不留下任何东西（§8-3），`plan_install` 与 `apply` 之间来源被换时盘上一个字节不动、`register` 不被调用（§8-28）。

决定的条目与它们住的地方：

| 决定 | 标题 | 住处 |
|---|---|---|
| D1 | 定规：一个 run 不改写审判它自己的规则 | `crates/city/spec/RulesTool.lean` |
| D2 | 定规：读界在调用时按目标所在楼现读规则 | `crates/city/spec/Policy.lean` |
| D3 | 定规：一个 run 内按 (mtime, len) 留住他楼的规则 | `crates/city/spec/Policy.lean` |
| D4 | 没有生产调用者的公开面不留 | `crates/city/Spec.lean` |
| D5 | 定规：项目文件夹里的工作文档与对话记录恒不进 git | `crates/city/spec/Gitignore.lean` |
| D6 | 定规：`[resident] harness` 上梯子，会话记录压过它 | `crates/city/spec/ConfigLayers.lean` |
| D8 | 定规：拒词的恢复语归写拒词的那一处 | `crates/city/Spec.lean` |
| D9 | 城外书架上的持有带着扫描读到的正文 | `crates/city/spec/Library.lean` |
| D10 | 写入限制随派活走，不进 `RULES.toml` | `crates/city/spec/Policy.lean` |
| D11 | 身份住在两份治理文档的身份区里，一个 session 在房间那一层冻一版 | `crates/city/spec/Identity.lean` |
| D13 | 治理工具的 scope 用账本上的 scope 文字 | `crates/city/spec/RulesTool.lean` |
| D16 | `[remote]` 只在城那一层，值照写下的读，判在开门时 | `crates/city/spec/ConfigLayers/Remote.lean` |
| D17 | 读-判-换的门把此刻的字节交给判定，判定留在调用方 | `crates/city/spec/Document.lean` |
| D18 | 模板与 `City.md` 住进 city 的包目录，不留在 `docs/` | `crates/city/spec/SpineFiles.lean` |
| D19 | 审核状态从账本与书架摘要读出；审核在落位之后发起，不拦上架 | `crates/city/spec/Library/Audit.lean` |
| D20 | User 加 skill 只经 `InstallSkill` 一扇门，自带的 skill 编进二进制 | `crates/city/spec/Library/Install.lean` |
| D21 | JOB.md 用 `<from>` 写明交活的人，任务与目标转义后放进各自的节 | `crates/city/spec/SpineFiles.lean` |
| D22 | `City.md` 写明哪种形状是 User 的话，信与居民交下的活只带那个居民的身份 | `crates/city/spec/SpineFiles.lean` |
| D23 | `rules` 工具只读：说明不提供它恒拒的操作，也不留写面 | `crates/city/spec/RulesTool.lean` |
| D24 | `[search]` 是三臂的值，`Custom` 不回落到缺省那一家，缺省那一家只在 `config_layers::search` 声明 | `crates/city/spec/ConfigLayers.lean` |
| D25 | MCP url 用 `url::Url` 解析，凭据的名字与值仍由 `kernel::secret` 判 | `crates/city/spec/ConfigLayers.lean` |
-/

/-! D8 定规：拒词的恢复语归写拒词的那一处

**(a) 梯子只加文件，不改恢复语。**

**决定**：`ladder::stated` 包一个解析拒词时只在 subject 前加上文件路径；码、action、nearby 与恢复语照 `ConfigLayer::parse` 交出的原样保留。

**理由**：派活、设置页与 `sprawling check` 都经梯子读配置，而恢复语是写拒词的那一处按场合写的：一层同时写了 `[model] name` 与 `[resident] harness` 时，`two_residents` 说「`/new` 忘掉会话写下的模型，或删掉 harness 那一行」，这正是人要做的那一步。梯子把它换成通用的「改掉消息点名的值，或删掉那个键」，派活路径上的人就读不到 `/new` 这条出口，而 `sprawling check` 与写路径读到的却是完整的拒词：同一份文件的同一个错，两条路给出两句话。

**被否**：梯子自己按错误种类挑恢复语——那是恢复语的第二个家，拒词加一种它就要跟一种。

**重开参数**：梯子开始读 `ConfigLayer::parse` 以外的来源（例如人层 `~/.sprawling/config.toml`），而那个来源的拒词不带恢复语时。

**(b) harness 的墙钟上限是楼规的一键，缺省 60 分钟。**

**决定**：`RULES.toml` 的 `harness_minutes` 给一栋楼里每次 harness run 的墙钟上限，缺省 `HARNESS_MINUTES_DEFAULT = 60`，`0` 在解析时拒。

**理由**：上限回答的是「这栋楼肯让一个外来居民占一条 lane 和它的进程吃掉的内存多久」（pool 不设车道数，`crates/sprawling/Spec.lean` D34；新 run 的放行只看可用内存，§8-46-3 的内存闸），与 `review`、`confidential` 同是楼对它的居民立的规矩，而楼规由人写、run 改不了（D1）。放在 `CONFIG.toml` 的 `[resident]` 表里，它会爬城／楼／房间的梯子，一间房自己的那一层就能把上限写大，而那一层是会话写记录的地方。缺省给一个值而不是「不限」：不限时一个卡住的 harness 一直吃着别的 run 放行要看的那份可用内存，直到人发现，60 分钟够一次大的改动做完。

**被否**：①`[resident] minutes` 与 `harness` 并列：见上，房间一层能改楼的上限；②缺省不限、只靠停摆：卡住的 harness 要人来发现。

**重开参数**：放行不再看可用内存（`crates/sprawling/Spec.lean` §8-46-3 的内存闸），于是一个卡住的 harness 挡不住别的 run，或 harness 能在回合中间报告进度、城能分辨「在做事」与「卡住了」时。
-/

/-! ## 13 依赖选型

workspace 内只依赖 `kernel`（拓扑硬约束）。dev 依赖 `tempfile`。外部依赖如下，均在 workspace 钉版（不新增版本权威）。

`toml` 与 `serde`（derive）。理由：三层配置的格式是 TOML，而 `toml` 已被 `xtask` 消费（budgets.toml／lexicon.toml）；解析走 serde derive 加 `deny_unknown_fields`，使「写错的键」在反序列化那一刻失败。手写一个 TOML 子集解析器是可行的另一条路，已落选：它会把一个已有权威的格式变成本库自己的私有变体。

`url`（workspace 钉版）：MCP 与搜索供应方的 url 判定要按 WHATWG URL 标准读出 scheme、userinfo、host 与 percent 解码后的 query 参数（`spec/ConfigLayers.lean` §8-4b，D25）。它已在锁文件里（`reqwest` 之下），许可相同。落选的另一条路是手写切分，理由见 D25。

`cap-std` 与 `cap-fs-ext`（workspace 钉版）：技能包预检要按打开的目录句柄相对地列举与打开（§8-28），标准库只按路径打开。它们是安全 Rust 里的那一层 `openat`；其下的 `cap-primitives` 已在锁文件里（`wasmtime-wasi` 之下），许可相同。落选的另一条路是只收城自己控制的暂存区里的来源：它把「来源是谁的目录」变成一条安装方要遵守的约定，而不是由读法成立。

规格本身只 import 工具链的库、本 crate 的分部与 kernel 的分部（`crates/kernel/spec/Address.lean`、`crates/kernel/spec/Layout.lean`），ARCHITECTURE.md §3 的 `depmap` 允许 city 依赖 kernel。
-/

/-! ## 14 硬编码声明

城里每一份文件叫什么、落在哪，权威是 `kernel::layout`：`ARCHIVE_DIR`、`BUILDING_SHELF`、`CONFIG_FILE`、`LIBRARY_DIR`、`URBANITE_FILE` 由本 crate `pub use` 转出而不复述，路径由 `CityLayout` 的十一个方法给出而不逐段 push。`spine_files` 的 `HANDOFF_FILE` 同样只是 `kernel::layout` 那一份的别名；房间的 `JOB.md` 只经 `job_path` 取，文件名留在 `kernel::layout::JOB_FILE` 一处。本 crate 自己定义的文件名只剩没进布局表的那几份：`RULES_FILE`、`DESKTOP_SCOPE_FILE`、`PREFERENCES_FILE`、`SCHEDULE_FILE`、`WATCH_FILE`、`GITIGNORE_FILE` 与 spine 剩下的那一组。

Ephemeral 段文本（私有常量，改它即改一个 Ephemeral 读到的第一句话）。

`PACKAGE_BYTES_LIMIT`（32 MiB，`library::install::precheck::walk`）：一件技能包文件字节的上限，理由见 §8-28。它是产品的界，不是按某一类机器调出来的数：包的字节要在内存里握两份，而技能的本分是文本和小脚本。

`CONFIG_FILE`（三层同名，理由见 §10）；新楼的 `RULES.toml` 字节不写在代码里，而是 `include_str!("../../templates/RULES.toml")`——它的权威是那份模板，路径写错在编译期就会被堵住；`Confidential` 模板对该字串做一处行替换（`confidential = false` → `true`），替换是否真的生效由 `policy::evaluate` 读回来断言。
-/

/-! ## 15 影响面

改身份、规则、配置梯子或书架的公开面，波及 `crates/sprawling` 的装配层（派活、建楼、开房间、写会话）与视图；改 `crates/city/templates/` 下被 `include_str!` 的模板即改新楼与新城的第一批字节；加一件工具即 `ChatRequest.tools` 每回合多一条 disclosure 与一份 schema。
-/

/-! ## 16 测试与约束

三条：身份两态各一条；**段字节跨两次加载稳定**；Dossier 只计本人的 Run 且跨 Run 累加。bin 侧另有一条端到端断言（两次 Dispatch 的 resident 段哈希相同、run 段不同）。

建楼与配置九条：新建的楼被 `policy::load` 读回且 confidential 模板真的锁本地模型池｜二次出生恒拒｜reserved prefix 下建楼恒拒｜房间地址建楼恒拒且拒词指出该建哪栋｜下层配置盖上层｜**同一层写模型又写 harness 即拒，城自己的写路径也写不出这样的文件；房间有会话记录时梯子上的 harness 不生效，`/new` 之后生效**｜不认的键即拒｜**写在 `CONFIG.toml` 里的 effort 出现在真实出线请求体里**（bin 侧端到端，假 provider 录下请求体）｜**`usersbrowser` 的三种值各读回各的形状，`browser` 与它互不影响，confidential 楼写它即拒**。

技能安装六条（`library::install::tests`，CAS 绑定一例在 `crates/sprawling/tests/skill_install.rs`）：装上架的字节与来源一致且扫描读回的 `Holding` 整体相等｜同哈希重装幂等且盘上字节不变｜**plan 与 apply 之间来源目录被换即整体拒收**（盘上无文件、登记零调用）｜经符号链接的包（包目录本身或 `SKILL.md`）恒拒｜同格异哈希占名拒｜跨 section 同名拒。

自带 skill 一条（`accounting::worker::genesis::tests`）：新城成形之后城库的 `shipped` 格恰是包目录 `skills/` 下的每件 skill，二次成形被拒后书架不变（§8-28c）。

审核状态一张轨迹向量表（`library::audit::tests`）：上架、审、改、迟到的旧摘要审核、失败的取，逐行比对 `audit_state` 与 Audit.lean 的 `shown` 在同一条轨迹上的答案（§8-28b，Roadmap §3.3 LV1）。

读界两条：`kernel::address::tests` 的三类读者矩阵（本楼读本楼、他楼读非机密楼、楼外读机密楼，外加机密楼读自己、规则读不出）逐格判出 `ReadVerdict`，且本楼的读从不调用规则闭包｜bin 侧 `a_run_in_another_building_reads_nothing_of_a_confidential_one`：普通楼里的 run 按路径 `read`、再不带路径 `search` 机密楼里的文件，假 provider 录下的每一份请求体里都没有那份文件的字节，且最后一份带着 `E_GATE_DENIED`。

邻里名册六条：扫到的名册**不含我自己**且有人的与空的各自落在对的臂上｜`## Bring them` 在场时取它、缺席时退回第一段正文且跳过标题与引文｜同一座城扫两次字节相同（`read_dir` 序不得泄漏到答案里）｜`scope=city` 只交出楼名、不交出任何住户｜`.sprawling` 与 archive 目录都不是房间｜**模板仍然带着 `## Bring them` 这一节**（对 `crates/city/templates/URBANITE.md` 的 `include_str!` 断言；模板改名而代码不改，就是一份永远退回正文的名册）。

形式化的义务由证明清偿：`lake build crates.city.Spec`（`just models` 在 `just check` 里构建全部规格），不留 `sorry`、`admit` 与 `axiom`，`cargo xtask gates spec` 检查这一点。模型与生产实现的对应由这些 Rust 测试检查，它们是行为比对，不是精化证明：

- 梯子：`config_layers::tests`（下层盖上层、不认的键即拒、写回往返）、`config_layers::ladder::tests`（解析拒词在梯子上带着文件、恢复语照旧）、`config_layers::resident::tests`（同一层两种居民即拒，会话记录压过梯子，`/new` 之后最近的 harness 生效）、`config_layers::remote::tests` 与 `config_layers::shelves` 的测试（城独占的表在楼层即拒）。
- 写配置：`config_layers::write::tests`（写面写出的字节读得回，读者不收的写不落）、`config_layers::session::tests`。
- 楼：`building::tests`（新建、收编、四种拒）与 `building::removal` 的测试。
- 楼规：`policy::tests`（confidential 四条、缺键即拒、写域）、`policy::cache::tests`（戳不变不读盘、长度变了重读）。
- 文档：`document::tests` 的 `two_writers_of_one_document_do_not_overwrite_each_other`、`a_revision_reads_the_bytes_on_disk_and_holds_the_lock_while_it_decides`。
- 书架：`library::tests`（近的一格替换城库那一件）。
- 职分：`vocation::tests` 的 `city_hall_plans_and_every_other_building_builds`。

没有 Lean 模型的分部，其要求由类型与 `cargo nextest run -p sprawling-city` 的各模块测试守住：身份与 Dossier、身份区、spine 文档与交接表单、日程、watch、技能安装、向导、名册、归档、会话、房间、gitignore、`check`、`history`。
-/

/-! ## 17 文档关系

- ARCHITECTURE.md §11「Specifications in Lean」：本规格的布局；它改了，分部的路径与 `architecture.toml` 里 city 各行的 `spec` 锚点一起重看。ARCHITECTURE.md 模块表的 city 各行讲的是同一组模块。
- `architecture.toml` 的模块图：city 每一行的 `spec` 指向规定它的分部，`cargo xtask gates specalign` 检查锚点在盘上。
- `docs/glossary.md`：本规格用的词（Resident、Neighbourhood 等），`cargo xtask gates lexicon` 检查。
- `crates/city/templates/` 下被实例化的模板：它们与 §8-3、§8-5、§8-41 同期改，因为模板的字节就是新楼与新城的第一批字节。
- kernel 的规格（`crates/kernel/Spec.lean`）：地址与保留子树（§8-2，`crates/kernel/spec/Address.lean`）、布局（`crates/kernel/spec/Layout.lean`）、配置梯子的胜负与冻结（§8-22）、效果层对 `Govern` 的拒（§8-27）。它们改了，这里的 `Building`、`Policy`、`ConfigLayers/Ladder` 三个模型与 §8-2、§8-4 一起重看。
- runtime 的规格（`crates/runtime/Spec.lean` §8-29、§8-30-1）：阅览室怎样把书架上的一件交给 run、模型选的路径怎样过读界；accounting 的规格（`crates/accounting/Spec.lean`）与 sprawling 的规格（`crates/sprawling/Spec.lean`）：装配层怎样调本 crate 的写面。
- 引本规格的其他规格与 rustdoc 写 `crates/city/Spec.lean §8-n` 或 `city D<n>`；本 crate 的 rustdoc 写规定它的分部。一节换了分部，它的标签不变，引用不必改。
-/
