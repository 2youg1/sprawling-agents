-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import Mei.spec.Agent
import Mei.spec.Capture
import Mei.spec.Environment
import Mei.spec.Session

/-! # Mei 的规格

`Mei/` 是极简 Agent Harness：一个可变 Anthropic Messages 请求、一个串行反馈循环、
一个有状态 python 进程。Cargo 包名 `sprawling-mei`，库名 `mei`，无头命令名 `mei`。
唯一工具名为 `python`；小型 UTF-8 输出直接返回，大输出只返提示与完整路径，非 Unicode 本机路径则用可无损还原的 Python 表达式表示。
终端居住面属于顶层生产包 `Calyx/`（非 Archetype）；Mei 核不含 TUI。
指令 / MCP / Skills 的权威在 magpie；Agent 自去 skill 仓库。`MEI.md` 仅在构建时进入 system；
新会话只额外读取工作区顶层的 `AGENTS.md`，并将其冻结在首个 user 消息。
-/

/-! ## 1 需求分解

| 单元 | 责任 |
|---|---|
| `main` | 装配环境、参数、stdin/stdout、退出与解释器收口 |
| `agent` | 唯一反馈循环；只读 JSON 与 `std::io::Write` |
| `transport` | 非流式 Anthropic Messages HTTP/1.0；阻塞读时可写存活观测 |
| `python` 与 `boot.py` | 持久 python；完整输出落盘；小输出内联，大输出只给提示与路径 |
| `workspace` | 新会话顶层 AGENTS 的读取与首消息构造 |
| `doctor` | 当前 Mei 代次的 python/uv 缓存、探测与必要安装 |
| `capture` | inline/spilled 捕获的文件所有权与回收 |
| `error` | 人可见诊断：Description / Cause / Resolution |
| `MEI.md` | 构建时静态系统提示；改之须重建；不作运行时注入面 |
| `spec/Agent.lean` | 反馈顺序与整波预检的形式模型 |
| `spec/Session.lean` | 完整日志恢复的形式模型 |
| `spec/Environment.lean` | doctor 代次与修复判定 |
| `spec/Capture.lean` | 捕获所有权与退出后的文件有效性 |

禁止：第二本地工具、`MEI_TOOLS`、阈值截断/头尾窗、内置超时 alarm、核内 TUI、
LLM compaction、验真门、Skills/MCP 自动暴露、Mei 自动更新、为弱模型准备的重试。
交互斜杠与会话文件由 Calyx 拥有；无头 `mei` 仍可纯脚本使用。
-/

/-! ## 2 验收标准

- `spec/Agent.lean` 与 `agent::run` 闭包测试：整波预检、顺序执行、thinking 保真、
  pause、max_tokens、异常 stop、执行失败不伪造 tool_result。
- TCP 工装：HTTP/1.0、EOF、非 200 原文、chunked 拒绝、停滞返回 `E_MEI_TRANSPORT`。
- 真实 python：变量、imports 与 cwd 跨调用保留；异常打印 traceback 后不结束会话，
  下一调用可继续使用既有状态；表达式不自动回显；每次 `run` 的 tool_result 是完整的小型 UTF-8 捕获；
  文件字节等于该次捕获的 stdout+stderr；空输出返回空字符串；
  超过 16 KiB 或非 UTF-8 时只返回一条提示与完整文件路径，不内联头尾片段；
  下一次 python 调用可按路径读取、筛选或切片后打印；`sys.executable` 可启动并捕获子 python；
  非法帧与解释器死亡为核错误；Windows 与 Unix 均可 spawn。
- AGENTS 只在新会话首轮注入一次；只接受打开后确认的普通文件，拒绝符号链接、目录、FIFO 与其他文件类型；同会话后续轮次与 resume 不重读、不补注；`/new` 才可读取新代次。
- doctor 对当前 Mei 版本的完整缓存不探测、不联网；缺席、损坏、版本更新、或记录的程序已不是普通文件
  才重建；受支持 python 不追新，缺失/EOL 才安装最新稳定版；uv 缺失或不可执行才安装，PATH 无 uv 时
  复用私有目录里可用的 uv。精确版本只进入 `<python version=… uv_version=…>`，
  路径仅供本地执行，不进入模型描述，python 子进程可通过 PATH 调用 uv。
- inline capture 在 fd 转移后或解释器 reap 后回收；spilled 路径跨解释器退出与 resume 有效；
  成功回收解释器后的 inline 清理失败单独报警，不吞掉最终答复或 spilled 交接；失败清理保留路径供重试。
  Calyx 只删除本 run 独占创建且未提交的 capture 目录。宿主崩溃后的 orphan 回收不在本次承诺内。
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

- User ruling：极简核；小输出直接观察，大输出只给提示与路径；Calyx 为生产终端壳；
  仅新会话顶层 AGENTS 可运行时注入，Skills/MCP/环境工具不自动暴露；诊断 Description/Cause/Resolution；
  Mei 不含壳（核进程不内嵌 TUI）；doctor 不更新 Mei；capture 按 run/session/caller 分配所有权；
  npm 以机制装配交互面。
- Anthropic Messages / client tools；[strict tool use](https://platform.claude.com/docs/zh-CN/agents-and-tools/tool-use/strict-tool-use)；
  magpie reference（`docs/third-party.md` §1）。
- Rust `std::process::Child`：须显式收口。
- `Calyx/Spec.lean`：终端与斜杠；本文件不复制。
-/

/-! ## 6 命名统一

状态只有 `request: serde_json::Value`。`python` 是唯一本地客户端工具。
tool_result 的 `content` 是小型完整输出，或“提示 + 完整结果路径”；不含 BLAKE3。
`pause_turn` 不是取消或持久化。观测行 `type=mei.obs` 不是 Messages role。
`python` 是有状态执行器而非 REPL：表达式不自动回显，`print` 是主动观察。
`inline` / `spilled` 描述输出是否只内联或把路径交给模型；`run` / `session` / `caller` 是文件 owner。
-/

/-! ## 7 模块边界

`agent` 决策；`transport` / `python` 适配；`resume` 读完整 JSONL；`workspace` 读取顶层 AGENTS；
`doctor` 管理运行环境；`capture` 管理输出文件；`main` 装配；`error` 诊断。
生产依赖只有 `serde_json`，安装经操作系统已有的 shell、curl/wget 或 PowerShell 与官方 uv installer。
Calyx（及任何 Face 消费者）在进程边界 spawn 无头 `mei`；Mei 不知道 Entry/Zig。

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
    pub fn spawn_in(environment: &Environment) -> Result<Py>;
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

`doctor::ensure(now)` 接收装配采样时间；`Environment` 同时持有两个程序的精确版本与绝对路径，
`write_cache` 写入当前 Mei 代次的完整记录。`CaptureStore::create/observe/complete/discard` 分别对应
run 创建、结果分类、成功收口与未提交回收。

启动请求：`model`、`max_tokens`（默认见 §14）、`system`、`tools = [python]`、messages。
新会话且顶层 AGENTS 存在时，首个 user content 依次是指令文本块与 prompt 文本块；否则仍为字符串。
resume 只重放日志已有消息，不读取或补注当前 AGENTS。工具描述以 `<python version="…" uv_version="…">`
携带精确版本，与持久变量/imports/cwd、异常后恢复、主动打印、16 KiB 内联预算、完整捕获路径、
subprocess 和 `sys.executable` 一致；内联数字由 `INLINE_BYTES` 生成；不写 REPL 或核内超时。
`MEI_CAPTURE_DIR` 是 Face 分配当前 run 目录的内部环境接口；缺席时无头核创建临时 run 目录。
Calyx 先独占创建空目录，再以 `MEI_CAPTURE_CREATED=1` 让 Mei 接管；没有这个标记时既有目录一律拒绝。
私有控制信道只传 ASCII capture 文件名，`Store` 在本机 root 下拼接；spilled 提示给完整 Unicode 路径，
或给能在 Python 中无损还原非 Unicode 本机路径的表达式。
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
- `boot.py`：每帧一行 JSON string；stdout/stderr 重定向到当前 run 的捕获文件；同一执行命名空间保留
  变量与 imports，解释器进程保留 cwd；异常只形成 traceback 反馈，循环继续；
  结束后只把**该文件的 ASCII basename** 经控制信道回传；不改写文件、无 alarm。
  `MEI_CAPTURE_DIR` 与 `MEI_CAPTURE_CREATED` 在模型代码运行前由 `boot.py` 移出环境，递归子 Mei 不会误继承父 run 目录。
- `Py::run` 先读文件元数据；仅不超过 16 KiB 时做有界读取。完整 UTF-8 文本直接作为
  tool_result；更大或非 UTF-8 时只给提示与路径。下一响应已把 fd 1/2 移到新文件，
  此时回收此前的 inline 文件；当前 inline 文件在解释器收口后回收。
- 空输出：仍创建零字节文件，tool_result 为空字符串，文件按 inline 回收。
- 成功 `close` 先完成 spilled 交接，再回收 run 所有的 inline；inline 删除失败只报警并保留清理记录，答复仍成功。
  失败 `kill_and_wait` 回收整个未提交 run。`Py` 被丢弃时，子解释器已退出且未 `close` 的 run 视为未交接，回收其未提交 run。
  Calyx 在强制取消后只删除它独占创建的 run 目录。`close` 的 5s 宽限后 kill+wait；成功收口的强制清理不是运行失败。
- doctor 以 `CARGO_PKG_VERSION` 和缓存 schema 判定代次；缓存命中不执行探测或安装，但记录里的两个程序
  必须仍是普通文件，否则整条记录作废并重建。
  缺失/EOL python 的修复先刷新 Mei 私有的最新稳定 uv，再在隔离目录安装最新稳定 python，
  不读取工作区 `.python-version`，不改系统 python、全局 PATH 或 shell profile。
  首启互斥用操作系统的文件咨询锁，锁随持有进程退出由系统释放，遗留锁文件不会阻塞下一次启动；
  等待按单调截止时间轮询，最后一次休眠后仍尝试取锁；等待上限覆盖完整修复截止时间与缓存发布余量，取得锁后重读缓存。
  完整 uv+Python 修复共享一个 900s 单调截止时间，到期 kill+wait 且不发布缓存；锁等待另加 5s 发布余量。
  uv 子进程移除 `UV_PYTHON`，find 使用 `--no-project`，Windows install 使用 `--no-registry`；损坏安装以 `--reinstall` 重试一次。
  修复结果写缓存前按同一个 `now` 再验证 Python 支持期，过期结果不发布。PATH 中不可执行候选跳过并可回退私有 uv，
  成功探测使用同一绝对候选路径。`MEI_DOCTOR_DIR` 在切换 uv cwd 前转绝对路径；空或相对 XDG 缓存路径回退 HOME。
  探测脚本以 `-I` 运行：工作区文件与 `PYTHON*` 变量都不影响结果，只有 `NotFound` 视为"该程序缺席"，
  其他 I/O 失败按错误上报。缓存写入与锁错误均产生带路径、值、可执行修复命令的诊断；失败不写成功缓存。
-/

/-! ## 11 边界枚举

空/Unicode prompt、stdin EOF、重复参数、恢复残缺、缺模型、非法 URL、非 200、
chunked、停滞、空工具波、未知客户端工具、重复 id、max_tokens、pause、
AGENTS 缺失/修改/删除/非 UTF-8、重复 resume、无 AGENTS 旧日志、失败首轮重试、
doctor 缓存命中/损坏/更新/记录程序缺失或非普通文件/遗留锁文件/并发首启/安装失败、
EOL 日期边界（与 `Mei/spec/Environment.lean` 的向量一致）、非 Unicode 可执行路径、
inline 文件占用、spilled 跨进程读取、失败/取消 run、宿主崩溃 orphan、
解释器死亡、非法帧、非 UTF-8、空文件路径、收口非 daemon 线程、取消杀组。
-/

/-! ## 12 错误处理

核错误退出 1；不重试。Python traceback 在结果文件内，小型 traceback 直接进 tool_result。
非 200 正文原样 stderr。诊断不泄露 API key。
-/

/-! ## 13 依赖选型

仅 `serde_json`；std TcpStream/Command/闭包。被否：reqwest、clap、Model/Tool trait、
Mei 自更新器、为输出捕获引入异步 runtime。uv 官方 installer、`uv python install` 与
`UV_UNMANAGED_INSTALL` / `UV_PYTHON_INSTALL_DIR` / `UV_PYTHON_BIN_DIR` 的合同见官方文档，
跟踪关系记入 `docs/third-party.md`。
-/

/-! ## 14 硬编码声明

`max_tokens` 默认 `65536`（可被 `HARNESS_EXTRA` 覆盖）、连接 10s、IO 600s、
收口宽限 5s、poll 20ms、输出内联上限 16 KiB、默认网关与 key、
`anthropic-version: 2023-06-01`、pulse 间隔 5s、完整 doctor 修复预算 900s（锁等待 905s，含 5s 发布余量）、
各一处权威。`MEI.md` 构建时注入。
`INLINE_BYTES` 同时驱动内联判定、超限提示和工具描述的数字，没有第二份阈值或换算目标。
16 KiB 覆盖本仓绝大多数源码/文档，避免普通文件多读一轮；无头尾采样、无 250s alarm。
Python EOL 日期表在 `Mei/src/doctor/support.rs` 与 `Mei/spec/Environment.lean` 各一份，二者由
`doctor_tests::the_lean_environment_vectors_replay` 对齐：Lean 将全部已记录 minor 的边界向量与独立提交的
`doctor_vectors.txt` 自动比对，Rust 逐条重放同一文件，任一侧单独漂移都红。日期取自 `python/peps` 仓库 `release_management/` 目录下的 `python-releases.toml`，
`docs/third-party.md` §1 跟踪其提交。当前时间作参数；doctor 目录覆写只用于隔离状态与测试。
-/

/-! ## 15 影响面

Calyx 依赖进程 Face；City 账本/wire 不变。接口变化先改本文件与 Agent/Session，
再改实现与测试。
-/

/-! ## 16 测试与约束

```bash
cargo nextest run -p sprawling-mei
cargo clippy -p sprawling-mei --all-targets --all-features -- -D warnings
lake build Mei.Spec Mei.spec.Agent Mei.spec.Session Mei.spec.Environment Mei.spec.Capture
cargo test -p sprawling-mei --test cli
cargo test -p sprawling-mei --test observation
```

Windows 与 Unix 都实际执行 beginner E2E；网关只有在下一请求确实携带小输出时才回答正确结果。
大输出 E2E 钉住不内联、下一次 python 按路径筛选和 resume 后读取；测试失败不得退化为 0 tests。
AGENTS 生产门测试统计新会话、后续轮次、resume、无 AGENTS 旧日志与失败重试的精确消息形状。
doctor 测试用假探测/安装闭包和隔离缓存，不执行真实下载；capture 用 N/2N 精确文件计数，
不据此声称墙钟提速。live V1/V2 以 §2 为准。
-/

/-! ## 17 文档关系

`Mei/README*.md`；`Calyx/Spec.lean`；`architecture.toml`；`docs/ARCHITECTURE.md`；
`docs/third-party.md`。斜杠与 TUI 细节以 Calyx 为准。
-/

/-! D1 请求 JSON 唯一状态；闭包替代单实现 trait。 -/
/-! D2 非流式 HTTP/1.0 读至 EOF；被否 reqwest/SSE/chunked/重试。 -/
/-! D3 完整输出永远落盘；不超过 16 KiB 的 UTF-8 输出直接作为 tool_result，更大或非 UTF-8 输出只给提示与路径。理由：模型必须看见主动打印的小结果，而巨大输出会成为上下文 DoS；下一次仍用 python 读取、筛选或切片。被否：纯路径（没有观察通道）、全量内联（上下文 DoS）、头尾截断（伪造完整观察）、第二读取工具（小输出多一轮）。不加 BLAKE3：当前没有校验、缓存或内容寻址需求。 -/
/-! D4 核不知 TUI；终端属 Calyx；被否核内 Entry 类型。 -/
/-! D5 删除 `MEI_TOOLS`；透明服务端工具不由 Mei 声明；`pause_turn` 续跑。 -/
/-! D6 `MEI.md` 仅构建时注入；新会话唯一运行时文档例外是顶层 AGENTS。它冻结于首个 user 消息，
后续 resume 不重读、不补注；只读取打开后确认的普通文件，Unix 非阻塞打开防止 FIFO 卡住启动，发布目标禁止跟随链接/reparse point，避免把工作区外内容发给 provider。被否：独立 AGENTS 消息（现有日志只记录
本次 prompt，会丢失它）、拼入 system（顺序与会话冻结均不符）、每轮刷新（重复且漂移）、跟随符号链接读取外部指南。 -/
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
/-! D15 工具名统一为小写 `python`，理由只是协议、描述与终端显示的一致性；不声称成功率提升。
旧日志中的 `Python3` / `python` 均原样保留，只有新工具波必须用小写名。 -/
/-! D16 doctor 是当前 Mei 代次的环境修复器，不是 Mei 更新器。完整缓存短路；更新后重建整条记录。
绝对路径供执行而非 prompt；版本进入 python XML 属性。被否：每次启动探测、已有受支持环境追新、
修改全局 PATH/shell profile、自动更新 Mei。 -/
/-! D17 inline 文件归 run，spilled 文件归 session/caller；path 是引用而非所有权转移。
被否：Python 退出时删除全部 capture（resume 路径失效）、永久保留 inline（无引用文件随调用数增长）、
以管道 lazy-spill 替换当前文件捕获（后台子孙的 drain 合同超出本次改动）。 -/
