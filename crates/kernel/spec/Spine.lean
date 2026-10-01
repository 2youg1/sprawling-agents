-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# kernel::spine

规定 `kernel::spine`（`crates/kernel/src/spine.rs` 与 `crates/kernel/src/spine/` 下的 `row`、`grammar`、`rewrite`、`memo`）：六列 roadmap 的文法与它仅有的两个编辑入口。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。
-/

/-!
### 8-19 kernel::spine（六列树）

```rust
pub enum RoadmapStatus { NotStarted, InProgress, Done, Blocked, AwaitingApproval }   // 携 serde（snake_case）
pub const ROADMAP_STATUS_SPELLINGS: [(RoadmapStatus, &str); 5];   // 表内拼写，单套
pub const ROADMAP_COLUMNS: usize = 6;
impl RoadmapStatus { pub fn spelling(self) -> &'static str; }     // 唯一拼写产地
pub enum EvidenceCell { Empty, Invalid { raw: String }, Present(Locator) }
pub struct RoadmapRow { pub id: NodeId, pub item: String, pub weight: u32,
                        pub needs: Vec<NodeId>, pub status: RoadmapStatus,
                        pub evidence: EvidenceCell }
pub enum RoadmapShape { WellFormed { rows: Vec<RoadmapRow> }, Malformed { problems: Vec<String> } }
pub struct NewChild { pub item: String, pub weight: u32 }
pub fn check_roadmap_shape(text: &str) -> RoadmapShape;
pub fn set_roadmap_status(text: &str, id: &NodeId, status: RoadmapStatus,
                          evidence: Option<&Locator>) -> Result<String, AxError>;
pub fn insert_children(text: &str, parent: &NodeId, children: &[NewChild]) -> Result<String, AxError>;

pub enum ScopeChange { Keep, Add, Drop }
pub enum WriteMoment { BeforeReport, AfterFeedback, OnPlanChange }
```

- **本模块只管文法，结构归 `plan`。** `check_*` 答「写得像不像一张 roadmap」；这些行**彼此怎么挂、各值多少、哪个能动**是 `kernel::plan` 的事。分开是因为两种坏法的修法不同：一行写错了改那一行，依赖成环了要重想这件事怎么排。
- **六列，且索引是路径**。`| # | Item | Weight | Needs | Status | Evidence |`。`2.3.1` 挂在 `2.3` 下，于是**一张表就说清了多级计划**，不需要第二个文件描述层级；`Weight` 是同一父下诸行之间的**比例**（空格＝1，整层同乘不变），`Needs` 是必须先完成的行。四列表**不是被兼容而是被报告**：把四列当六列读会把状态词读进 weight 格，所以 `check_roadmap_shape` 报 `4 columns` 并让整栋楼落到 `Progress::Unplanned`——**一份读不懂的计划没有分母，这件事必须看得见。**
- **拼写单套且不区分大小写**：运行时文档是英文，中英两套拼写会成为「一个 Resident 允许写什么」的第二个权威；而大小写不入契约是因为 `done` 这类行表达的事实表装得下，把它判成 Malformed 等于拿一个读者不接受的理由把该行逐出分母。**线上拼写另有一套**（`snake_case` 标识符）：线帧是给程序读的，表格是给人读的，让客户端硬编码 `Awaiting approval` 正是短语表存在要防的事。
- **写者与读者同住**：`set_roadmap_status` 与 `insert_children` 是这张表仅有的两个编辑入口，两者共用同一段「哪几行是这张表」的判定（`locate_table`／`body_row`）。三条契约不变：只改首个表；输出行规范化，故**同一次改写两次得到同一字节**；`Done` 缺证据恒拒（`E_EVIDENCE_MISSING`）。
- **`insert_children` 只往后编号，不补空位**：一个计划索引是一个名字，复用它等于悄悄搬走别人的证据。子节点落在父节点**最后一个后代之后**，于是阅读顺序不变、昨天看到的编号今天仍指同一件活。
- **`WriteMoment` 有消费者**：正因为可写时刻是封闭的，读者才可以在两次写之间**持有已解析的树**而不是每问一次就把每栋楼的文件重解析一遍（`accounting::plan_view`）。
-/
