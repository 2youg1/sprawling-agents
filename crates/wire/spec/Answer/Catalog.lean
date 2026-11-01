-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::answer::catalog

规定 `answer::catalog`（`crates/wire/src/` 下同名的文件），以及 `ConfigureBuilding`、`CitySettings` 的 `catalog` 面与 `PrefixAnswer.tools`。一栋楼的每件工具怎样到达它的 run、谁定的、用得怎样，以及人改它的那两扇门。本文件是 `crates/wire/Spec.lean` 的一个分部；下面一节保留它在 wire 规格里的标签 §8-99，别处引作 `crates/wire/Spec.lean §8-99`，决定引作 `wire D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的帧形状由 Rust 的类型与 `crates/wire/tests/wire_contract.rs` 钉住的 wire schema（`wire::schema_hash`）守住。页面草稿的性质在 `client/spec/Views/Catalog.lean`。
-/

/-!
### 8-99 一栋楼的能力：`Query::Catalog` 与两扇 `[catalog]` 写门

```rust
// Query
Catalog { building: Address },               // → Answer::Catalog(Box<CatalogAnswer>)
pub struct CatalogAnswer {
    pub building: Address,
    pub run: Option<RunId>,                  // 这栋楼冻结了提示的 run 里最后开始的那个；第一个 run 之前为 None
    pub core_bytes: u64,                     // 常驻工具数组字节
    pub index_bytes: u64,                    // 休眠索引字节
    pub reading_bytes: u64,                  // Always skill 正文字节
    pub always: u32, pub dormant: u32, pub skills: u32, // 与 status 相同的 catalog 计数
    pub changed: u32,                        // 当前 Changed 行数，侧栏读同一个数
    pub open_sessions: u32,                  // 冻结值不同于楼当前选择的会话数
    pub runs_seen: u32,                      // 最近 30 天使用证据覆盖的 run 数
    pub tools: Vec<CatalogTool>,             // 内建与 connector 工具，按名字；connector 由 namespace 分组
    pub skills: Vec<CatalogSkill>,           // 同一目录的 skill 行
    pub references: Vec<McpReference>,       // 楼的引用与 pin，不携定义
    pub skill_pins: BTreeMap<String, B3Hash>,
    pub reading_room: Vec<String>,
    pub city: ExposureTable,                 // 城那一级的 [catalog]，照写下的
    pub stated: ExposureTable,               // 这栋楼那一级的 [catalog]，照写下的
}
pub struct CatalogTool {
    pub name: String, pub namespace: String,
    pub current: Option<CatalogVersion>,     // Removed 时缺席；其余是当前候选定义
    pub pinned: Option<CatalogVersion>,      // Changed／Removed 时是被钉住的旧定义
    pub change: Option<CatalogChange>,       // Added｜Changed｜Removed；未变为 None
    pub exposure: Exposure, pub from: ExposureFrom,
    pub fallback: Exposure,
    pub runs_used: u32,                      // 最近 30 天调过它的 run 数
    pub last_used: Option<TimeMs>,
}
pub struct CatalogVersion {
    pub digest: B3Hash, pub hint: String, pub bytes: u64,
    pub description: String, pub input_schema: Payload,
}
pub enum CatalogChange { Added, Changed, Removed }
pub struct CatalogSkill {
    pub name: String, pub section: String,
    pub disclosure: String,
    pub current_document: Option<String>,    // 候选 SKILL.md；消失时为 None
    pub pinned_document: Option<String>,     // Changed 时供并排比较
    pub candidate: Option<B3Hash>, pub pinned: Option<B3Hash>,
    pub change: Option<CatalogChange>,
    pub exposure: Exposure, pub from: ExposureFrom,
    pub audit: Option<AuditVerdict>, pub audit_stale: bool,
    pub runs_used: u32, pub last_used: Option<TimeMs>,
}

// Command 的两个面（整张替换；空表拿掉这一级的键）
ConfigureBuilding(BuildingSettings)          // 与 ConfigureCity(CitySettings) 同形；线上 JSON 与原来的结构体变体相同
pub struct BuildingSettings { .., pub catalog: Option<ExposureTable>,
    pub mcp_references: Option<Vec<McpReference>>,
    pub skill_pins: Option<BTreeMap<String, B3Hash>>, pub idem: IdemKey }
pub struct CitySettings { .., pub catalog: Option<ExposureTable>, pub idem: IdemKey }

// §8-35 的答复多一格
pub struct PrefixAnswer { pub run: RunId, pub segments: Vec<PrefixSegment>,
                          pub tools: Option<kernel::event::record::PromptTools> }
```

- **工具行取这栋楼最新 run 冻下的工具表，漂移行取 `digest_invalidated`，版本正文取 CAS**：答一问时不装配工作台，因为装配要与每台 MCP 服务器握手、可能起浏览器（accounting D55）。行的名字、命名空间、定义 digest 与字节来自那个 run 的 `prompt_assembled.tools`（runtime D111、D122），`Off` 的工具也有一行，所以关掉的能在这里开回来；Added／Changed／Removed 由当前 pin 与最新观察比较得出，Pinned 与 Current 的 description/schema 分别按 digest 完整读取 CAS。`exposure` 与 `from` 用此刻盘上的两张 `[catalog]` 表经 `runtime::catalog::exposure::exposure_of` 算出，与下一个新会话的目录调的是同一个函数。
- **skill 行从当前书架索引与 `[skills.pins]` 合并**：当前候选与 pin 相同即未变，不同即 Changed；pin 的 `SKILL.md` 与候选正文都来自完整 CAS 读取，外部候选只在 Accept 时复制进城。审核判定仍按 digest 读 `skill_audited`，30 天使用证据与工具一样从 usage fold 得出。
- **一次问答足以保存整楼选择**：`references`、`skill_pins`、`reading_room` 与两张 `[catalog]` 表给出草稿初值；保存经 `ConfigureBuilding` 更新引用、pin 与档位，并经已有 RULES 写门更新名字，绝不从另一栋楼复制服务器定义。`open_sessions` 由每个会话的 `CATALOG.toml` 与楼当前选择逐份比较，不能用最后活动时间代替（city D114）。
- **两张表照写下的一起答**：页面编辑其中一张（本楼或全城），要知道另一张说了什么才算得出一件工具在草稿下的档位；`fallback` 给出两张都不说时的那一档，所以页面不另写一份缺省表。
- **使用证据**：`views::usage` 同一遍整账本折叠里，每条 `tool_called` 按它记的工具名归到它的 run（经 `tool` 的 `run` 调用的记的是那件工具本身，runtime §8-61）；以账本最新时刻为基准的最近 30 天里，这栋楼（含它的房间）每件工具或 skill 被调过的 run 数与最后一次的时刻。城只给筛选证据，不自动关闭未使用项。
- **写门整张替换**：`ConfigureBuilding.catalog` 写这栋楼那一级的整张 `[catalog]`，`mcp_references` 与 `skill_pins` 分别写对应整张表；`CitySettings.catalog` 写城那一级的。空 catalog 表拿掉这一级的键，这一级回到继承；空引用或空 pin 表是明确选择为空，不继承。所有可拒面先判再写，被拒绝时页面留着草稿。楼那一级写下后记 `building_configured`，已有会话的 `CATALOG.toml` 不动，新会话才取新选择。
- **`BuildingSettings` 是一个具名的值**：一栋楼的五个面总是一起从页面走到 `accounting` 的 `configure_building`，一起走的值是一个值；它与 `CitySettings` 同形，外部标记的 JSON 与原来的结构体变体逐字节相同，所以页面与旧帧不受影响，只有 Rust 的匹配写法变了。
- **`PrefixAnswer.tools`**：run 页的 prompt 透镜在 City 段之前画「工具」一块，用的是这一格；`None` 是工具表入账之前冻结的 run。
- **字节是本城序列化的字节**：各家 provider 的兼容格式拼法不同，账要一把对所有兼容格式都一样的尺；页面只写字节，不换算 token（客户端没有分词器，系数因模型而异）。
- 验收：`crates/sprawling/tests/connectors.rs` 的 `a_namespace_a_building_turns_off_leaves_the_next_run_and_stays_listed`（两百件工具的模拟服务器在这一问里有两百行；楼把它的命名空间设为 `off` 之后，下一个 run 的工具表与休眠索引里都没有它，这一问仍列出两百行，档位 `Off`、来自楼）；city 的 `the_city_and_a_building_state_how_tools_reach_a_run_and_a_room_may_not`；runtime 的 `a_catalog_table_moves_tools_between_core_dormant_and_off`。
-/

/-! D112 `Query::Catalog` 是统一目录的唯一问答，版本正文按 digest 从 CAS 读，保存只写引用与 pin

**决定**：工具与 skill 在同一 `CatalogAnswer`；Added／Changed／Removed 由 pin 与最新观察得出，版本正文由 digest 定位，不新增在线探测查询。`BuildingSettings` 增加整张 `mcp_references` 与 `skill_pins`，沿用 `ConfigureBuilding`；`[catalog]` 仍是三档的唯一写面。`open_sessions` 比较会话冻结目录与楼当前目录。

**理由**：统一面板若由客户端拼接 `Skills`、`McpHealth`、`Catalog` 与在线握手，会得到不同时刻的目录并产生页面副作用；直接携定义全文又会复制服务器主机事实。CAS digest 同时承担 pin 和版本定位，旧版与新版可以离线并排。

**被否**：新增 `ProbeCatalog`——打开页面即起服务器；继续让客户端分别拼三张页——同一事实多个权威；`BuildingSettings` 再携 `McpServer` 作为楼引用——楼配置重新包含定义。重开参数：需要提供显式“立即重连”动作时，作为单独命令及其副作用设计，不并入只读目录问答。
-/
