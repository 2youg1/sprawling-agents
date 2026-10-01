-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# kernel::release

规定 `kernel::release`（`crates/kernel/src/release.rs`）：一次发布的两种拼法与谁更新。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。
-/

/-!
### 8-54 `kernel::release`：一次发布的两种拼法，以及两次发布谁更新

```rust
pub struct Release { /* major, minor, patch, year, month, day —— 私有 */ }
impl Release {
    pub fn from_tag(tag: &str, expected_version: &str) -> Result<Release, AxError>;
    pub fn from_npm_version(text: &str) -> Result<Release, AxError>;
    pub fn version(&self) -> String;      // 0.0.5
    pub fn tag(&self) -> String;          // v0.0.5-Pre-alpha-260912
    pub fn npm_version(&self) -> String;  // 0.0.5-pre.260912
    pub fn released(&self) -> String;     // 2026-09-12
}
pub enum ReleaseVerdict { Current, Behind, Ahead }
pub fn stands(mine: &Release, newest: &Release) -> ReleaseVerdict;
```

**四条口径：**

1. **一次发布有两种拼法，而只有这一处同时认识两种。** git tag 写 `v0.0.5-Pre-alpha-260912`；npm 只收 semver，同一次发布因此发成 `0.0.5-pre.260912`。`xtask channel` 按前者转出后者去发布，运行中的二进制按后者读回注册表——两边转换各写一份，就是「这是哪一次发布」有了两个答案。
2. **排序是 semver 自己的。** 日期落在 pre-release 段，于是 `0.0.5-pre.260912` 高于 `0.0.5-pre.260911` 而低于裸的 `0.0.5`——semver 对点分数字标识符按数值比。`Ord` 按字段声明顺序派生即复现该规则，本 crate 与注册表因而对同一对发布给出同一个次序。**这正是本类型存在的理由**：二进制拿自己的裸 `0.0.5` 去比注册表的 `0.0.5-pre.260912`，会把最新的那一版读成更旧的那一版，且无声。
3. **日期必须随版本一起走，不能摆在旁边。** 一个 pre-alpha 的版本号几乎说不出树有多旧，而树有多旧正是它的读者最需要知道的（CHANGELOG.md 开篇）。故 `released()` 是给人读的那一个渲染，`npm_version()` 是给注册表的那一个。
4. **无钟无套接字。** 注册表此刻给的是什么，归调用方去取；本模块只判它被递到的东西（ARCHITECTURE.md 第 1 段）。
-/
