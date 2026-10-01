-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::answer::known_hosts

规定 `answer::known_hosts`（`crates/wire/src/` 下同名的文件）。城按 host 认得的厂商，每一面带登记时会存下的 base URL。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。
-/

/-!
### 8-51 设置页的厂商表：`Query::KnownHosts`

```rust
Query::KnownHosts                                   // 无参数；答案不随城变
Answer::KnownHosts(KnownHostsAnswer)
pub struct KnownHostsAnswer { pub hosts: Vec<KnownHost> }
pub struct KnownHost { pub host: String, pub faces: Vec<KnownFace> }
pub struct KnownFace { pub dialect: DialectKind, pub base_url: String }
```

- **一个人挑厂商，而不是去厂商文档里复制一个地址。** 本城认得的 host 住 `gateway::provider::preset`（`crates/gateway/Spec.lean` §8-17），设置页经这一问读它：每个 host 说几面、每面的 base URL 是什么。`base_url` 是这座城登记时自己会算出的那个地址（`normalise_entered`），所以页上填进框里的与登记下来的是同一串。
- **客户端据同一答案决定哪几面可选**：一个 host 不说的那一面在控件上拒点，理由写出它说的几面。客户端不再持自己的 host 表。
- **一问而不是塞进 `EndpointsAnswer`**：那个答案说的是这座城登记了什么，随账本变；这一问说的是本城认得哪些厂商，只随二进制变。合成一个答案，会让每一次登记都重发一份不变的表。
- 名字表多一项，schema 哈希因此而变，`WIRE_V` 不为此进位。
-/
