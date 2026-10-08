-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::answer::agents

规定 `answer::agents`（`crates/wire/src/` 下同名的文件）：ACP 页读的 agent 目录、一段粘贴读成的一个 agent 条目，以及已添加的 agent。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的帧形状由 Rust 的类型与 `crates/wire/tests/wire_contract.rs` 钉住的 wire schema 守住。
-/

/-!
### 8-90 ACP 页的三问：`Query::AgentCatalog`、`Query::ParseAgentSpec` 与已添加的 agent

```rust
Query::AgentCatalog                                  // 无参数
Answer::AgentCatalog(Box<AgentCatalogAnswer>)
pub struct AgentCatalogAnswer {
    pub detected: Vec<AgentOffer>,     // 本机上找到的 agent，一项一张同意卡
    pub catalog: Vec<AgentOffer>,      // 随版本附带的目录快照（含五个内置条目）
    pub added: Vec<AgentLine>,         // 城里已添加的 agent
    pub snapshot: CatalogSnapshot,
}
pub struct CatalogSnapshot { pub date: String, pub etag: String }   // 快照取自 registry 时的 `Date` 与 `ETag`

Query::ParseAgentSpec { text: String }
Answer::AgentSpec(Box<AgentOffer>)                  // 读不成一个条目时整问以拒绝作答（`AskOutcome::Refusal`）

pub struct AgentOffer {
    pub id: String,
    pub name: String,
    pub source: AgentSource,
    pub launch_preview: String,        // 同意卡上照原样显示的那一行命令
    pub version: Option<String>,
    pub pinned: PinState,
    pub licence: Option<String>,
    pub env_names: Vec<String>,        // 它设的环境变量的名字，不带值
    pub login: Vec<LoginKind>,
    pub spec_digest: B3Hash,           // 这份启动说明的摘要；`AddAgent` 带回它作为同意
}
pub enum AgentSource { Registry, Detected, Pasted }   // 线上 snake_case
pub enum PinState { Exact, Floating, Unknown }        // 线上 snake_case
pub enum LoginKind { Agent, Terminal }                // ACP 稳定 schema 的两种 `authMethods` 类型

pub struct AgentLine {
    pub id: String,
    pub name: String,
    pub source: AgentSource,
    pub version: Option<String>,
    pub pinned: PinState,
    pub login_state: LoginState,
    pub auth_methods: Vec<AuthMethod>,
    pub seated_in: Vec<Address>,       // 哪些房间的 `[resident] harness` 指着它
}
pub enum LoginState { Unasked, Ready, Required }      // 线上 snake_case
pub struct AuthMethod { pub id: String, pub name: String, pub kind: LoginKind }
```

- **同意就是摘要**：`AgentOffer.spec_digest` 是城对它给出的那份启动说明算的摘要，`AddAgent` 把它带回来；城重算它此刻会给出的那一份，不相等就拒，所以人同意的那一行命令就是城写进 `[[agent]]` 的那一行（`crates/wire/spec/Command/Kind.lean` §19-2）。
- **`launch_preview` 照原样显示**：同意卡上显示的是确切的命令行，页面不改写、不截断；路径与命令行只在答复与 `CONFIG.toml` 里，不进账本（`crates/kernel/Spec.lean` §8-87）。
- **只给名字，不给值**：`env_names` 只列它设的环境变量的名字；值可能是 `secret:realm/name` 引用，也可能是 registry 写的关掉自更新的开关，页面都不需要。
- **`PinState` 三态**：`Exact` 是启动说明钉在一个确切的版本上（`pkg@1.2.3`）；`Floating` 是交给启动程序去解析版本（`@latest` 或不写）；`Unknown` 是本机上找到的一个程序，城没有执行它，所以不知道它是哪一版。
- **登录只在需要时出现**：`LoginState::Unasked` 是城还没有与它开过会话；`Required` 是它以 `-32000` 答过（`E_AUTH_REQUIRED`），页面这时才给登录入口；`Ready` 是它接过一次会话而没有要求登录。`auth_methods` 是 agent 在 `initialize` 里声明的方法，已滤掉城不代为启动的那一种（claude.ai 订阅登录，人的裁定）。
- **粘贴的文法只在 Rust 里**：`ParseAgentSpec` 读一行命令（空白与双引号分词，含 `|`、`&`、`;`、`<`、`>` 的拒绝）、Zed 或 JetBrains 的 `agent_servers` 块，或 registry 的 `agent.json`，WebUI 与 CLI 都经它，页面不另写一份。
- **现状**：三问在线上，城以 `Answer::Unavailable` 答 `AgentCatalog`、以拒绝答 `ParseAgentSpec`，直到目录、检测与粘贴的读法落地（`crates/agent_protocols/Spec.lean`）。`Query::Harnesses` 与 `HarnessLine`（§8-52）仍在线上，供今天的 harness 页读；ACP 页改读 `AgentCatalog` 的那个变更集删去它们，五个官方 harness 从那时起是 `catalog` 里的内置条目。
-/
