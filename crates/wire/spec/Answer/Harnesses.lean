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

/-! D23 harness 一行说三态：启动程序缺失、harness 没装或没登录、可用

**决定**：`HarnessLine.found: bool` 换成 `state: HarnessState`：

```rust
pub enum HarnessState {
    LauncherMissing { program: String },          // 启动程序（多是 npx）不在搜索路径上
    NotSetUp { looked: Vec<String> },              // 启动程序在，但这家的安装或登录目录一个都不在；looked 是查过的路径，已展开
    Ready { at: String },                          // 找到的那个目录
}
```

每家 harness 查哪些目录是 `agent_protocols` 里这家登记的一张表，按平台分三栏（Windows 以 `%USERPROFILE%`、`%APPDATA%` 展开，macOS 与 Linux 以 `$HOME` 与各家文档写的环境变量展开，例如 Codex 的 `CODEX_HOME`）；表里的每一行引这家的官方文档，实现这一行的车道读文档填表，不猜。判定是只读的文件存在检查，三个平台同一个函数。改形不改名，`WIRE_V` 随 V0.0.9 的那一次进位（`crates/wire/Spec.lean` D22）。

**理由**：四家 harness 的启动程序都是 `npx.cmd`，装了 Node 就都显示「找到」（roadmap A5）。人要做的三件事不同：装 Node、装或登录这家 harness、直接用，所以要三态而不是两态；带上查过的路径，页面才能说「我们在这里找过」。

**被否**：①真的起一次 harness 问它版本：慢，且没登录的 harness 会弹登录；②只多一个 `installed: bool`：两个布尔有一种组合（没启动程序却装了）没有意义。

**重开参数**：某家 harness 提供了稳定的「是否已登录」命令时，`Ready` 改用它来判。
-/
