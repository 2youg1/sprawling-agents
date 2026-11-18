# Archetype:Mei

[English](README.md) · [简体中文](README.zh-CN.md)

Archetype:Mei 是极简 Agent Harness：把任务交给模型，模型通过一个有状态的 **python** 工具行动并看到小结果，再继续直到最终答案。大型或非 UTF-8 结果完整留在磁盘，只返回提示与路径。本包是**无头核**。Mei 不含终端壳。

交互面是 [Calyx](../Calyx/)。`@sprawling/mei` 同时发布无头 `mei` 与 `calyx` 面：TTY 且无 `-p` 时走 Calyx，`-p` 或管道仍走无头核。脚本只用无头路径。

> **先读安全提示：** Mei 没有沙箱、权限确认或撤销。模型生成的 Python 能以当前账户权限读写文件、联网和执行命令。只把可丢失的项目副本交给它；使用单独、可撤销且设好消费上限的 API key。不要提供真实隐私、重要凭据或唯一一份项目。

## 安装与运行

| 目标 | 命令 |
|---|---|
| 脚本 / 单次 | `bunx @sprawling/mei -p '…'`，或 `cargo install sprawling-mei --locked` 后 `mei -p '…'` |
| 交互（`/new`、`/resume`、Esc） | 在工作区执行 `bunx @sprawling/mei`；或 `cargo install sprawling-mei sprawling-calyx --locked` 后执行 `calyx`（同目录 `mei` 或 `MEI_BIN`） |

`@sprawling/mei` 发布无头核与 Calyx 面（Windows x64、macOS arm64、Linux x64 musl）。Mei 永不进程内嵌壳；Calyx 同时服务 `sprawling` 里的城控制台。

## 黑盒路径（只用键盘）

需要能联网、[Bun](https://bun.sh)、Magpie、支持工具调用的模型 API key，以及可弄坏的目录。机器可以已有 Python 3 和 uv；Mei 的 doctor 会在需要时修复。不必克隆本仓库。

### A. 脚本冒烟

1. 检查 `bun --version`。
2. 安装并打开 [Magpie](https://usemagpie.ai)，在 **Providers → Add provider** 添加服务商；保持应用打开，它已提供 gateway。纯终端可改用 `magpie serve`。
3. 从 `magpie models` 选择模型，再打开可弄坏的目录。

Bash：

```sh
export HARNESS_MODEL='deepseek/deepseek-flash'
mkdir -p ~/mei-playground && cd ~/mei-playground
```

Windows PowerShell：

```powershell
$env:HARNESS_MODEL='deepseek/deepseek-flash'
New-Item -ItemType Directory -Force "$HOME/mei-playground" | Out-Null
Set-Location "$HOME/mei-playground"
```

运行：

```sh
bunx @sprawling/mei -p '请调用 python 计算 40 + 2，只回答数字。'
```

stdout 为 `42`。失败时是带 `description` / `cause` / `resolution` 的 `toml` 块。

### B. 第一个工作区（交互）

1. 按 A 配置 magpie，设置 `HARNESS_MODEL`。
2. 把可弄坏的项目副本放进新目录。
3. 进入该目录：

```sh
bunx @sprawling/mei
```

| 按键 | 作用 |
|---|---|
| 文字后 Enter | 跑任务；首次成功在 cwd 建 `mei-….jsonl` |
| `/new` | 新 JSONL；旧文件保留 |
| `/resume <路径.jsonl>` | 继续该完整日志 |
| Shift+Enter | 换行 |
| Esc | 清空草稿 → 停任务 → 退出 |

横幅显示 cwd；composer 显示 `HARNESS_MODEL · usage`（尚无 usage 时为 `unknown`）。有任务在跑时显示 Mei working；pulse 行不刷屏。

Calyx 面通过 Program Status Protocol（OSC 7501）报告 `working`、`done`、`error`，或取消后的 `idle`。理解该协议的终端或 agent dashboard 因而无需读取对话，就能观察被另一个 agent 召唤的 Mei 子进程；报告只带计数，不带 prompt 文本。Mei 不发送支持探测，不识别该序列的终端会忽略它，Calyx 在交还终端前清除状态。设置 `SPRAWLING_PROGRAM_STATUS=never` 可关闭报告。无头 `-p` 与管道运行不发送 OSC 7501。

首句：

```text
先阅读项目说明和目录，不要修改任何文件。告诉我怎样构建和测试，并建议一个范围最小的首个任务。
```

用 `git status` 和项目自己的测试验收。

不经 npm 时：`cargo install sprawling-mei sprawling-calyx --locked`，再执行 `calyx`（PATH 上有 `mei` 或设置 `MEI_BIN`）。从 crates.io 构建 `sprawling` 时，Windows 仍需要 Zig **0.17.0**（Calyx 帧叶）。

### C. 自动化陌生人入口 E2E（不需 API key）

```sh
cargo test -p sprawling-mei --test cli e2e_beginner_workflow_from_first_prompt_to_project_change
cargo test -p sprawling-mei --test observation
cargo test -p sprawling-mei --test workspace
cargo test -p sprawling-mei --test capture
cd Calyx && zig build test
```

这些测试在 Windows 与 Unix 用真实 `mei` + Python 和本地网关，覆盖结果观察、大输出恢复、工作区冻结以及旧日志/媒体块 resume。它们使用隔离的 doctor 缓存，不安装 npm 包、Python、uv，也不驱动 TUI。

## 工作区、运行环境与捕获

新会话启动时，Mei 只读取工作区根目录中存在的 `AGENTS.md`，并把它放在首个 user 消息中真正 prompt 之前。日志冻结这个开头。后续任务和 `/resume` 不重读也不补注 AGENTS，即使文件新出现、修改或删除；只有 `/new` 才重新读取。Skills、MCP 配置与环境工具不自动暴露。

每个 Mei 版本的第一次启动，doctor 探测 Python 与 uv 并缓存完整环境。该版本的有效缓存会跳过全部探测与网络访问。仍受支持的 Python 直接使用；缺失、损坏或已 EOL 的 Python 会换成最新稳定 managed 版本。缺失或损坏的 uv 会被安装。修复 Python 前先刷新 Mei 私有 uv，避免旧 uv 决定哪个 Python 才是最新。

安装留在每用户 doctor 目录，不替换系统 Python、不修改全局 PATH、不编辑 shell profile。所选绝对路径仅用于本地执行；工具描述只携带 `version` 与 `uv_version` 属性。uv 在 python 工具所启动 subprocess 的 PATH 上可用。doctor 永不更新 Mei 本身。`MEI_DOCTOR_DIR` 可覆盖每用户目录，用于隔离和测试。

每个 run 拥有一个全新的 capture 目录。小型 UTF-8 捕获内联，其文件在 fd 转移或解释器收口后回收。大型与非 UTF-8 捕获会 spill：交互路径归 session，位于 `<session>.captures/<run>`，跨 resume 有效；无头路径归调用者，进程退出后仍保留。失败或取消的交互 run 会删除整个未提交目录。Mei 拒绝复用已有 capture 目录。宿主强制崩溃可能留下 orphan 目录；本次不承诺崩溃清理器。

## 进程 Face

| 通道 | 合同 |
|---|---|
| cwd | 调用者工作目录即工作区 |
| `-p` / stdin | 任务；无 `-p` 时 stdin 须为非终端完整提示 |
| `--replay`（Calyx） | 交互面可选 JSONL |
| `--resume <jsonl>` | 可选完整消息日志 |
| stdout | 最终答案，不补换行 |
| stderr | 消息 JSONL；`mei.obs` pulse/usage；失败为三段诊断 |
| 取消 | 杀进程组 |
| tool_result | 不超过 16 KiB 的完整 stdout+stderr；否则为提示与完整捕获路径 |

Resume 保留旧工具名与字符串/数组 content，包括图片、音频和视频块。Provider 与模型必须支持这些块；Mei 没有附件旗标，也不恢复 Python 现场。

| 变量 | 含义 |
|---|---|
| `HARNESS_MODEL` | 必填 magpie `provider/model` |
| `ANTHROPIC_BASE_URL` | 默认 `http://127.0.0.1:3425`；Mei 不监听 |
| `ANTHROPIC_API_KEY` | 默认 `magpie` |
| `HARNESS_EXTRA` | 启动时浅合并一次；拒绝 `stream` / `tools` |
| `MEI_BIN` | Calyx spawn 本核时的路径；npm shim 设为同目录无头核 |
| `MEI_DOCTOR_DIR` | 可选的隔离 doctor 缓存与私有运行环境目录 |
| `MEI_CAPTURE_DIR` | Face 传入的内部 run 目录；该目录不得预先存在 |
| `SPRAWLING_PROGRAM_STATUS` | `never` 关闭 Calyx 交互面的 OSC 7501 |

## 提示词与工具合同

`Mei/src/MEI.md` 在构建时进入 `system`；唯一的 python 定义位于 `tools[0]`。下表使用 Python `3.12.13`、uv `0.12.3` 的固定文档 fixture；实际版本属性来自 doctor。字节数不含消息外层，也不含 provider 自行添加的内容。

| 部分 | UTF-8 字节 |
|---|---:|
| `system` | 297 |
| 完整 compact `tools[0]` JSON | 1145 |
| 合计 | 1442 |

下面的 JSON 为便于阅读做了 pretty-print；字节数按线上 compact 形式计算。

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
  "description": "<python version=\"3.12.13\" uv_version=\"0.12.3\">\nA persistent python interpreter. Variables, imports, and the working directory (cwd) persist across calls. An exception prints its traceback without ending the session, so you can recover and continue without rebuilding state. Expressions do not auto-echo, so explicitly print any values you need to inspect. If combined UTF-8 stdout/stderr is up to 16 KiB, the tool returns it inline. Larger or non-UTF-8 output returns a notice and the path to a file containing the full output; filter that file, or re-run with narrower output, in the next python call. Run shell commands and CLIs through subprocess, capture their output, and check return codes. Start child interpreters with sys.executable, so they inherit the active environment. Because capabilities like web_search, web_fetch or sub-agent delegation are not exposed as built-in tools, python serves as your interface to invoke external APIs, CLIs, headless tools, and more directly via subprocess.\n</python>",
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

Mei 只保留一份跨 provider 的 schema，并在 agent loop 中校验整个工具调用波；不声明 provider 专有的 `strict` 模式。

## 边界

核内无 python 超时；连接 10s / IO 600s；空输出为空文本；超过 16 KiB 或非 UTF-8 只给提示与路径，从不返回头尾样本；无 `MEI_TOOLS`、Skills/MCP 自动暴露、Mei 自更新或核内 TUI。只有新会话的顶层 AGENTS 会注入。

## 规格与检查

[`Spec.lean`](Spec.lean)、[`spec/Agent.lean`](spec/Agent.lean)、[`spec/Session.lean`](spec/Session.lean)、[`spec/Environment.lean`](spec/Environment.lean)、[`spec/Capture.lean`](spec/Capture.lean)；终端 [`Calyx/Spec.lean`](../Calyx/Spec.lean)。

```sh
cargo nextest run -p sprawling-mei
cargo test -p sprawling-mei --test cli
cargo test -p sprawling-mei --test observation
cargo test -p sprawling-mei --test workspace
cargo test -p sprawling-mei --test capture
cargo test -p sprawling-calyx --lib console::mei
cargo clippy -p sprawling-mei -p sprawling-calyx --all-targets -- -D warnings
lake build Mei.Spec Mei.spec.Agent Mei.spec.Session Mei.spec.Environment Mei.spec.Capture
```
