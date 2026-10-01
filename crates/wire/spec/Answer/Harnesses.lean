-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::answer::harnesses

规定 `answer::harnesses`（`crates/wire/src/` 下同名的文件）。harness 页：每一家官方 harness、起它的命令、城所在的机器跑不跑得了。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。
-/

/-!
### 8-52 设置页的 harness 页：`Query::Harnesses`

```rust
Query::Harnesses
Answer::Harnesses(HarnessesAnswer)
pub struct HarnessesAnswer { pub harnesses: Vec<HarnessLine> }
pub struct HarnessLine { pub name: String, pub launch: Vec<String>, pub found: bool, pub docs: String }
```

- **provider 页与 harness 页分开**（定规）：provider 页收 API key，harness 页说明五家官方 harness（`crates/agent_protocols/Spec.lean` §8-19）。
- `name` 是 `agent_protocols::Harness::as_str` 的词；`launch` 是起它说 ACP 的那条命令，逐词；`found` 是那条命令的程序在这台电脑的搜索路径上找不找得到；`docs` 是这家自己写的登录说明。**登录是人在 harness 里做的**，这一问不答任何凭据的事。
- 名字表多一项，schema 哈希因此而变，`WIRE_V` 不为此进位。
-/
