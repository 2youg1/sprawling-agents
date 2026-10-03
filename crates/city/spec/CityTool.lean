-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# city::city_tool

规定 `city_tool`（`crates/city/src/` 下同名的文件）。市政厅对城市本身的那一扇门：`raise`、`adopt`、`list`。本文件是 `crates/city/Spec.lean` 的一个分部；下面每一节保留它在 city 规格里的标签 §8-n，别处引作 `crates/city/Spec.lean §8-n`，决定引作 `city D<n>`。
-/

/-!
### 8-23 city::city_tool：市政厅对城市本身的那一扇门

**接口**：

```rust
// city::city_tool（形状 4 适配器）
pub struct CityTool { /* city_root —— 私有 */ }
impl CityTool { pub fn new(city_root: &Path) -> Result<CityTool, AxError>; }
// 工具名 `city`；action ∈ { raise, adopt, list }
// raise:  { name, template }  —— template 缺省 `minimal`
// adopt:  { name }
// list:   —— 无参，答本城的楼与每栋楼是否已有 RULES.toml
```

- **`Effect::Govern`，不是 `Effect::Write`**：立一栋楼是在城根下造目录，任何写域都够不到那里，理由与 `rules_tool` 同——这个决定是人的。判定与 §8-2b 同一条（此处第二次适用而不是第二个权威）：`raise`／`adopt` 的调用在效果层被拒，run 里到不了盘；`list` 不改城的形状，`effect_of` 对它答 `Effect::Read`，run 里放行（`crates/kernel/spec/Gate.lean` D26）。
- **三个动作一条目录行**：`list` 是 `raise` 与 `adopt` 的前提（叫什么名字、哪个目录已经在那儿），拆成第二个工具只会多一行给模型读。
- **动作不认即拒并报出已知集**：猜错这里意味着把「收编一个已有目录」执行成「新建一栋空楼」，而后者会在人的工作目录旁边多出一份不属于它的模板。
- **本工具只装给市政厅的居民**（见 8-22），但装上不改变判定——每一次调用都在效果层被拒（§8-2b）。建楼与收编恒走人的手：线上命令 `CreateBuilding`，或 CLI 的 `sprawling adopt <city> <addr>`（§8-3），无论哪个地址在问。
-/
