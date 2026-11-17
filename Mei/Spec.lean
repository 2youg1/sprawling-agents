-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import Mei.spec.Agent
import Mei.spec.Session

/-! # Mei 的规格

`Mei/` 是极简 Agent Harness：一个可变 Anthropic Messages 请求、一个串行反馈循环、
一个有状态 Python 进程。Cargo 包名 `sprawling-mei`，库名 `mei`，无头命令名 `mei`。
唯一工具名为 `Python3`；小型 UTF-8 输出直接返回，大输出只返提示与完整文件路径。
终端居住面属于顶层生产包 `Calyx/`（非 Archetype）；Mei 核不含 TUI。
指令 / MCP / Skills 的权威在 magpie；Agent 自去 skill 仓库。运行时不注入 `MEI.md`。
-/

/-! ## 1 需求分解

| 单元 | 责任 |
|---|---|
| `main` | 装配环境、参数、stdin/stdout、退出与解释器收口 |
| `agent` | 唯一反馈循环；只读 JSON 与 `std::io::Write` |
| `transport` | 非流式 Anthropic Messages HTTP/1.0；阻塞读时可写存活观测 |
| `python` 与 `boot.py` | 持久 Python3；完整输出落盘；小输出内联，大输出只给提示与路径 |
| `error` | 人可见诊断：Description / Cause / Resolution |
| `MEI.md` | 构建时静态系统提示；改之须重建；不作运行时注入面 |
| `spec/Agent.lean` | 反馈顺序与整波预检的形式模型 |
| `spec/Session.lean` | 完整日志恢复的形式模型 |

禁止：第二本地工具、`MEI_TOOLS`、阈值截断/头尾窗、内置超时 alarm、核内 TUI、
LLM compaction、验真门、运行时 AGENTS/Skills 注入、为弱模型准备的重试。
交互斜杠与会话文件由 Calyx 拥有；无头 `mei` 仍可纯脚本使用。
-/

/-! ## 2 验收标准

- `spec/Agent.lean` 与 `agent::run` 闭包测试：整波预检、顺序执行、thinking 保真、
  pause、max_tokens、异常 stop、执行失败不伪造 tool_result。
- TCP 工装：HTTP/1.0、EOF、非 200 原文、chunked 拒绝、停滞返回 `E_MEI_TRANSPORT`。
- 真实 Python3：变量、imports 与 cwd 跨调用保留；异常打印 traceback 后不结束会话，
  下一调用可继续使用既有状态；表达式不自动回显；每次 `run` 的 tool_result 是完整的小型 UTF-8 捕获；
  文件字节等于该次捕获的 stdout+stderr；空输出返回空字符串；
  超过 16 KiB 或非 UTF-8 时只返回一条提示与完整文件路径，不内联头尾片段；
  下一次 Python3 调用可按路径读取、筛选或切片后打印；`sys.executable` 可启动并捕获子 Python；
  非法帧与解释器死亡为核错误；Windows 与 Unix 均可 spawn。
- stderr：消息 JSONL；可选 `{"type":"mei.obs",...}` 观测行（pulse / usage）；
  失败诊断为 Description/Cause/Resolution 的 toml 块。
- stdout：最终 text，不加换行。
- resume：完整日志的首条 user content 可为字符串或内容块数组；旧工具名、路径与媒体块原样保留，
  不恢复 Python 现场。Provider 是否接受图片/音频/视频由其模型合同决定，不由 JSONL 透传测试代替。
- live V1/V2（magpie）须实测后宣称；范围以本节记录为准。
-/

/-! ## 3 假设与歧义

- 超时由外层进程管理；核内不设 `SIGALRM`。
- 调用者提供隔离与凭证；Mei 继承调用者 FS/网络权限。
- `pause_turn` 续跑；非法控制帧使运行失败；`HARNESS` 未预设才回填；整波先预检。
- 串行只约束主循环的请求与调用；模型留下的线程可并发，结果以落盘文件为准。
- 递归子进程共享 FS，不共享 messages 或 Python 命名空间。
-/

/-! ## 4 现状分析

核去掉输出窗与 POSIX-only 拒绝；删除 `MEI_TOOLS`。交互后端在 `Calyx/src/console`，
入口为 `calyx` 二进制（npm：TTY 且无 `-p`／`--resume` 时由 shim 启动）并通过 `MEI_BIN`／同目录
`mei`／PATH 解析无头核。`@sprawling/mei` 归档同时携带无头核与 Calyx 面（含 Windows）。
V1/V2 实测以 magpie + 已测模型为准，不外推。
-/

/-! ## 5 权威信源

- User ruling：极简核；小输出直接观察，大输出只给提示与路径；Calyx 为生产终端壳；无运行时注入；
  诊断 Description/Cause/Resolution；Mei 不含壳（核进程不内嵌 TUI）；一次完成——
  npm 以机制装配交互面。
- Anthropic Messages / client tools；[strict tool use](https://platform.claude.com/docs/zh-CN/agents-and-tools/tool-use/strict-tool-use)；
  magpie reference（`docs/third-party.md` §1）。
- Rust `std::process::Child`：须显式收口。
- `Calyx/Spec.lean`：终端与斜杠；本文件不复制。
-/

/-! ## 6 命名统一

状态只有 `request: serde_json::Value`。`Python3` 是唯一本地客户端工具。
tool_result 的 `content` 是小型完整输出，或“提示 + 完整结果路径”；不含 BLAKE3。
`pause_turn` 不是取消或持久化。观测行 `type=mei.obs` 不是 Messages role。
`Python3` 是有状态执行器而非 REPL：表达式不自动回显，`print` 是主动观察。
-/

/-! ## 7 模块边界

`agent` 决策；`transport` / `python` 适配；`resume` 读完整 JSONL；`main` 装配；
`error` 诊断。生产依赖只有 `serde_json`。Calyx（及任何 Face 消费者）在进程边界
spawn 无头 `mei`；Mei 不知道 Entry/Zig。

`ANTHROPIC_BASE_URL` 出站默认 `http://127.0.0.1:3425`；Mei 不监听 2501/3425。
-/

/-! ## 8 接口先行

```rust
pub fn run(
    request: &mut Value,
    post: impl FnMut(&Value, &mut dyn Write) -> Result<Value>,
    exec: impl FnMut(&str) -> Result<String>,
    trace: &mut dyn Write,
) -> Result<String>;

impl Client {
    pub fn new(base: &str, key: String) -> Result<Client>;
    pub fn post(&self, request: &Value, pulse: &mut dyn Write) -> Result<Value>;
}

impl Py {
    pub fn spawn() -> Result<Py>;
    pub fn run(&mut self, code: &str) -> Result<String>; // Ok = tool_result content
    pub fn close(&mut self) -> Result<()>;
    pub fn kill_and_wait(&mut self) -> Result<()>;
}
```

stderr 观测行（穷尽；非 Messages role）：

- `{"type":"mei.obs","kind":"pulse"}` — 阻塞读满一个 pulse 间隔仍无字节时写一行
- `{"type":"mei.obs","kind":"usage","usage": <response.usage>}` — 响应带 usage 时写一行

`Diagnostic` 渲染 toml 块字段：`code`、`description`、`cause`、`resolution`，
以及可选 `path` / `value`（写入 cause 叙事所需处）。内部仍可用 action/subject/recovery
构造，对外版式固定为三段。`Body` 仅非 200 原文。

启动请求：`model`、`max_tokens`（默认见 §14）、`system`、`tools = [Python3]`、messages。
工具描述与持久变量/imports/cwd、异常后恢复、主动打印、16 KiB 内联预算、完整捕获路径、
subprocess 和 `sys.executable` 一致；内联数字由 `INLINE_BYTES` 生成；不写 REPL 或核内超时。
唯一工具 schema 与 agent 整波预检跨 provider 生效，不声明 provider 专有的 `strict` 模式。
`HARNESS_EXTRA` 浅合并一次；拒绝 `stream: true` 与 `HARNESS_EXTRA.tools`。
无 `MEI_TOOLS`。`HARNESS_MODEL` 必填。

无头语法：`mei [-p PROMPT] [--resume JSONL]`；无 `-p` 时非 TTY stdin 为 prompt。
恢复只带 role/content；环境重定 model/system/tools/网关；Python 重启。

发布：`sprawling-mei` / `@sprawling/mei`；平台含 Windows；平台包含 `mei` 与 `calyx`。
交互经 `calyx` + 无头核解析（Calyx D4）；城控制台仍经 `sprawling`。打包次序归
`Tools/xtask/Spec.lean` D31。
-/

/-! ## 9 工作流程

```text
A = post(R)
可选 trace mei.obs usage / 阻塞中的 pulse
trace 本次 user prompt（首个有效响应）
R.messages.push(assistant); trace
pause_turn → continue
end_turn → return text
tool_use → 预检 → exec 得捕获或小结果提示 → user tool_result content=文本 → continue
其它 stop → fail
```

取消 = 杀掉 harness 进程组。无 turn ceiling。
会话斜杠与 JSONL 文件代次由 Calyx 合同拥有。
-/

/-! ## 10 实现逻辑

- 传输：HTTP/1.0、Connection: close、读至 EOF；CONNECT 10s、IO 600s；
  阻塞读期间可按固定间隔写 `mei.obs` pulse，不改 messages。
- `boot.py`：每帧一行 JSON string；stdout/stderr 重定向到临时文件；同一执行命名空间保留
  变量与 imports，解释器进程保留 cwd；异常只形成 traceback 反馈，循环继续；
  结束后把**该文件路径**经控制信道回传；不改写文件、无 alarm。
- `Py::run` 先读文件元数据；仅不超过 16 KiB 时做有界读取。完整 UTF-8 文本直接作为
  tool_result；更大或非 UTF-8 时只给提示与路径，完整文件保留供下一次调用筛选。
- 空输出：仍创建零字节文件，tool_result 为空字符串。
- `close`：5s 宽限后 kill+wait；成功收口的强制清理不是运行失败。
-/

/-! ## 11 边界枚举

空/Unicode prompt、stdin EOF、重复参数、恢复残缺、缺模型、非法 URL、非 200、
chunked、停滞、空工具波、未知客户端工具、重复 id、max_tokens、pause、
解释器死亡、非法帧、非 UTF-8、空文件路径、收口非 daemon 线程、取消杀组。
-/

/-! ## 12 错误处理

核错误退出 1；不重试。Python traceback 在结果文件内，小型 traceback 直接进 tool_result。
非 200 正文原样 stderr。诊断不泄露 API key。
-/

/-! ## 13 依赖选型

仅 `serde_json`；std TcpStream/Command/闭包。被否：reqwest、clap、Model/Tool trait。
-/

/-! ## 14 硬编码声明

`max_tokens` 默认 `65536`（可被 `HARNESS_EXTRA` 覆盖）、连接 10s、IO 600s、
收口宽限 5s、poll 20ms、输出内联上限 16 KiB、默认网关与 key、
`anthropic-version: 2023-06-01`、pulse 间隔 5s、各一处权威。`MEI.md` 构建时注入。
`INLINE_BYTES` 同时驱动内联判定、超限提示和工具描述的数字，没有第二份阈值或换算目标。
16 KiB 覆盖本仓绝大多数源码/文档，避免普通文件多读一轮；无头尾采样、无 250s alarm。
-/

/-! ## 15 影响面

Calyx 依赖进程 Face；City 账本/wire 不变。接口变化先改本文件与 Agent/Session，
再改实现与测试。
-/

/-! ## 16 测试与约束

```bash
cargo nextest run -p sprawling-mei
cargo clippy -p sprawling-mei --all-targets --all-features -- -D warnings
lake build Mei.Spec Mei.spec.Agent Mei.spec.Session
cargo test -p sprawling-mei --test cli
cargo test -p sprawling-mei --test observation
```

Windows 与 Unix 都实际执行 beginner E2E；网关只有在下一请求确实携带小输出时才回答正确结果。
大输出 E2E 钉住不内联与下一次 Python3 按路径筛选；测试失败不得退化为 0 tests。
live V1/V2 以 §2 为准。
-/

/-! ## 17 文档关系

`Mei/README*.md`；`Calyx/Spec.lean`；`architecture.toml`；`docs/ARCHITECTURE.md`；
`docs/third-party.md`。斜杠与 TUI 细节以 Calyx 为准。
-/

/-! D1 请求 JSON 唯一状态；闭包替代单实现 trait。 -/
/-! D2 非流式 HTTP/1.0 读至 EOF；被否 reqwest/SSE/chunked/重试。 -/
/-! D3 完整输出永远落盘；不超过 16 KiB 的 UTF-8 输出直接作为 tool_result，更大或非 UTF-8 输出只给提示与路径。理由：模型必须看见主动打印的小结果，而巨大输出会成为上下文 DoS；下一次仍用 Python3 读取、筛选或切片。被否：纯路径（没有观察通道）、全量内联（上下文 DoS）、头尾截断（伪造完整观察）、第二读取工具（小输出多一轮）。不加 BLAKE3：当前没有校验、缓存或内容寻址需求。 -/
/-! D4 核不知 TUI；终端属 Calyx；被否核内 Entry 类型。 -/
/-! D5 删除 `MEI_TOOLS`；透明服务端工具不由 Mei 声明；`pause_turn` 续跑。 -/
/-! D6 `MEI.md` 仅构建时注入；运行时注入归 magpie。 -/
/-! D7 `close` 5s 后 kill；被否无限 wait。 -/
/-! D8 传输 10s/600s；被否无时限与过短时限。 -/
/-! D9 会话斜杠与 JSONL 代次归 Calyx；核只提供 `--resume` 消息恢复。 -/
/-! D10 恢复路径保留 OsString；诊断带实际 value。 -/
/-! D11 crate `sprawling-mei`、npm `@sprawling/mei`；发布含 Windows；只发无头核；
交互走 `sprawling mei`（Calyx）；被否核内嵌壳、根包装配 Calyx 进 `mei` 命令、为未来工具留槽。
理由：Mei 不含壳；Calyx 同时服务城控制台，权威在 sprawling 依赖而非 Mei 安装面。 -/
/-! D12 人可见诊断版式为 Description / Cause / Resolution。 -/
/-! D13 无核内超时；超时外置。 -/
/-! D14 不声明 provider 专有 `strict`；唯一 schema 与 agent 整波预检是跨 provider 合同。
理由：Anthropic 的 strict 模式可约束 Claude 的工具输入，但默认 magpie 0.1.4347 的
`internal/gateway/anthropic.go` 既不读入工具级 strict，也不在上游构造中转发；启用会在默认路径静默失效。
当前 schema 只有一个必需 string，agent 已拒绝未知工具、缺失或错型 code、重复 id 与空调用波。
被否：为 Claude 增加配置或第二套规格，以及宣称默认路径已获得语法约束。重开参数：默认网关端到端保留
该字段，且目标模型实测证明它能阻止本会终止运行的 malformed tool call。 -/
