-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::answer::mcp_health

规定 `answer::mcp_health`（`crates/wire/src/` 下同名的文件）。一个地址够得到的每台工具服务器站在哪、供什么。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的帧形状由 Rust 的类型与 `crates/wire/tests/wire_contract.rs` 钉住的 wire schema（`wire::schema_hash`）守住。
-/

/-!
### 8-34 一台工具服务器带着 `claude mcp add` 允许写的东西上线，`McpHealth` 答它站在哪

`McpTransport` 的三支携 MCP 页的环境变量表、多行请求头与 `sse`，且**不标 `#[non_exhaustive]`**（全库如此）：本枚举的读者全在这一个二进制里，通配臂什么也换不来，却会把下一种 transport 从必须表态的三个模块面前藏起来。

```rust
pub enum McpTransport {
    Stdio { command: String, args: Vec<String>, env: Vec<(String, String)> },
    Http  { url: String, headers: Vec<(String, String)> },
    Sse   { url: String, headers: Vec<(String, String)> },
}

// Query 第 29 条（声明序，QUERY_NAMES 同序追加）
McpHealth { addr: Address },              // → Answer::McpHealth(Box<McpHealthAnswer>)

pub struct McpHealthAnswer { pub addr: Address, pub servers: Vec<McpServerHealth> }
pub struct McpServerHealth { pub label: ServerLabel, pub transport: String,
                             pub target: String, pub state: McpState }
pub enum McpState {
    Connected { protocol_version: String, server: String, tools: Vec<McpToolLine> },
    Authenticating { recovery: String },
    Failed { refusal: Box<AxError> },
}
pub struct McpToolLine { pub remote: String, pub name: String,
                         pub disclosure: String, pub input_schema: Payload }
```

**五条口径：**

1. **名与值成对而不是一行 `"Name: value"`**。旧写法把「怎么切这一行」留给了读者，而一个值里合法地含冒号；成对之后拆分这件事不存在，写的人与读的人也不必约定同一条切法。值可以是 `secret:realm/name` 引用，兑付点在离线最近的那一格（`crates/sprawling/Spec.lean` §8-4／§8-15）。
2. **`Sse` 是自己的一支而不是 `Http` 的一个开关**。两者开法不同、败法也不同：http 服务器拒绝一次请求，而流式服务器可以接下每一次请求却一次也不答。
3. **三种状态穷尽，而不是一个标志加一句可选的理由**。「在答」「要登录」「失败了，原因在此」是人接下来要做的三件不同的事；理由可选的形状会让一次失败不带理由地上线。`Authenticating` 的判断只有一个来源——传输层对 401／403 抬的 `E_CREDENTIAL_MISSING`，故这里没有什么要猜。
4. **失败携整条三段式拒绝**，界面逐字画出：措辞的权威在城里，页面再写一遍就是第二个权威。
5. **这是唯一一条按秒计的读**，每台服务器一次 `initialize` ＋ `tools/list`，且用的就是 Run 起点那一次握手（`agent_protocols::mcp`）。它恒不由记录触发、也不上定时器——人打开 MCP 页或加完一台服务器时问一次。**被否**：把握手结果写进账本再折出来——一台服务器此刻通不通是关于此刻的事实，记下来的那一份会在它停掉一小时后仍说它在。
-/
