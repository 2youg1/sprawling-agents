-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::catalog

规定 `catalog`（`crates/runtime/src/` 下同名的文件）。渐进披露：一个 run 被告知哪些工具与 skill。本文件是 `crates/runtime/Spec.lean` 的一个分部；下面每一节保留它在 runtime 规格里的标签 §8-n，别处引作 `crates/runtime/Spec.lean §8-n`。
-/

/-!
### 8-11 runtime::catalog（形状 6＋渲染）


```rust
pub struct CatalogEntry { pub name: String, pub disclosure: String, pub expansion: String,
                          pub hash: Option<B3Hash>,    // 架上那份文档被读到时的哈希
                          pub package: Option<String> } // 包目录，由 city 的扫描给出；单文档为 None
pub struct SkillPin { pub name: String, pub hash: B3Hash }
pub struct Catalog { /* tools: BTreeMap<ToolName,…>、skills: BTreeMap、mode: Option<Mode> —— 私有 */ }
impl Catalog {
    pub fn new() -> Catalog;
    pub fn admit_tool(&mut self, meta: &ToolMeta) -> Result<(), AxError>;      // disclosure 非空；重名＝E_INVALID_ARGS
    pub fn admit_skill(&mut self, entry: CatalogEntry) -> Result<(), AxError>; // 只收阅览室准入者（装配层按楼的 city::policy 规则求值后直供）；expansion 是城内地址
    pub fn admit_carried_skill(&mut self, entry: CatalogEntry) -> Result<(), AxError>; // 城外书架上的一件：expansion 是扫描读到的那份文档正文（§8-29-6）
    pub fn set_mode(&mut self, mode: Mode);                                    // 只列本 Run 所处者
    pub fn render(&self) -> String;              // Resident 段的 catalog 部分：段头一行自述＋一行一件；BTreeMap 序恒定
    pub fn tool_defs(&self) -> Vec<ToolDef>;     // ChatRequest.tools 的唯一来源
    pub fn expand(&self, name: &str) -> Option<Expansion>;   // 第二级披露（怎么用），§8-6
    pub fn skill_pins(&self) -> Vec<SkillPin>;   // 本 Run 拿到了哪几份，当时各是什么字节
}
```

- **`hash` 是 `Option`，而那个 `None` 不是「没算」**：目录里另有两类条目的正文由本构建自己握着（mode 的纪律、dev 那一条），它们背后没有一份能在无人看着时改掉的文档。
- **pin 从 catalog 取，不重扫一遍书架**：catalog 已经是「本 Run 能够到什么」的权威，再扫一次就是在另一个时刻对同一个问题给第二个答案。
- **一件 skill 怎么交给 run，由它进 catalog 的那扇门定**：`admit_skill` 收城内书架上的一件，`expand` 答 `Expansion::Skill`，`read` 到那个地址去开；`admit_carried_skill` 收城外书架上的一件，`expand` 答 `Expansion::Said`，正文就是 catalog 手里那份。两扇门而不是一个布尔参数：`expansion` 这一格在两扇门后是两种东西（地址与正文），门名把这件事说在调用处。两扇门共用同一套卫生检查（名字与一行披露非空、不重名），重名跨两扇门同样拒。

**`render()` 与 `set_mode()` 的生产调用者是装配层的 prefix 组装**。工具走 `ChatRequest.tools` 到达模型；没有 `render()`，**阅览室准入的 SKILL 与本 Run 所处的 mode 就到不了任何模型**，`city::library` 的准入判定就是一道没有下游的门。

接法：`Catalog::render()` 追在 `identity.segment_bytes()` 之后，合成 Resident 段。**不另开第五个槽**：一个居民能够伸手取到什么，与它是谁同属一类常住事实，且两者都随 Run 冻结，故前缀在整个 Run 的寿命里仍可缓存。装配层因此把 prefix 的组装移到目录建好之后。

**第二级披露经 `read`**：SKILL 的 `expansion` 是 `city::holding_address()` 给的一个地址，坐在**保留前缀 `.sprawling/` 下**。`render()` 不印那个地址；模型按名字调 `read`，`read` 先查 catalog（§8-29），所以它不必知道、也读不到那个保留前缀下的路径。
-/
