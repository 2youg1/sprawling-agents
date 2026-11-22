# Archetype:Mei

[English](README.md) · [简体中文](README.zh-CN.md)

Archetype:Mei 是极简 Agent Harness：模型接收任务，通过唯一持久的 **python** 工具行动，看到每个小结果，再循环到最终答复。大输出或非 UTF-8 输出完整落盘，只返回提示和完整路径，或能无损还原非 Unicode 原生路径的 Python 表达式。本包是**无头**核心，Mei 本身不含终端壳。

交互面是 [Calyx](../Calyx/)。`@sprawling/mei` 同时携带无头 `mei` 与 `calyx`；TTY 且无 `-p` 时进入 Calyx，`-p` 或管道仍走无头路径。脚本只使用无头路径。`mei --acp` 以 [Agent Client Protocol](https://agentclientprotocol.com/) version 1 的 agent 角色在 stdio 上应答，城 harness 或其他 ACP client 可像调用其它 ACP agent 一样调用 Mei；见 [integrations](../docs/integrations.md)。

> **安全优先：** Mei 没有沙箱、权限提示或撤销。生成的 Python 可以读写文件、访问网络，并以当前账户权限运行命令。只用于一次性项目副本和单独、可撤销且有限额的 API key。不要提供私密数据、重要凭证或项目唯一副本。

## 安装

| 方式 | 命令 | 得到什么 |
|---|---|---|
| Bun | `bun install --global @sprawling/mei@latest` | npm 包同时携带无头 `mei` 核与 Calyx 面；TTY 下 `mei` 直接进入 Calyx，`mei -p` 走无头。`bunx @sprawling/mei` 可不安装直接运行一次。 |
| crates.io | `cargo install sprawling-mei --locked` | 只装无头 `mei` 核。Calyx 面不是 crate 二进制：它随 Bun 包或源码构建而来。 |

已发布包会落后于本树，直到下一次发布；要本仓库的版本，用下方源码构建。

## 从源码构建

### 1. 启动 Magpie

安装并打开 [Magpie](https://usemagpie.ai)，添加服务商并配置需要的模型路由，保持它监听 `127.0.0.1:3425`。Mei 会通过网关发现首选路由、上下文窗口、思考级别、实际模型和实际思考强度。无需复制服务商 key，也无需设置 `HARNESS_MODEL`。

### 2. 克隆并构建

安装 Rust 工具链，以及 [`Calyx/ffi/zig-version`](../Calyx/ffi/zig-version) 指定的 Zig 版本。

```sh
git clone https://github.com/2youg1/sprawling-agents.git
cd sprawling-agents
cargo build --locked -p sprawling-mei -p sprawling-calyx
```

两个程序是 `target/debug/mei` 和 `target/debug/calyx`（Windows 带 `.exe`）。Calyx 会自动找到同目录的 Mei 核。

### 3. 打开工作区

Bash：

```sh
export PATH="$PWD/target/debug:$PATH"
mkdir -p /tmp/mei-playground
cd /tmp/mei-playground
calyx
```

Windows PowerShell：

```powershell
$env:PATH = "$PWD\target\debug;$env:PATH"
New-Item -ItemType Directory -Force "$HOME/mei-playground" | Out-Null
Set-Location "$HOME/mei-playground"
calyx
```

模型、网关和其他参数都不用输入。TTY 会进入 Calyx；`-p` 和管道仍走无头路径。

## 一次一步使用

### 1. 把首个任务写进 `AGENTS.md`

例如，把 `first.png` 放进工作区，然后写：

```markdown
# 图片任务

当用户输入“开始”时，只调用一次 `python`，执行
`show_image('first.png')`。然后描述图片的背景、形状、颜色和准确文字。
不要使用 OCR、像素统计或另一个模型。
```

Mei 会把工作区顶层普通文件 `AGENTS.md` 注入新会话的首条消息。此时只需输入：

```text
开始
```

这一次 Python 调用同时返回图片和打印文本。支持 PNG、JPEG、GIF、WebP，单图 base64 编码前至多 5 MiB，一次调用最多两张，且只在该调用自己的线程上附加；第三张、别的线程的或调用结束后的提交会被拒绝，原因写进工具结果。模型收到的是真正的视觉输入，不是路径或普通文本形式的 base64。

### 2. 继续或切换会话

| 按键 | 效果 |
|---|---|
| 输入文本后 Enter | 运行一个任务 |
| Shift+Enter | 插入换行 |
| `/new`、`/n` | 新建 JSONL 会话，旧文件保留 |
| `/resume`、`/r` | 恢复刚离开的非空会话 |
| `/resume 路径.jsonl`、`/r 路径.jsonl` | 恢复指定的完整日志 |
| `/e`、`/effect` | 按 Magpie 提供的级别切换下一次任务的思考强度 |
| Esc | 清草稿，或停止当前任务及其进程树；绝不退出 Calyx |
| `/quit`、`/q` | 关闭当前 Calyx 及其进程树，其他会话互不影响 |

模型选择继续交给 Magpie 路由，不另设 `/model` 命令。

### 3. 查看状态栏

composer 示例：

```text
actual-model · effort high · Agent Step 2 · 0.05% · 487/1048576
```

Magpie 尚未确认路由时，模型和思考强度标记为 `requested`。`Agent Step` 统计本次任务的模型响应数。上下文用量是 `input_tokens + cache_creation_input_tokens + cache_read_input_tokens`，相对模型最大上下文显示，比例固定两位小数；它不是累计账单。

### 4. 进行中追加（Steer）与关闭

任务仍在跑时再提交普通文本，不会打断当前工具调用：文字写入该次 run 的 `MEI_STEER_PATH`，Mei 在本波工具全部返回后接到最新一条 tool_result 文本末尾（前缀 `user: `）。Esc 仍只停止该任务及其子孙；`/q` 关闭当前 Calyx。空闲时按 Esc 不做任何事。

## 工作区、运行环境与捕获

新会话启动时，Mei 只读取工作区根目录的普通 `AGENTS.md`，不跟随符号链接，也不打开非普通文件，并把它放在首个 user 消息中真正 prompt 之前。日志冻结这个开头。后续任务和 `/resume` 不重读也不补注 AGENTS，即使文件新出现、修改或删除；只有 `/new` 才重新读取。Skills、MCP 配置与环境工具不自动暴露。

每个 Mei 版本首次启动时，doctor 探测 Python 和 uv 并缓存完整环境。当前版本的有效缓存会跳过所有探测和网络访问。受支持的 Python 直接复用；缺失、损坏或停止维护的 Python 会替换为最新稳定托管安装。uv 缺失或损坏时才安装。修复 Python 前会先更新 Mei 私有 uv。

安装只进入每用户 doctor 目录，不替换系统 Python、不修改全局 PATH、不改 shell profile、不写 Windows Python 注册表。一次完整 uv+Python 修复有 900 秒期限。doctor 不更新 Mei 本身。`MEI_DOCTOR_DIR` 可为隔离和测试覆盖该目录。

每次运行都有独立捕获目录。小型 UTF-8 捕获内联并回收；大输出和非 UTF-8 输出落盘，给出完整路径或无损原生路径表达式。交互会话的 spilled 路径位于 `<session>.captures/<run>`，可跨 resume 使用。失败或取消的交互运行只删除 Calyx 为该运行独占创建的目录。宿主强制崩溃可能留下 orphan 目录，不承诺崩溃清理器。

## 进程接口

| 通道 | 合同 |
|---|---|
| cwd | 调用者工作目录就是工作区 |
| `-p` / stdin | prompt；无 `-p` 时，stdin 必须是非终端的完整 prompt |
| `--replay`（Calyx） | 可选交互回放 JSONL |
| `--resume <jsonl>` | 可选完整消息日志 |
| stdout | 最终答复，不额外换行 |
| stderr | 消息 JSONL；`mei.obs` model/route/pulse/usage；失败为 Description/Cause/Resolution |
| cancel | 杀掉进程组 |
| tool_result | 文本与可选图片块；文本超过 16 KiB 或非 UTF-8 时改为提示和完整路径 |

Resume 保留旧工具名、字符串/数组内容以及图片块，不恢复 Python 现场。Provider 与模型必须接受这些媒体内容。

| 变量 | 含义 |
|---|---|
| `HARNESS_MODEL` | 可选的显式 Magpie 路由；缺省时选择目录首选路由 |
| `ANTHROPIC_BASE_URL` | 默认 `http://127.0.0.1:3425`；Mei 不监听 |
| `ANTHROPIC_API_KEY` | 默认 `magpie` |
| `HARNESS_EXTRA` | 最后浅合并的一个 JSON 对象；默认请求 adaptive thinking 和 `high` effort；拒绝 `stream` / `tools` |
| `MEI_BIN` | Calyx 启动核心的可选路径；同目录和 PATH 解析无需设置 |
| `MEI_DOCTOR_DIR` | 可选的隔离 doctor 缓存与私有运行目录 |
| `MEI_CAPTURE_DIR` | Face 提供的内部单次运行捕获目录 |
| `MEI_CAPTURE_CREATED` | 证明 Calyx 创建了空捕获目录的内部 `1` 标记 |
| `MEI_STEER_PATH` | Face 提供的 Steer JSONL（每行 `{"text":"…"}`）；缺席则无追加 |
| `SPRAWLING_PROGRAM_STATUS` | `never` 禁用 Calyx 的 OSC 7501 |

Calyx 通过 Program Status Protocol（OSC 7501）报告 `working`、`done`、`error`，以及取消后的 `idle`。不支持的终端会忽略它。无头 `-p` 和管道运行不发送 OSC 7501。

## 内存

在 Windows 11 x86-64（16 核）上的一次读数，使用本树 release 二进制，按各进程的 Windows 工作集统计。会话含 6.8 MiB 消息文本：Mei 存成 6.8 MiB JSONL，Pi 1.5.2 从同一文本的 7.1 MiB 会话加载。Pi 使用 `--offline` 及其默认上下文文件、Skills 与扩展。两侧都没有联系模型服务；Mei 的一次请求发往本地替身。Calyx 在 `/resume` 后 30 秒读数；Mei 取一次携带该历史的请求期间的峰值。

| 进程 | 空会话 | 6.8 MiB 会话 |
|---|---:|---:|
| Calyx（空闲） | 8.3 MiB | 33.8 MiB |
| Mei 核（峰值） | 5.8 MiB | 22.1 MiB |
| python 子进程（峰值） | 13.5 MiB | 13.5 MiB |
| **Mei + Calyx + python** | **27.6 MiB** | **69.4 MiB** |
| Pi TUI（空闲） | 120.8 MiB | 144.1 MiB |

核与 python 只在任务运行时存在；空闲时只有 Calyx 占内存。大会话给 Calyx 增加约 26 MiB，给核增加约 16 MiB。这是单机工作集读数，不是跨平台基准；计数器、会话或平台不同，数字就不同。

## Prompt 与工具合同

`Mei/src/MEI.md` 在构建时进入 `system`；唯一的 python 定义位于 `tools[0]`。下表使用 Python `3.12.13`、uv `0.12.3` 的固定文档夹具；实际版本属性来自 doctor。字节数不含消息信封和 provider 自行添加的内容。

| 部分 | UTF-8 字节 | Token（`o200k_base`） |
|---|---:|---:|
| `system` | 297 | 57 |
| 完整紧凑 `tools[0]` JSON | 1489 | 332 |
| 合计 | 1786 | 389 |

Token 列用 OpenAI 的 `o200k_base` 分词器（GPT-4o/4.1/5 家族）对同一文本计数，不含聊天信封或 provider 追加内容；换成其他分词器数值会不同。以下 JSON 为便于阅读而格式化；字节数和 Token 数按紧凑线上格式计算。

### `system`

```text
<agent-harness>
You are operating inside Mei.
No additional documents or environment capabilities are injected or exposed; fetch what you need.
Workflow: Sense → Act → Verify → Adapt…
Aligned with context, fulfill user intent thoroughly and resourcefully and stay scoped.
</agent-harness>
```

### `tools[0]`

```json
{
  "description": "<python version=\"3.12.13\" uv_version=\"0.12.3\">\nA persistent python interpreter. Variables, imports, and the working directory (cwd) persist across calls. An exception prints its traceback without ending the session, so you can recover and continue without rebuilding state. Expressions do not auto-echo, so explicitly print any values you need to inspect. If combined UTF-8 stdout/stderr is up to 16 KiB, the tool returns it inline. Larger or non-UTF-8 output returns a notice and the full path, or a Python expression that losslessly reconstructs a non-Unicode native path; filter that file, or re-run with narrower output, in the next python call. Run shell commands and CLIs through subprocess, capture their output, and check return codes. Start child interpreters with sys.executable, so they inherit the active environment. Call show_image(path) to attach a PNG, JPEG, GIF, or WebP picture as visual tool-result content; a picture is not text output. Attach at most 2 pictures per call and at most 5 MiB per picture, on the call's own thread; a picture from another thread or a later call is refused with how to read it. Because capabilities like web_search, web_fetch or sub-agent delegation are not exposed as built-in tools, python serves as your interface to invoke external APIs, CLIs, headless tools, and more directly via subprocess.\n</python>",
  "input_schema": {
    "properties": {
      "code": {
        "type": "string"
      }
    },
    "required": [
      "code"
    ],
    "type": "object"
  },
  "name": "python"
}
```

Mei 只保留一个 provider-neutral schema，并在 agent loop 中校验每一波工具调用；不声明 provider 专有的 `strict` 模式。

## 边界

核内无 python 超时；连接 10s / IO 600s；空输出为空文本；超过 16 KiB 或非 UTF-8 只给提示与完整路径或无损路径表达式，从不返回头尾样本。无 `MEI_TOOLS`、Skills/MCP 自动暴露、Mei 自更新、核内 TUI、OCR 兜底或第二图片工具。只有新会话顶层的普通 AGENTS 会注入。

## 规格与检查

[`Spec.lean`](Spec.lean)、[`spec/Agent.lean`](spec/Agent.lean)、[`spec/Session.lean`](spec/Session.lean)、[`spec/Environment.lean`](spec/Environment.lean)、[`spec/Capture.lean`](spec/Capture.lean)、[`spec/Gateway.lean`](spec/Gateway.lean)；终端规格见 [`Calyx/Spec.lean`](../Calyx/Spec.lean)。

离线检查用真实 `mei`、Python 和本地网关，不调用模型 API：

```sh
cargo nextest run -p sprawling-mei
cd Calyx && zig build test
cd .. && lake build Mei.Spec Mei.spec.Agent Mei.spec.Session Mei.spec.Environment Mei.spec.Capture Mei.spec.Gateway
cargo clippy -p sprawling-mei -p sprawling-calyx --all-targets -- -D warnings
```

真实 Magpie 运行需要主动执行并消耗模型 token。执行 `AGENTS.md` 图片任务，再用 `/new` 和 `/resume` 展示第二张图片，并在两个独立 Calyx 窗口检查 Esc 和 `/q`。若某步失败，先保留命令、会话 JSONL 和网关响应，再修改产品。
