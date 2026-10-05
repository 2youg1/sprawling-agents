-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# accounting::home

规定 `crates/accounting/src/home.rs`。本文件是 `crates/accounting/Spec.lean` 的一个分部；下面每一节保留它在 accounting 规格里的标签 §8-n，别处引作 `crates/accounting/Spec.lean §8-n`，决定引作 `accounting D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `accounting::home` 旁的测试守住。
-/

/-!
### 8-7 accounting::home：这个人的家目录，以及本产品放在它下面的东西（形状 4 适配器）

```rust
pub const CITY_DIR: &str;                       // "city"，exe 同级的默认城
pub const NO_HOME: &str;                        // detect 的拒绝与 doctor 的报告共用这一句
pub struct Home { /* root —— 私有 */ }
impl Home {
    pub fn detect() -> Result<Home, AxError>;   // USERPROFILE，其次 HOME；E_PATH_NOT_FOUND
    pub fn at(root: impl Into<PathBuf>) -> Home; // detect 读完环境后造的就是它；比较路径的调用方不读环境直接造
    pub fn path(&self) -> &Path;
    pub fn components(&self) -> PathBuf;        // ~/.sprawling/components
    pub fn privacy_history(&self) -> PathBuf;   // 本用户在城外的 JSONL；方法只派生路径
    pub fn person_config(&self) -> PathBuf;     // ~/.sprawling/config.toml
    pub fn default_city(&self) -> PathBuf;      // ~/sprawling/city
}
```

**五条口径：**

1. **三处派生合一。** 读 `USERPROFILE || HOME` 的地方只此一处：doctor 的组件目录、安装目录、默认城的位置与人层配置都由它派生，而不是各读一遍环境。
2. **住在本 crate，因为读者跨三处。** `person`、`views::skills` 与 run 的阅览室在本 crate，`doctor` 在 `sprawling` 的库那一半，`install` 与 `router` 在它的二进制那一半；`sprawling` 的两半都够得到本 crate，本 crate 够不到它们。
3. **`detect` 失败是类型化错误，调用方各自决定是否致命。** 探组件时家目录缺席只是「看不到」，报告里由 `Absence::NoHome` 说明；装二进制时 Windows 还有 `LOCALAPPDATA` 可落，两者皆无才由 `install::no_home` 拒绝。两处都显式 `match` 错误臂而不是 `.ok()`，于是「没有家目录」是一个被做过的决定。
4. **城不住点目录，因为城是这个人的东西。** `default_city()` 给 `~/sprawling/city`：点目录下装的是与这台电脑绑定的状态（组件、这个人的配置层），而一座城是人要打开、编辑、备份、拷到另一台机器上的工作，看不见的城是备份不了的城。`Absence::NoHome` 那句「neither USERPROFILE nor HOME is set」由 `accounting::home::NO_HOME` 一处定义，`detect` 的拒绝与 doctor 的报告读的是同一句。
5. **`~/.sprawling` 与城里的保留子树共用 `kernel::RESERVED_PREFIX`。** 这是本产品拥有的那一个点目录名，一个名字一个家；它在家目录下装的是属于这个人的东西，不属于任何一座城。`person_config()` 用小写 `config.toml`，与城内各层的 `CONFIG.toml` 不同名——两者是不同的层，同名会诱使某个读者把其中一个当成另一个。本模块只给路径，读写与分层归配置阶梯（H-10）。
-/

/-! privacy_history 派生本用户的独立 changes.jsonl，路径的唯一拼写在 Home Rust 实现。
读取不存在的 history 不创建目录；记录不属于 Ledger、城导出或发行资源。
平台身份与写入权限由 privacy coordinator 验证，Home 不把环境变量变成身份凭据。 -/
