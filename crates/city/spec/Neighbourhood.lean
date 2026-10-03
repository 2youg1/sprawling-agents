-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# city::neighbourhood

规定 `neighbourhood`（`crates/city/src/` 下同名的文件）。这座城有哪些地方，我身边站着谁。本文件是 `crates/city/Spec.lean` 的一个分部；下面每一节保留它在 city 规格里的标签 §8-n，别处引作 `crates/city/Spec.lean §8-n`，决定引作 `city D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `city::neighbourhood` 旁的测试守住。
-/

/-!
### 8-15 city::neighbourhood（形状 1 判定＋形状 2 值类型）

```rust
pub enum Occupancy { Resident { bring: String }, Empty }   // 穷尽两态
pub struct Neighbour { pub addr: Address, pub name: String, pub occupancy: Occupancy, pub waiting: u32 }
pub struct Neighbourhood { /* building、rooms: Vec<Neighbour>、buildings: Vec<Address> —— 私有 */ }
impl Neighbourhood {
    pub fn scan(city_root: &Path, building: &Address, me: &Address,
                waiting: &dyn Fn(&Address) -> u32) -> Result<Neighbourhood, AxError>;
    pub fn building(&self) -> &Address;
    pub fn here(&self) -> &[Neighbour];        // 本楼，除我之外的每个地址
    pub fn buildings(&self) -> &[Address];     // 全城，只有楼名
    pub fn residents(&self) -> u32;            // here 中真有人站着的个数
}
// 房间与楼的枚举各归其既有权威，本模块只调用：
pub fn room::all(city_root: &Path, building: &Address) -> Vec<Address>;   // city::rooms
pub fn building::all(city_root: &Path) -> Vec<Address>;                   // city::buildings
```

- **这栋楼里有言语，却没有地址簿**：`signal` 的 `to` 只说「the address you are speaking to」，越界拒词只报边界不报住户，于是地址靠猜；而装配层投递时 `.entry(room).or_insert_with(new_inbox)`，**猜错的一句话会当场开出一个没人读的信箱并回 `queued: true`**。本模块存在的第一个理由是让那次猜测消失。
- **`crates/city/templates/URBANITE.md` 早就承诺了这件事**：模板原话是「other agents and the User read it to know what to expect from them and what to bring to them」。承诺写在模板里，兑现它的代码在本模块。
- **一行取自 `## Bring them`，取不到才退回第一段正文**：这一行要回答的是「我为什么找他」，而模板里正是那一节写「什么样的活属于这位住户」。退回规则跳过标题行与引文行——引文行是模板留给作者的说明，把它显示出来等于让全城住户共用一句自述。**这与 `library::first_line` 不是同一条规则**：书架条目取的是标题，住户名册取的是正文，两种文档、两条规则、两个家。
- **准入判定复用 `Identity::load`，不自读文件**：「一个地址上有没有常住的人」已经有权威，第二次实现必然在某天与第一次分叉。空的 `URBANITE.md` 仍是 Resident（`bring` 为空串），沿用 §11 已记的口径：空描述是作者的选择，不是缺陷。
- **空房间照列，不隐藏**：藏起来的话，模型会把「这里没人」读成「这个地址不存在」，而一间空房恰是可以请人搬进来、或派一件活过去的地方。
- **详略随距离衰减**：本楼给到每个地址的自述，全城只给楼名。这不是新规则，而是 `signal` 的 `reach` 与 `CrossBuildingTransfer` 已经画好的那条界——**看得清的范围与说得着的范围必须是同一个**，否则名册会教模型去够它够不到的人。
- **房间＝楼下一层的非点头目录，且不是 archive 目录**：这条规则与城级的同类规则分别只住 `city::room::all` 与 `city::building::all`，装配层只调用它们——页面看到的房间与模型看到的房间因此不可能不同。
- **只到直接子目录**：房间就是这样被造出来的（`room::open` 与 delegate 都建直接子目录）。翻案条件：楼层真的成为目录的那天，改的是 `room::all` 一处。
- **一个活口径接进来了，另一个被判定为噪音**：`waiting`（那间房积压几封信）由装配层以闭包供给——队列是它的，本 crate 看不到那么远。而「谁在跑」**不接**：多条 lane 同时驱动，「在跑」在一次 drive 之内就会变，而名册随 Run 冻结（§8-15b），冻下来的一列「在跑」一回合后就可能是错的；`Dossier::is_live` 真正能说的是「某位的上一跑没冻结过」，那是崩溃后的事实，归 `resume` 而不归名册。
-/
