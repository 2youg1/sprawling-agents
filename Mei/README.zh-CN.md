# Mei

[English](README.md) · [简体中文](README.zh-CN.md)

Mei 是极简 Agent Harness：把任务交给模型，模型通过一个 Python 3 工具行动，每次效应只拿回**路径句柄**，再继续直到最终答案。本包是**无头核**。Mei 不含终端壳。

交互面是 [Calyx](../Calyx/)。`@sprawling/mei` 同时发布无头 `mei` 与 `calyx` 面：TTY 且无 `-p` 时走 Calyx，`-p` 或管道仍走无头核。脚本只用无头路径。

> **先读安全提示：** Mei 没有沙箱、权限确认或撤销。模型生成的 Python 能以当前账户权限读写文件、联网和执行命令。只把可丢失的项目副本交给它；使用单独、可撤销且设好消费上限的 API key。不要提供真实隐私、重要凭据或唯一一份项目。

## 安装与运行

| 目标 | 命令 |
|---|---|
| 脚本 / 单次 | `bunx @sprawling/mei -p '…'`，或 `cargo install sprawling-mei --locked` 后 `mei -p '…'` |
| 交互（`/new`、`/resume`、Esc） | 在工作区执行 `bunx @sprawling/mei`；或 `cargo install sprawling-mei sprawling-calyx --locked` 后执行 `calyx`（同目录 `mei` 或 `MEI_BIN`） |

`@sprawling/mei` 发布无头核与 Calyx 面（Windows x64、macOS arm64、Linux x64 musl）。Mei 永不进程内嵌壳；Calyx 同时服务 `sprawling` 里的城控制台。

## 黑盒路径（只用键盘）

需要能联网、支持工具调用的模型 API key，以及可弄坏的目录。不必克隆本仓库。

### A. 脚本冒烟

1. 需要时安装 [bun](https://bun.sh)。
2. 一个终端里配置 magpie：

```sh
curl -fsSL https://usemagpie.ai/install.sh | sh -s -- --tui
magpie provider add deepseek
magpie serve
```

3. 第二个终端：

```sh
export HARNESS_MODEL='deepseek/deepseek-flash'
mkdir -p ~/mei-playground && cd ~/mei-playground
bunx @sprawling/mei -p '请调用 Python 计算 40 + 2，只回答数字。'
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

首句：

```text
先阅读项目说明和目录，不要修改任何文件。告诉我怎样构建和测试，并建议一个范围最小的首个任务。
```

用 `git status` 和项目自己的测试验收。

不经 npm 时：`cargo install sprawling-mei sprawling-calyx --locked`，再执行 `calyx`（PATH 上有 `mei` 或设置 `MEI_BIN`）。从 crates.io 构建 `sprawling` 时，Windows 仍需要 Zig **0.17.0**（Calyx 帧叶）。

### C. 自动化陌生人入口 E2E（不需 API key）

```sh
cargo test -p sprawling-mei --test cli e2e_beginner_workflow_from_first_prompt_to_project_change
cargo test -p sprawling-calyx --lib console::mei
cargo test -p sprawling-calyx --lib console::ui::mei
```

第一条用真实 `mei` + Python + 本地脚本网关（计算 → 改项目 → 测项目）。不安装 npm，也不驱动 TUI。

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
| tool_result | 捕获 stdout+stderr 的路径 |

| 变量 | 含义 |
|---|---|
| `HARNESS_MODEL` | 必填 magpie `provider/model` |
| `ANTHROPIC_BASE_URL` | 默认 `http://127.0.0.1:3425`；Mei 不监听 |
| `ANTHROPIC_API_KEY` | 默认 `magpie` |
| `HARNESS_EXTRA` | 启动时浅合并一次；拒绝 `stream` / `tools` |
| `MEI_BIN` | Calyx spawn 本核时的路径；npm shim 设为同目录无头核 |

## 边界

核内无 Python 超时；连接 10s / IO 600s；空输出仍是路径；无 `MEI_TOOLS`、无输出窗截断、无运行时 AGENTS/Skills 注入、无核内 TUI。

## 规格与检查

[`Spec.lean`](Spec.lean)、[`spec/Agent.lean`](spec/Agent.lean)、[`spec/Session.lean`](spec/Session.lean)；终端 [`Calyx/Spec.lean`](../Calyx/Spec.lean)。

```sh
cargo nextest run -p sprawling-mei
cargo test -p sprawling-mei --test cli
cargo test -p sprawling-calyx --lib console::mei
cargo clippy -p sprawling-mei -p sprawling-calyx --all-targets -- -D warnings
lake build Mei.Spec Mei.spec.Agent Mei.spec.Session
```
