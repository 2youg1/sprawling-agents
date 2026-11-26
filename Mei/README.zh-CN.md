# Archetype:Mei

[English](README.md) · [简体中文](README.zh-CN.md)

**一个工具，一个持久的 Python 解释器，其余由你决定。** Mei 适合想把 Agent Harness 压到极简、自己掌握工具和运行环境的人，也适合当一个专做小任务的子代理。可以用 [Calyx](../Calyx/README.zh-CN.md) 交互，从脚本调用，或用 `mei --acp` 接到支持 ACP 的主 Agent。要用哪个 Python 库、CLI 或 API，由你明确选择。

Rust 写的无头核心通过 Magpie 接入兼容 Anthropic Messages 的模型路由，一次运行只走一条串行的 Python 工具循环。会话用 JSONL 保存，可以续接。独立的 Calyx 控制台用 Zig 实现。

> **安全优先：** Mei 没有沙箱、权限提示或撤销。生成的 Python 可以读写文件、访问网络，并以当前账户权限运行命令。只用于一次性项目副本和单独、可撤销且有限额的 API key。不要提供私密数据、重要凭证或项目唯一副本。

## 快速上手

先打开 [Magpie](https://usemagpie.ai) 并选好模型路由，然后安装：

```sh
bun install --global @sprawling/mei@latest
```

安装后打开新终端，在一次性目录中试跑。找不到 `mei` 时，见[安装章节的 PATH 设置](#安装)。

```sh
mei -p "Print hello using Python, without changing files."
```

用 `mei` 进入交互界面，或把 `mei --acp` 配为 [ACP 子代理](../docs/integrate/integrations.md)。

## 安装

| 方式 | 命令 | 得到什么 |
|---|---|---|
| Bun | `bun install --global @sprawling/mei@latest` | npm 包同时携带无头 `mei` 核与 Calyx 面，并提供 `mei`、`calyx` 两个命令。TTY 下 `mei` 进入 Calyx；`mei -p`、`mei --acp` 与管道仍走无头。`bunx @sprawling/mei` 可不安装直接跑一次。 |
| crates.io | `cargo install sprawling-mei --locked` | 只装无头 `mei` 核。TTY 下无 `-p` 的 `mei` 会以 `E_MEI_USAGE` 拒绝并指向本说明：此通道没有 Calyx 面。交互请用 Bun 或源码构建。 |

Bun 全局安装后，**新开终端**再运行 `mei` 或 `calyx`。包的 `bin` 字段已足够：Bun 会把两个名字链进它的 global bin，**不依赖**生命周期脚本。若新壳找不到命令，先跑 `bun pm bin --global`，再（仅当前会话）：

```sh
export PATH="$(bun pm bin --global):$PATH"
```

把该目录写进登录壳会读的文件，之后新终端才一直可用。若 Bun 执行了包的 postinstall，它还可能在 `~/.local` 下的 bin 目录放包装器并打印同样的 **Fix your PATH**；生命周期被跳过时，上面的 global bin PATH 修复仍然有效。用同一条 Bun 命令重装即可刷新 shim 与平台二进制。不要省略 optionalDependencies：平台包里才有两个二进制。

已发布包会落后于本树，直到下一次发布；要本仓库的版本，用下方源码构建。

### 该敲哪个命令

| 安装方式 | 交互 | 无头 / CI |
|---|---|---|
| Bun `@sprawling/mei` | TTY 下敲 `mei`（npm shim 打开 Calyx）；`calyx` 也可 | `mei -p …`、`mei --acp` 或管道 |
| 源码构建 / release zip | 只用 `calyx` | `mei -p …`、`mei --acp` 或管道 |
| crates.io `sprawling-mei` | 无 Calyx 面；TTY 无 `-p` 的 `mei` 以 `E_MEI_USAGE` 拒绝 | `mei -p …` 或管道 |

Bun 用户只需记住 `mei`。源码、release zip 或 crates.io 的无头 `mei` 二进制不会打开 Calyx；交互请运行含 `sprawling-calyx` 构建里的 `calyx`。

### 更新与卸载

Mei 没有自更新。要换版本，用同一通道再装一次：

```sh
bun install --global @sprawling/mei@latest
# 或
cargo install sprawling-mei --locked
```

卸载 Bun 全局安装时，去掉根包，并清掉当前平台的 optional 依赖：

```sh
bun remove --global @sprawling/mei
```

crates.io 安装用 `cargo uninstall sprawling-mei` 卸掉。源码构建只删 `target/debug`（或 `target/release`）下的 `mei` 与 `calyx` 程序，保留共享 target 树；release zip 只删解压出的 `mei`/`calyx`。这些路径都不改 shell profile。

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

模型、网关和其他参数都不用输入。交互面请运行 `calyx`；`mei -p` 与管道仍走无头。非交互 CI 必须用 `mei -p` 或管道，避免误进 Calyx 面等待输入。

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
| `/e`、`/effect` | 打开 Magpie 支持的思考强度菜单，选择下一次任务的设置 |
| Esc | 清草稿，或停止当前任务及其进程树；绝不退出 Calyx |
| `/quit`、`/q` | 关闭当前 Calyx 及其进程树，其他会话互不影响 |

模型选择继续交给 Magpie 路由，不另设 `/model` 命令。

### 3. 查看状态栏

composer 示例：

```text
actual-model · effort high · Agent Step 2 · Input 487
```

Magpie 尚未确认路由时，模型和思考强度标记为 `requested`。`Agent Step` 统计全会话的模型响应数。`Input` 累加全会话的输入、缓存创建和缓存读取 token。旧记录缺少用量时，`—` 表示未知，`≥` 表示已有读数的下界。

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

<a name="内存"></a>

## 性能

Mei 0.18.3 和 Pi 1.6.1 在同一台 Linux x86-64 主机运行，CPU 配额为 4 vCPU，内存上限为 16 GiB。每项任务启动独立的无头进程，连接同一个本地脚本模型替身。文件任务执行相同的 Python 代码，Mei 使用持久解释器，Pi 通过 Bash 工具调用 Python。续接任务读取相同的 1 MiB 会话历史。

下表选取 Mei 的 P50 和 P95 均表现较好的负载。每格依次为 **P50 / P95 / CV**。CV 是样本标准差除以均值。吞吐按批次统计，活跃内存取最终请求阶段的进程树总 PSS。

<!-- mei-pi:begin -->
| 指标 | 负载 | 并发 | 样本 Mei / Pi | Mei | Pi |
| --- | --- | ---: | ---: | ---: | ---: |
| 启动至首次请求 (ms) | 短答 | 1 | 40 / 40 | 1.66 / 2.18 / 17.1% | 277.23 / 366.19 / 14.5% |
| 任务总耗时 (ms) | 短答 | 8 | 320 / 320 | 5.86 / 9.15 / 57.9% | 847.97 / 1,031.37 / 17.8% |
| 任务总耗时 (ms) | 64 KiB 输入 | 8 | 320 / 320 | 8.71 / 11.64 / 23.9% | 836.53 / 1,030.74 / 18.6% |
| 任务总耗时 (ms) | 1 MiB 历史续接 | 8 | 320 / 320 | 30.54 / 43.20 / 80.7% | 852.85 / 1,055.11 / 17.0% |
| 任务总耗时 (ms) | 三轮工具调用 | 1 | 40 / 40 | 28.37 / 34.19 / 70.3% | 384.34 / 536.38 / 14.9% |
| 任务总耗时 (ms) | 三轮工具调用 | 8 | 320 / 320 | 58.08 / 75.65 / 31.4% | 1,174.17 / 1,403.07 / 14.3% |
| 后续工具往返 (ms) | 三轮工具调用 | 8 | 640 / 640 | 1.78 / 7.27 / 102.0% | 69.80 / 138.30 / 44.3% |
| 批次吞吐 (任务/s) | 三轮工具调用 | 8 | 40 / 40 | 117.55 / 136.58 / 15.0% | 6.33 / 6.93 / 7.5% |
| 活跃进程树 PSS (MiB) | 三轮工具调用 | 1 | 40 / 40 | 8.78 / 8.81 / 0.5% | 100.75 / 102.98 / 1.1% |
| 活跃进程树 PSS (MiB) | 三轮工具调用 | 8 | 40 / 40 | 51.70 / 51.71 / 0.04% | 610.96 / 620.41 / 1.3% |
<!-- mei-pi:end -->

完整报告涵盖 1、4、8 并发、分布图、区间、任务验证和测量方法。固定响应测量的是本地 Harness 表现。在报告页面选择“Download raw file”，然后用浏览器打开下载的 HTML 文件查看图表。 [HTML 报告](https://github.com/2youg1/sprawling-agents/blob/main/docs/benchmarks/mei-pi/report.html) · [测试记录 CSV](https://github.com/2youg1/sprawling-agents/blob/main/docs/benchmarks/mei-pi/results.csv)

## Prompt 与工具合同

[查看 system prompt 和 Python 工具 schema](TOOL-CONTRACT.zh-CN.md)。

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
