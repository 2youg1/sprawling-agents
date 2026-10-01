-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# kernel::schema

规定 `kernel::schema`（`crates/kernel/src/schema.rs`）：线上每个值的 JSON Schema。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。
-/

/-!
### 8-45 kernel::schema（形状 4 adapter）——线上每个值的 JSON Schema，从 serde 读的那一份声明派生

**需求**：客户端（`client/src/wire.ts`）由 Rust 的 wire 类型生成，而 wire 携带的值大半是 kernel 的（`RunId`／`Seq`／`Address`／`EventRecord`／`AxError`／`ApprovalItem`……）。它们的 JSON 形状必须有且只有一个权威，而那个权威已经存在：类型声明上的 `#[serde(...)]`。

**接口**：feature `schema`（缺省关，`schemars` 为可选依赖）。开启时，每个出现在帧里的类型带 `#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]`——派生宏读的正是 serde 读的那些属性（`rename_all`／`transparent`／`flatten`／`skip_serializing_if`／`deny_unknown_fields`／`try_from`），所以形状不可能与编码漂开。派生说不出语法的值在 `crates/kernel/src/schema.rs` 里各写一条 `impl JsonSchema`：一律是带 `pattern` 的 `string`，`AxCode` 的 `enum` 取自 `AxCode::ALL`——同一张表既产 `as_str` 也产 schema。前五个（`AxCode`／`IdemKey`／`B3Hash`／`GitOid`／`Locator`）的 serde 本就是手写的；第六个 `Address` 的 serde 是 `transparent`，派生因此只说得出「一个字符串」，而客户端要知道的恰是哪些字符串算数——`ADDRESS_PATTERN` 把 `Address::parse` 收的那套语法写成一条正则（`\p{Cc}` 即 `char::is_control`，`\p{White_Space}` 即 `char::is_whitespace`，读它的引擎一律开 Unicode 语义），`cargo xtask wire-ts` 据此发出 `Schema.pattern`，客户端不再自备一份语法。`ServerLabel` 同理：它的 serde 经 `try_from = "String"`，派生只说得出「一个字符串」，`SERVER_LABEL_PATTERN`（`^[a-z0-9]+$`）把 `ServerLabel::parse` 收的那套语法写成一条正则，MCP 页用 `Schema.is(ServerLabel)` 判标签，不自备第二份文法。判定权威仍是 `Address::parse`：它答得出**违反了哪一条**，那是一个人需要的。这些 impl 住一个文件而不是各回原文件，因为它们不是所在模块的判定，而 `locator.rs` 再收三条就越过文件行数上限；文件只装 `impl` 与它们引用的形状字符串，一处判定也没有。

**缺省公开面不变**：feature 关着时 `cargo public-api -p sprawling-kernel` 逐字节同以前，基线不动；`--all-features` 下多出的只是 `JsonSchema` 实现。产品二进制不开它。

**被否**：（a）在 wire 用 schemars 的 remote derive 镜像这些类型——每一个镜像都是同一形状的第二个权威，kernel 改一个字段名，镜像静默不动，客户端在握手通过后误读；（b）不加 feature、无条件派生——把 `schemars` 压进产品二进制，换来的只是省一个 cfg。
-/
