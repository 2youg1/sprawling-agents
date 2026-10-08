-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::answer::harnesses

规定 `answer::harnesses`（`crates/wire/src/` 下同名的文件）。harness 页：每一家官方 harness、起它的命令、城所在的机器跑不跑得了。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的帧形状由 Rust 的类型与 `crates/wire/tests/wire_contract.rs` 钉住的 wire schema（`wire::schema_hash`）守住。
-/

/-!
### 8-52 设置页的 harness 页：`Query::Harnesses`

```rust
Query::Harnesses
Answer::Harnesses(HarnessesAnswer)
pub struct HarnessesAnswer { pub harnesses: Vec<HarnessLine> }
pub struct HarnessLine { pub name: String, pub launch: Vec<String>, pub state: HarnessState, pub docs: String }
pub enum HarnessState { LauncherMissing { program: String }, NotSetUp { looked: Vec<String> }, Ready { at: String } }
```

- **provider 页与 harness 页分开**（定规）：provider 页收 API key，harness 页说明五个内置条目，也就是五家官方 harness（`crates/agent_protocols/Spec.lean` §8-19）；任意 ACP agent 的添加在 ACP 页（`Answer/Agents.lean` §8-90）。
- `name` 是 `agent_protocols::Official::word`；`launch` 是起它说 ACP 的那条命令，逐词，取自随版本附带的目录快照；`state` 是这台电脑能不能用它（D23 的三态）：启动程序不在搜索路径上答 `LauncherMissing`，`program` 是那个程序名；启动程序在时，按 `agent_protocols::Official::set_up` 那张表逐行查：第一个存在的目录答 `Ready { at }`；一个都不在答 `NotSetUp`，`looked` 是查过的路径，已展开；这家的表是空的（厂商文档没写这样一个目录），`looked` 为空，读作「没有查过任何目录」，而不是「查过都不在」。`docs` 是这家自己写的登录说明。**登录是人在 harness 里做的**，这一问不答任何凭据的事。
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

当前的表（`agent_protocols::OFFICIAL` 的 `set_up`），每行的出处就是行里的那页：

| harness | 变量 | 家目录下 | 出处 |
|---|---|---|---|
| `claude_code` | `CLAUDE_CONFIG_DIR` | `.claude` | https://code.claude.com/docs/en/settings |
| `codex` | `CODEX_HOME` | `.codex` | https://developers.openai.com/codex/auth |
| `pi` | `PI_CODING_AGENT_DIR` | `.pi/agent` | https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/environment-variables.md |
| `grok_build` | `GROK_HOME` | `.grok` | https://docs.x.ai/build/settings/reference |
| `kimi_code` | `KIMI_CODE_HOME` | `.kimi-code` | https://moonshotai.github.io/kimi-code/en/configuration/data-locations |

这五家的文档对三个平台写的是同一个位置：家目录（Windows 的 `%USERPROFILE%`，macOS 与 Linux 的 `$HOME`）下的一个目录，设了变量就换成变量的值，所以三栏合成一栏；某家给某个平台另写了位置时，这一行按平台分开。家目录由 `bin::doctor::host` 读（`accounting::home::Home::detect`，先 `USERPROFILE` 后 `HOME`），变量由它读环境；views 只经 served city 交进来的那个函数读这台电脑。

表里的每一行引这家的官方文档，不猜。判定是只读的文件存在检查，三个平台同一个函数。改形不改名，`WIRE_V` 随 V0.0.9 的那一次进位（`crates/wire/Spec.lean` D22）。

**理由**：四家 harness 的启动程序都是 `npx.cmd`，装了 Node 就都显示「找到」（roadmap A5）。人要做的三件事不同：装 Node、装或登录这家 harness、直接用，所以要三态而不是两态；带上查过的路径，页面才能说「我们在这里找过」。

**被否**：①真的起一次 harness 问它版本：慢，而且那是在人同意之前执行它的程序，违反「同意先于任何执行」（`crates/agent_protocols/Spec.lean` D16）；按 ACP，`initialize` 在认证之前，不会弹登录，所以否掉它的理由只剩这两条；②只多一个 `installed: bool`：两个布尔有一种组合（没启动程序却装了）没有意义。

**重开参数**：某家 harness 提供了稳定的「是否已登录」命令时，`Ready` 改用它来判。
-/
