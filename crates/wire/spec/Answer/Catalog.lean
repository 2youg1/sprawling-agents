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
    pub run: Option<RunId>,                  // 这栋楼冻结了提示的 run 里最后开始的那个（按 run_started 的先后）；第一个 run 之前为 None，tools 为空
    pub core_bytes: u64,                     // 那个 run 的常驻工具数组的字节
    pub index_bytes: u64,                    // 那个 run 的休眠索引的字节
    pub runs_seen: u32,                      // 使用证据数过的 run 数：至多窗口，年轻的楼更少
    pub tools: Vec<CatalogTool>,             // 那个 run 的装配层建出的每件工具，按名字
    pub city: ExposureTable,                 // 城那一级的 [catalog]，照写下的
    pub stated: ExposureTable,               // 这栋楼那一级的 [catalog]，照写下的
}
pub struct CatalogTool {
    pub name: String, pub namespace: String,
    pub hint: String,                        // 描述的第一句，按休眠索引的截法
    pub bytes: u64,                          // 它的定义的字节：常驻时每次请求多付这些
    pub exposure: Exposure, pub from: ExposureFrom,   // 下一个 run 的档位与定它的那一级
    pub fallback: Exposure,                  // 两张表都不说时的档位；from 为 Fixed 时就是固定的那一档
    pub runs_used: u32,                      // runs_seen 个 run 里调过它的个数
    pub last_used: Option<TimeMs>,           // 那些 run 里最后一次调它的时刻
}

// Command 的两个面（整张替换；空表拿掉这一级的键）
ConfigureBuilding(BuildingSettings)          // 与 ConfigureCity(CitySettings) 同形；线上 JSON 与原来的结构体变体相同
pub struct BuildingSettings { .., pub catalog: Option<ExposureTable>, pub idem: IdemKey }
pub struct CitySettings { .., pub catalog: Option<ExposureTable>, pub idem: IdemKey }

// §8-35 的答复多一格
pub struct PrefixAnswer { pub run: RunId, pub segments: Vec<PrefixSegment>,
                          pub tools: Option<kernel::event::record::PromptTools> }
```

- **行取这栋楼最新 run 冻下的工具表，档位按盘上的表现算**：答一问时不装配工作台，因为装配要与每台 MCP 服务器握手、可能起浏览器（accounting D55）。行的名字、命名空间、第一句与字节来自那个 run 的 `prompt_assembled.tools`（runtime D111），`Off` 的工具也有一行，所以关掉的能在这里开回来；`exposure` 与 `from` 用此刻盘上的两张 `[catalog]` 表经 `runtime::catalog::exposure::exposure_of` 算出，与下一个 run 的目录调的是同一个函数，页面说的就是下一个 run 得到的。
- **两张表照写下的一起答**：页面编辑其中一张（本楼或全城），要知道另一张说了什么才算得出一件工具在草稿下的档位；`fallback` 给出两张都不说时的那一档，所以页面不另写一份缺省表。
- **使用证据**：`views::usage` 同一遍整账本折叠里，每条 `tool_called` 按它记的工具名归到它的 run（经 `tool` 的 `run` 调用的记的是那件工具本身，runtime §8-61）；这栋楼（含它的房间）最新的至多 50 个 run 里，每件工具被调过的 run 数与最后一次的时刻。
- **写门整张替换**：`ConfigureBuilding.catalog` 写这栋楼那一级的整张 `[catalog]`，`CitySettings.catalog` 写城那一级的；空表拿掉这一级的键，这一级回到继承（city §8-4g）。表在城那里先判，被拒什么都不落、页面留着草稿；`ConfigureBuilding` 的 `catalog` 先于别的面写。楼那一级写下后记 `building_configured`，`catalog: true`。
- **`BuildingSettings` 是一个具名的值**：一栋楼的五个面总是一起从页面走到 `accounting` 的 `configure_building`，一起走的值是一个值；它与 `CitySettings` 同形，外部标记的 JSON 与原来的结构体变体逐字节相同，所以页面与旧帧不受影响，只有 Rust 的匹配写法变了。
- **`PrefixAnswer.tools`**：run 页的 prompt 透镜在 City 段之前画「工具」一块，用的是这一格；`None` 是工具表入账之前冻结的 run。
- **字节是本城序列化的字节**：各家 provider 的兼容格式拼法不同，账要一把对所有兼容格式都一样的尺；页面只写字节，不换算 token（客户端没有分词器，系数因模型而异）。
- 验收：`crates/sprawling/tests/connectors.rs` 的 `a_namespace_a_building_turns_off_leaves_the_next_run_and_stays_listed`（两百件工具的模拟服务器在这一问里有两百行；楼把它的命名空间设为 `off` 之后，下一个 run 的工具表与休眠索引里都没有它，这一问仍列出两百行，档位 `Off`、来自楼）；city 的 `the_city_and_a_building_state_how_tools_reach_a_run_and_a_room_may_not`；runtime 的 `a_catalog_table_moves_tools_between_core_dormant_and_off`。
-/
