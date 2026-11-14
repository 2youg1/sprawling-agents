-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import Mei.spec.Agent

/-! # Archetype:Mei 的规格

`Mei/` 是学术探索与二次开发原型 `Archetype:Mei`（简称 Mei），Cargo 包名
`sprawling-mei`，库名和命令名 `mei`。它探索最极简 Agent Harness 的可行性：
一个可变 Anthropic Messages 请求、一个串行反馈循环、一个有状态 Python 进程。
它不是可直接投入使用的成品；Provider、shell、git 与隔离都需要调用者自行搭配和调试，
主要用途是二次开发与技术讨论。
-/

/-! ## 1 需求分解

| 单元 | 责任 |
|---|---|
| `main` | 装配环境、参数、stdin/stdout、退出与解释器收口 |
| `agent` | 唯一反馈循环；只读 JSON 与 `std::io::Write`，不认识 HTTP 或子进程 |
| `transport` | 非流式 Anthropic Messages HTTP/1.0 客户端 |
| `python` 与 `boot.py` | 持久 Python 3 解释器、私有控制帧、输出快照与超时反馈 |
| `error` | 稳定错误码、失败动作、主题、违反规则与可执行恢复步骤 |
| `MEI.md` | 唯一静态系统提示词，构建时注入；二次开发可直接修改后重建 |
| `spec/Agent.lean` | 反馈顺序、整波预检、终止与保真的形式模型 |

核心没有会话恢复、权限、成本、审查、专用文件工具、异步运行时、Provider 抽象、
SSE、重试、压缩、记忆、Skills、MCP/ACP、图片输入或原生 Windows 支持。
隔离由外层容器、VM、bwrap 或专用用户承担；没有内部安全边界。
-/

/-! ## 2 验收标准

- `spec/Agent.lean` 证明：每次转移先追加 assistant；不合法工具波在执行前失败；
  合法调用按 content 顺序得到同 ID 结果；`pause_turn` 只重发 assistant；异常 stop 不执行工具。
- `agent::run` 可用脚本闭包走完直接回答、多轮工具、字符串 content、整波预检、
  重复 ID、thinking 保真、pause、max_tokens、异常 stop 与执行失败，不需要真实模型。
- TCP 工装验证请求方法、路径、HTTP/1.0、UTF-8 字节 Content-Length、固定 header、
  响应 EOF、非 200 原文与 malformed 防御性诊断；没有 chunked、SSE、代理或重定向。
- 真实 Python 进程验证变量保留、顺序调用、异常与 SystemExit、子进程 stdin EOF、
  fd 输出捕获、非 UTF-8、空输出、大输出头尾截断、超时后继续、非法帧和解释器死亡。
- 整波在任何副作用之前验证；解释器死亡后不伪造完整 user/tool-result 历史。
- stdout 只含最终 text blocks，不加换行；stderr 每条新加消息一行相同 JSON，其余行是普通错误。
- live V1/V2（真实 magpie 非流式 EOF、非 Anthropic 模型工具调用与 thinking 回传）
  必须实测后才能宣称支持；本地生产路径验证不替代这两项。
- 实测记录：`curl --http1.0 -i -X POST http://127.0.0.1:3425/v1/messages` 在
  `deepseek/deepseek-flash` 上返回 HTTP 200、`Content-Length` 与完整 JSON，验证 V1；
  `HARNESS_EXTRA='{"thinking":{"type":"enabled","budget_tokens":1024}}' target/debug/mei`
  在同一模型上完成 Python `print(40+2)`，工具结果为 `42\n`，最终 stdout 为 `42`，
  后续请求接受带原 signature 的 thinking 块，验证 V2。其它模型及兼容格式不作外推。
-/

/-! ## 3 假设与歧义

- 参考环境是 POSIX：`SIGALRM`、`python3` 与 fd 语义属于边界，其他平台只保证能编译并明确拒绝运行。
- 调用者提供隔离与凭证。Mei 继承调用者文件系统、网络和环境权限，不在带真实密钥或不可替代数据的环境运行。
- 评审补充与原始封闭表的冲突按以下当前决定解决：`pause_turn` 继续；`MEI_TOOLS` 追加透明服务端工具；
  非法控制帧使运行失败；`HARNESS` 未预设才回填；整波先预检；显式收口子进程。
- `SIGALRM` 是 Harness 保留资源，每轮重设 handler 并在 `finally` 取消 alarm；但执行代码仍可捕获异常、
  修改或忽略信号，C 调用也可能不返回，所以 250 秒只是 best-effort，不是硬截止。
  取 250 秒是为了让模型代码等得起网络波动恢复（例如重插线缆后的重连），不是硬截止的替代。
- “串行”只约束主循环发起的请求和调用。模型代码留下的线程和进程可并发，输出是调用结束时的快照。
- `os.dup` 只防止常规 exec 型子进程继承私有控制 fd；裸 `fork` 的子孙仍可能继承它，不作更强的私有性承诺。
- 递归子任务整体处于父 Python 调用的 best-effort 超时内。默认只读子进程 stdout；
  模型主动打印捕获的 stderr 时，中间过程仍可进入父上下文。
-/

/-! ## 4 现状分析

本 crate 从封闭设计新建，没有旧实现需要保持兼容。Rust 核心与 Sprawling TUI Mei 后端
按照本规格实现。真实 magpie 与目标模型的 V1/V2 是独立验收门，脚本化工装不冒充这项事实；
上述实测已覆盖一个非 Anthropic 模型，不声称覆盖其它 Provider 的兼容格式。
-/

/-! ## 5 权威信源

- User 的 Archetype:Mei 封闭设计及后续 ruling：顶层 `Mei/`、学术原型定位、`src/MEI.md`、端口约束。
- Anthropic Messages 与 client tools 官方文档，以及[官方 TypeScript SDK 声明](https://github.com/anthropics/anthropic-sdk-typescript/blob/main/src/resources/messages/messages.ts)
  （跟踪版本见 `docs/third-party.md` §1）：
  `/v1/messages`、`tool_use`/`tool_result`、thinking 原样回传、string content、`pause_turn` 与透明服务端工具块。
- [magpie reference](https://github.com/yetone/magpie/blob/main/docs/reference.md)（跟踪版本见 `docs/third-party.md` §1）：
  默认网关 `127.0.0.1:3425`、任意 loopback key、`provider/model`、Anthropic Messages 转译。
- [Python `signal` 官方文档](https://github.com/python/cpython/blob/main/Doc/library/signal.rst)（跟踪版本见 `docs/third-party.md` §1）：handler 延迟执行、
  长 C 调用、主线程限制与 `alarm(0)` 取消语义。
- Rust `std::process::Child` 官方文档：离开作用域不会自动 kill 或 wait，正常与错误路径都要显式收口。
-/

/-! ## 6 命名统一

Mei 的状态只有 `request: serde_json::Value`。没有 `Conversation`、`ChatRequest` 或 canonical message。
`python` 是唯一本地客户端工具入口；`server_tool_use` 是 Provider 自己执行的透明块，不是第二个本地工具。
`pause_turn` 是 Provider 暂停后续跑，不是取消、恢复或持久化。
-/

/-! ## 7 模块边界

`agent` 是决策；`transport`、`python` 是适配器；`main` 是唯一装配点；`error` 是诊断值。
核心生产依赖只有 `serde_json`，不依赖 Sprawling 的 TUI、城市运行时、账本、gateway crate 或 `console_ffi`。
依赖方向只有现有 Sprawling TUI 在进程边界适配 Mei，Mei 不知道消费者的类型。

`ANTHROPIC_BASE_URL` 是客户端出站地址，不是 Mei 的监听地址。Mei 与 Sprawling TUI 都不得绑定
`2501` 或 `3425`；默认 3425 仅因 magpie 官方网关已在该地址监听，两个客户端可以安全地同时连接它。
-/

/-! ## 8 接口先行

```rust
// agent.rs
pub fn run(
    request: &mut Value,
    post: impl FnMut(&Value) -> Result<Value>,
    exec: impl FnMut(&str) -> Result<String>,
    trace: &mut dyn Write,
) -> Result<String>;

// transport.rs
impl Client {
    pub fn new(base: &str, key: String) -> Result<Client>;
    pub fn post(&self, request: &Value) -> Result<Value>;
}

// python.rs
impl Py {
    pub fn spawn() -> Result<Py>;
    pub fn run(&mut self, code: &str) -> Result<String>;
    pub fn close(&mut self) -> Result<()>;
    pub fn kill_and_wait(&mut self) -> Result<()>;
}
```

`Result` 的错误是 `error::Error`：`Diagnostic` 渲染一个 `toml` fenced block，
含稳定 `code`（`error::Code` 枚举，一种规则一个变体，穷尽匹配不带兜底）、`action`、`subject`、
`path`（适用时）、`value`（适用时）、`rule` 与 `recovery`；协议族错误的 path/value 指向失败字段
（如 `messages[1].content[3].id`）及其取值。`Body` 仅用于非 200，原文写 stderr，不额外包装。

启动请求包含 `model`、`max_tokens = 16000`、`system`、`tools = [python]` 与一个 user prompt。
`HARNESS_EXTRA` 是 object，启动时浅合并一次，然后重新写入 prompt messages；
`stream = true` 与用 `HARNESS_EXTRA.tools` 覆盖唯一客户端工具都拒绝，分别提示关闭 stream、改用 `MEI_TOOLS`。
`MEI_TOOLS` 是 array，按给定顺序追加在 python 后，Mei 不理解或修改这些声明。
`HARNESS_MODEL` 必填；`ANTHROPIC_BASE_URL` 默认 `http://127.0.0.1:3425`，只接受 `http://authority` 与可选末尾 `/`；
`ANTHROPIC_API_KEY` 默认 `magpie`；`HARNESS` 未预设才写为 `current_exe()`。

CLI 只有 `mei -p PROMPT` 与无 `-p` 时读取 stdin 全部内容。无 `-p` 且 stdin 是 TTY 时立即拒绝，
避免看似挂起。`-p` 缺值、其它参数、缺模型、非法 JSON 都在启动副作用前失败。
-/

/-! ## 9 工作流程

```text
A = post(R)
R.messages.push(assistant(A.content)); trace
if pause_turn: continue                         -- 不加 user
if end_turn: return 顺序连接的 text blocks
if stop != tool_use: fail                       -- max_tokens 也不执行
Y = 预检全部客户端调用，再按 content 顺序执行
R.messages.push(user(Y)); trace; continue
```

`server_tool_use` 与其结果块原样留在 assistant 历史，不产生本地 tool_result。
每轮至多一个模型请求在途、至多一段由主循环发起的 Python 代码执行；只有主循环写 messages。
没有 turn ceiling；模型不停止时由外部 Ctrl-C、kill 或进程管理结束。
-/

/-! ## 10 实现逻辑

- 请求 JSON 每次只序列化一次，`Content-Length` 按 UTF-8 字节数；发送 `Connection: close`，
  不发送 `Accept-Encoding`。响应读至 EOF，只解析状态行、CRLF header/body 分界与 JSON body。
  缺少分界时错误附前 200 字节；chunked 明确拒绝。传输对每个地址用 `CONNECT_TIMEOUT`（10 s）
  连接预算、对读写用 `IO_TIMEOUT`（600 s）宽松预算：非流式响应可能合法思考数分钟，而接受后
  停滞的网关必须返回 `E_MEI_TRANSPORT` 诊断且不重试，不能无限挂起脚本化调用。
- assistant content 作为 JSON 值追加。读时 string 视为单 text block，写回仍是原 string；
  其它未知块不建模。serde_json 默认数值语义是当前支持范围，不承诺任意高精度扩展数字的字节级保真。
- `boot.py` 用二进制私有 fd 信道，每帧是一行 UTF-8 JSON string。先解码并检查帧类型，
  再只捕获 `exec` 的 `BaseException`；非法帧退出解释器，使 Rust 运行失败。
- fd 0 改 `/dev/null`，每轮 fd 1/2 指向当前二进制临时文件；Python 标准流明确 UTF-8 + replacement。
- 输出经增量 UTF-8 decoder 分块读取，内存只保留解码块、头 8000 与尾 8000 字符。
  超过 16000 字符时保留头尾并留下完整文件路径，否则删除临时文件。空输出为 `(no output)`。
  快照经独立只读句柄按 `fstat` 固定上界读取：模型线程留在共享输出偏移上的后续追加只越过上界、
  不能覆盖开头或延长快照。
- `main` 成功时在 `CLOSE_GRACE`（5 s）内有界 wait 后关闭 stdin；模型留下的非 daemon 线程会阻止
  Python 退出，超时后 kill 并 wait——强制清理是正常收口，不是运行失败（模型已给出最终答案）。
  失败时 kill 并 wait；`Drop` 仅作最后兜底，不代替显式收口。宽限按 `CLOSE_POLL`（20 ms）轮询倍数计，
  不采样时钟。
-/

/-! ## 11 边界枚举

空 prompt、Unicode prompt、stdin EOF、缺环境变量、非法 URL、非 200、缺 status line、
缺 CRLF 分界、chunked、网关接受后停滞、connect 超预算、content 缺失或类型错误、空工具波、
未知客户端工具、空 ID、重复 ID、max_tokens、pause_turn、refusal、解释器死亡、非法控制帧、
SystemExit、非 UTF-8、空输出、16000/16001 字符边界、后台线程跨调用写入、后台写入与快照交错、
收口时存活的非 daemon 线程、递归超时与 kill 后 wait 都必须有明确行为或测试。
-/

/-! ## 12 错误处理

HTTP、Provider、上下文、协议与私有信道错误退出 1，不重试、不压缩、不恢复历史。
Python traceback 和 best-effort timeout 是普通文本 tool_result，不加 `is_error`。
工具波中死亡时，之前的副作用可能已经发生；不回滚，也不保证已有完整结果。
非 200 正文原样写 stderr；其他诊断给出可执行的恢复步骤，不泄露 API key。
-/

/-! ## 13 依赖选型

核心只继承 workspace 的 `serde_json`，复用已有 JSON Value、序列化与 `arbitrary_precision` 未开启时的明确语义；
不引入 HTTP、CLI、TOML、异步或错误框架。选择 std `TcpStream`、`Command` 与闭包，
是为了守住 P0 的最小控制面。被否：reqwest（HTTPS/代理/重定向超出边界）、clap（两个输入形式不需要）、
trait Model/Tool（生产各只有一个实现，脚本闭包已是第二真实实现）。
测试可用 workspace `tempfile`，不进入生产依赖。
-/

/-! ## 14 硬编码声明

`max_tokens = 16000`、超时 250 秒（`boot.py` 的 `alarm`）、输出 16000 字符且头尾各 8000、
空输出文本、连接预算 10 s 与读写预算 600 s（`transport.rs`）、收口宽限 5 s 与轮询 20 ms（`python.rs`）、
默认网关 `http://127.0.0.1:3425`、默认 key `magpie`、`anthropic-version: 2023-06-01`
各只有一处权威。`MEI.md` 是系统提示词唯一来源，修改它即可改变构建时的静态提示；
不再为二次开发复制一份 TOML 配置。服务端工具的结果绕过 Python 输出上限并逐轮重传，
调用者应自行在声明中限制 `max_uses`，Mei 不新增成本控制。
-/

/-! ## 15 影响面

新 crate 不影响现有 City 请求、账本或 provider。Sprawling TUI 只在 `console` 新增 Mei 后端，
复用 `Entry`、live state、editor、screen、单写者终端、回滚和 OSC 7501。
核心接口变化先改本文件与 `spec/Agent.lean`，再改实现和对应闭包/TCP/Python 测试。
-/

/-! ## 16 测试与约束

```bash
cargo nextest run -p sprawling-mei
cargo clippy -p sprawling-mei --all-targets --all-features -- -D warnings
lake build Mei.Spec Mei.spec.Agent
cargo test -p sprawling-mei --test cli
```

Lean 证明只覆盖反馈模型，不是 Rust refinement proof。Rust 闭包测试检查控制路径，
TCP/Python 测试检查真实适配器，CLI 测试检查 stdout/stderr 与退出码。POSIX-only 行为在 Unix lane 验证；
Windows 仅验证可编译和明确拒绝运行。live V1/V2 的实测范围以 §2 的记录为准。
-/

/-! ## 17 文档关系

`Mei/README.md` 是学术原型使用与二次开发入口；`architecture.toml` 注册模块；
`docs/ARCHITECTURE.md` §3 注册无产品 crate 依赖的拓扑；`docs/third-party.md` 跟踪协议事实。
`crates/sprawling/spec/Console.lean` §8-174 拥有 TUI 适配合同，本规格不复制它。
修改协议、固定值、进程边界或验收声明时，同时重审上述对应权威。
-/

/-! D1 请求 JSON 是唯一状态，闭包替代单实现 trait：避免第二份会话表示与假想 seam；
被否的是 canonical message 和 Model/Tool trait，以及为假想替代者预留的 seam。 -/
/-! D2 非流式 HTTP/1.0 只读至 EOF：以重复传输和等待完整响应换取最小协议面；
被否的是 reqwest/SSE/chunked/重试。 -/
/-! D3 大输出用临时文件和增量解码保持 O(1) 额外内存（相对输出总量）：
被否的是 `f.read()` 全量读入；磁盘总量仍不设上限。 -/
/-! D4 TUI 只在 Sprawling 一侧适配，核心不知道 Entry/RunId/TimeMs/Live：
被否的是为核心增加 TUI 事件类型。 -/
/-! D5 `MEI_TOOLS` 透明追加与 `pause_turn` 续跑只认识协议所需的最小子集：
被否的是 Provider 分支、服务端工具建模与截断其结果。 -/
/-! D6 `MEI.md` 在构建时注入，不新增动态配置体系：二次开发直接修改源码资源；
被否的是再引入 TOML 配置解析和第二份提示词。 -/
/-! D7 `close()` 用 5 s 有界等待代替无限 wait，超时 kill 并回收：模型留下的非 daemon 线程原会让
解释器永不退出、扣住最终答案；被否的是无限 wait（最终 stdout 被模型代码劫持）与把超时当运行失败
（答案已完整，升级清理不该吞掉它）。 -/
/-! D8 传输对每个地址设 10 s 连接预算、对读写设 600 s 宽松预算：非流式响应可能合法思考数分钟，
接受后停滞的网关必须返回 `E_MEI_TRANSPORT` 且不重试；被否的是不设时限（脚本化调用无限挂起）与
短时限（截断合法的长思考响应）。 -/
