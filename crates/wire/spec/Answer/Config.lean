-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::answer::config

规定 `answer::config`（`crates/wire/src/` 下同名的文件）。一个范围的生效配置，每个值旁边是说出它的那一级。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的帧形状由 Rust 的类型与 `crates/wire/tests/wire_contract.rs` 钉住的 wire schema（`wire::schema_hash`）守住。
-/

/-!
### 8-47e 缺省也是一层

`ConfigLayer` 多一个 `Default`（线上拼 `default`）；`ConfigAnswer::second` 从 `Option<SettledSecond>` 变成 `SettledSecond`：没有一级文件说过时，它是 `CTX_REMINDER_SECOND_DEFAULT` 且 `from = default`。字段换了形而名字没换，故同集进位 38→39。

- **缺席不是一个值**：`None` 让页面自己补上缺省，于是客户端带着第二份 `65`，和 kernel 的那一份之间没有任何东西把它们拴住。回答里写明生效值和它的来源，页面画出城说的数，只有一份。
- **`from = default` 是页面判断「这一级有没有写过」的依据**：`default` 时框留空、把生效值画成提示；否则框里是那一级写下的数。
- **合法域随值一起回答**：`SettledSecond::domain` 是 `SecondThreshold` 构造点读的那两个常量。页面仍不判域（拒因由城带回），但它画给人看的「31 到 90」不再是客户端自己写的第二份。
- **`effort` 不跟着变**：没有一级说过时生效的是提供方的缺省，城不知道那个值；为它编一个级别就是说一句城说不出的话。重开条件：城自己开始为 effort 定一个缺省值。
- **被否的方案**：保留 `Option` 并在旁边加一个 `default_percent` 字段——那样一个值有两个字段，读者要自己拼出生效值，拼法又多一个家。
-/

/-!
### 8-77 第一级提醒也随答复一起来

```rust
pub struct ConfigAnswer {
    // …既有字段…
    #[serde(default)] pub first: Option<u64>,   // `kernel::consts_policy::CTX_REMINDER_FIRST_PERCENT`，窗口的整数百分比
}
```

- **上下文环读的两级都在这一个答复上**：第二级是 `second.percent`，第一级是 `first`。页面不抄 30 这个数（docs/frontend-method.md §7J）。
- **不带层**：第一级不可调（`crates/kernel/Spec.lean` D19），没有哪一级文件说得出它，答一个恒为 `default` 的层只是多一个读者要处理的值。
- **`Option` 只为旧城**：这一版的城恒答 `Some`；理由与可缺的规则见 D13。
-/

/-!
### 8-86 网络搜索：生效值、说出它的那一级、城自己写的值与缺省地址

```rust
pub struct ConfigAnswer {
    // …既有字段…
    pub search: SettledSearch,
}
pub struct SettledSearch {
    pub configuration: kernel::config::SearchConfiguration,  // 这个地址上的 `web_search` 接哪一家
    pub from: ConfigLayer,                                   // 说出它的那一级；没有哪一级写 `[search]` 时为 `default`
    pub city: Option<kernel::config::SearchConfiguration>,   // 城自己那份 `CONFIG.toml` 写的值；没写为 `None`
    pub default_url: String,                                 // `Default` 接的地址，读自 `city::default_search_supplier`
    pub account_status: Vec<SupplierAccounts>,               // `city` 里每一家自定义供应方的账号 Key 状态
}
pub struct SupplierAccounts { pub supplier: kernel::ServerLabel, pub accounts: Vec<AccountStatus> }   // AccountStatus 见 §8-85
```

- **生效值与可编辑值分开答**：设置页只编辑城那一级（`crates/city/spec/ConfigLayers.lean` §8-4c），而这个地址上治理 `web_search` 的可能是这栋楼自己的 `[search]`。`configuration` 与 `from` 回答「这里实际接哪一家、谁说的」，`from = building` 时页面画一行只读的「此楼另有设置」；`city` 回答「城那一级写了什么」，编辑器从它起草。只答生效值时，一栋自己写了 `[search]` 的楼会让页面拿楼的值当城的值去改。
- **缺省地址只有一个家**：`default_url` 是 `city::default_search_supplier()` 的 `url`，页面在「默认」一项旁边画它，不写第二份。
- **Key 状态只答城那一级列出的账号**：页面能改的只有那些；楼一级覆盖的值只读，不需要它们的 Key 状态。`Default` 与 `Off` 列不出自定义账号，这一表为空。四态与 `EndpointSummary.account_status` 是同一个 `KeyState`（§8-85，D49）。
- **读不动就整份 `Unavailable`**：任一级的 `[search]` 读不成时，`city::settled_search` 拒，整个 `Config` 答复与读不动 `effort` 时一样是带原因的 `Unavailable`（D47）。
- `WIRE_V` 随本组改形进一位（D1、D49）。
-/
