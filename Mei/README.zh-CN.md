# Archetype:Mei

[English](README.md) · [简体中文](README.zh-CN.md)

Mei 是一个研究“最小 Agent Harness 能有多小”的学术原型：把任务交给模型，模型通过一个 Python 3 工具行动，再把结果交回模型，直到得到最终答案。它适合学习、实验和二次开发，不是开箱即用的成熟产品。

> **先读安全提示：** Mei 没有沙箱、权限确认或撤销按钮。模型生成的 Python 能以当前账户权限读写文件、联网和执行命令。只把可丢失的项目副本交给它；使用单独、可撤销且设好消费上限的 API key。不要提供真实隐私、重要凭据或唯一一份项目。

## 从零开始：一台全新的 Windows 笔记本

这条路线假设你只有一台能联网的 Windows 10/11 电脑，没有仓库、Docker、Linux 或开发工具，也没用过 Agent。你需要管理员权限，以及一家模型服务商的 API key；模型必须支持工具调用，调用可能收费。

### 先认识这些词

| 词 | 这里指什么 |
|---|---|
| Agent | 不只是聊天：模型可以调用工具，例如用 Python 读文件或算数 |
| 终端 | 输入命令的文字窗口；PowerShell 和 Ubuntu 是两种不同窗口 |
| WSL / Ubuntu | WSL 让 Windows 运行 Linux；Ubuntu 是这里安装的 Linux 系统 |
| Python | Mei 交给模型的唯一本地工具 |
| Git | 下载源码、查看项目改动的工具 |
| Rust / Cargo | Mei 的实现语言及其构建工具 |
| Zig | 构建 `sprawling mei` 的终端渲染器，版本必须是 0.17.0 |
| magpie | 把不同模型服务商统一接给 Mei 的本地网关 |
| API key | 模型服务商发给你的调用凭据，类似密码，不能公开 |

WSL 能降低误改 Windows 文件的概率，但**不是绝对安全隔离**，仍能访问 Windows 文件和网络。不要让它处理 `/mnt/c` 下的私人文件；需要更强隔离时，改用一次性虚拟机、容器、bwrap 或专用账户。

### 1. 安装并打开 Ubuntu

在 Windows 开始菜单搜索 **PowerShell**，右键选择“以管理员身份运行”，执行：

```powershell
wsl --install -d Ubuntu
```

按提示重启电脑。重启后从开始菜单打开 **Ubuntu**，首次启动会要求创建 Linux 用户名和密码。输入密码时屏幕不显示字符，这是正常现象。

以后仍从开始菜单打开 Ubuntu。本节之后，**除特别标明外，命令都在 Ubuntu 终端里执行，不要贴进 PowerShell。** 微软的[官方安装说明](https://learn.microsoft.com/windows/wsl/install)可处理安装失败等特殊情况。

### 2. 安装全部依赖

在 Ubuntu 中执行：

```sh
sudo apt update
sudo apt install -y ca-certificates curl git python3 build-essential xz-utils
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
curl -fsSL https://usemagpie.ai/install.sh | sh -s -- --tui
export PATH="$HOME/.local/bin:$PATH"
```

`sudo` 可能要求输入刚创建的 Ubuntu 密码。上述步骤安装下载与构建工具、Git、Python、Rust 和 magpie，不需要 Docker、Node.js 或数据库。

再安装终端渲染器要求的精确 Zig 版本。下面支持普通 x86-64 和 ARM64 笔记本，会核对官方 SHA-256，最后应打印 `0.17.0`：

```sh
case "$(uname -m)" in
  x86_64) ZIG_ARCH=x86_64; ZIG_SHA256=1cbe9df9f27e6b78d14ccbca43b6703a404ef79ef1c463de901d7f088d4e2026 ;;
  aarch64) ZIG_ARCH=aarch64; ZIG_SHA256=9e8d11661d4ae3bd57702a3832781e23ad151dde5798e16a5ccd503f65234ff8 ;;
  *) echo "不支持的 CPU：$(uname -m)"; exit 1 ;;
esac
cd /tmp
curl -fSLO "https://ziglang.org/download/0.17.0/zig-$ZIG_ARCH-linux-0.17.0.tar.xz"
echo "$ZIG_SHA256  zig-$ZIG_ARCH-linux-0.17.0.tar.xz" | sha256sum -c -
sudo tar -C /opt -xf "zig-$ZIG_ARCH-linux-0.17.0.tar.xz"
sudo ln -sf "/opt/zig-$ZIG_ARCH-linux-0.17.0/zig" /usr/local/bin/zig
cd ~
zig version
```

检查其它工具：

```sh
git --version
python3 --version
cargo --version
magpie help
```

前三条显示版本，最后一条显示 magpie 命令列表，就可以继续。

### 3. 配置模型

先在模型服务商网站创建 API key，并设置额度或消费上限。下面以 DeepSeek 为例；其他服务商可用 `magpie presets` 查看。

```sh
read -rsp '粘贴 DeepSeek API key，然后按 Enter：' DEEPSEEK_KEY; echo
magpie provider add deepseek "$DEEPSEEK_KEY"
unset DEEPSEEK_KEY
magpie models
```

粘贴 key 时不会显示。最后一条列出形如 `provider/model` 的可用模型；选择支持工具调用的模型，若列表中有 `deepseek/deepseek-flash`，可用它完成下面的例子。

保持当前窗口不变，启动网关：

```sh
magpie serve
```

这个窗口要一直保持打开。再从开始菜单打开**第二个 Ubuntu 窗口**，后续 Mei 命令都在第二个窗口执行。Mei 默认连接同一 Ubuntu 环境中的 `http://127.0.0.1:3425`，无需另填地址或网关 key。

### 4. 下载、构建并打开 Mei

在第二个 Ubuntu 窗口执行：

```sh
source "$HOME/.cargo/env"
export PATH="$HOME/.local/bin:$PATH"
cd ~
git clone https://github.com/2youg1/sprawling-agents.git
cd sprawling-agents
cargo build -p sprawling-mei -p sprawling
```

把模型名换成 `magpie models` 实际列出的名字：

```sh
export HARNESS_MODEL='deepseek/deepseek-flash'
mkdir -p ~/mei-playground
cd ~/mei-playground
MEI_BIN="$HOME/sprawling-agents/target/debug/mei" "$HOME/sprawling-agents/target/debug/sprawling" mei
```

`HARNESS_MODEL` 告诉 Mei 使用哪个模型，关闭窗口后需要重新设置。最后一条打开 Mei 的交互终端。第一次提交任务时，会在当前目录新建一个 `mei-….jsonl` 文件并打印完整路径。

### 5. 第一次简单会话

在 Mei 输入行输入下面这句话，按 Enter：

```text
请调用 Python 计算 40 + 2，只回答数字。
```

回答应该是 `42`。Mei 等每份非流式响应完整到达后才显示，中途安静不一定是卡死。终端会显示模型答复和 Python 调用结果。

当前 JSONL 保存这段对话的初始问题和每条成功消息。在这个终端里继续提交任务，会接着这份历史；每个任务都会重新启动 Python，所以 Python 变量不会跨任务保留。

### 6. 有哪些特性和指令

**特性：**

- 只有一个本地 `python` 工具，但可通过它操作文件、shell、网络和项目命令。
- 同一任务内，Python 变量、imports、工作目录和环境修改会保留。
- 模型与工具串行往返；最终 stdout 只有答案，观察消息可保存为 JSONL。
- 可编辑 [`src/MEI.md`](src/MEI.md) 改变构建时的系统提示词；也可用 `MEI_TOOLS` 追加服务商执行的工具。
- 不提供权限界面、费用上限、自动重试、长对话整理或 Python 进程现场恢复。

**在 `sprawling mei` 里：**

| 输入 | 作用 |
|---|---|
| 普通文字后按 Enter | 在当前 JSONL 对话中运行一项任务 |
| `/new` | 新建一个 JSONL，从空白历史开始；旧文件原地保留 |
| `/resume <路径.jsonl>` | 导入指定的完整 JSONL 并继续；成功的新段落追加到该文件 |
| Shift+Enter | 换行 |
| Esc | 依次清空草稿、停止当前任务、退出 |

Mei 只有这两个斜杠命令。`/resume` 必须给路径，不会猜测最近文件。导入日志必须从初始 user prompt 开始，以 assistant 的 `end_turn` 结束；畸形或不完整日志会在下一次模型请求和 Python 启动前报 `E_MEI_SESSION`。恢复消息不会恢复 Python 变量或进程现场。

只读查看一份日志：

```sh
"$HOME/sprawling-agents/target/debug/sprawling" mei --replay run.jsonl
```

脚本使用的独立二进制仍是单任务，没有斜杠命令：

```sh
"$HOME/sprawling-agents/target/debug/mei" -p '请调用 Python 计算 40 + 2' 2>run.jsonl
printf '介绍当前目录' | "$HOME/sprawling-agents/target/debug/mei"
```

它可选的 `--resume <jsonl>` 参数就是终端恢复会话所用的进程接口。停止独立任务按 Ctrl+C。

### 7. 开始第一个大型项目

先只复制一个**可以弄坏的项目副本**。假设副本在 Windows 桌面，替换下面路径中的用户名和文件夹名：

```sh
mkdir -p ~/work
cp -r "/mnt/c/Users/Windows用户名/Desktop/项目副本" ~/work/my-project
cd ~/work/my-project
export HARNESS_MODEL='deepseek/deepseek-flash'
MEI_BIN="$HOME/sprawling-agents/target/debug/mei" "$HOME/sprawling-agents/target/debug/sprawling" mei
```

`/mnt/c` 是 Ubuntu 看到的 Windows C 盘。不要复制 `.env`、密钥或私人资料，也不要直接操作唯一原件。

先做只读调查：

```text
先阅读项目说明和目录，不要修改任何文件。告诉我怎样构建和测试，并建议一个范围最小的首个任务。
```

确认建议合理后，每次只派一个有明确边界的任务：

```text
目标：……。范围：只修改……。完成标准：运行……并通过；做不到就报告卡点，不扩大改动。完成后列出改动文件、测试命令和结果。
```

终端会把历史带进当前 JSONL，但 Mei 不会预读整个大型项目，也不会压缩过长的对话。无关历史可能干扰模型时用 `/new`，然后重新写清目标、范围与完成标准。不要只说“把整个项目优化好”。

最后自己验收：若项目使用 Git，运行 `git status --short` 和 `git diff`；没有 Git 就与原始副本比较。亲自再跑项目测试，不能把模型的“已完成”当作测试证据。Mei 没有硬性的回合或费用上限，需要停止时按 Esc，并在 magpie 中检查用量。

### 下次打开与常见问题

重开 Ubuntu 后，在一个窗口运行 `magpie serve`；另一个窗口重新执行 `source "$HOME/.cargo/env"`、`export PATH="$HOME/.local/bin:$PATH"` 和 `export HARNESS_MODEL=…`，进入项目目录并启动 `sprawling mei`。用 `/resume <路径>` 继续指定日志。

| 现象 | 处理方法 |
|---|---|
| 找不到 `cargo` 或 `magpie` | 重新执行上面的 `source` 与 `export PATH` |
| Zig 拒绝构建 | `zig version` 必须恰好显示 `0.17.0` |
| `E_MEI_CONFIG` / `HARNESS_MODEL` | 把变量设成 `magpie models` 列出的名字 |
| `E_MEI_TRANSPORT` / 连接失败 | 确认 `magpie serve` 仍在同一 Ubuntu 环境的另一个窗口运行 |
| 找不到 `python3` | 执行 `sudo apt install -y python3` |
| `E_MEI_SESSION` | 换一份完整 JSONL，或输入 `/new`；不完整的任务不能恢复 |
| 服务商返回 401、402、429 或工具错误 | 检查 key、额度、模型名及工具调用支持；必要时换模型 |

## 技术参考

### 结构

- 唯一状态：包含全部 messages 的 Anthropic Messages 请求 JSON。
- 唯一循环：发送请求，按 content 顺序执行合法 Python 调用，追加结果，在 `end_turn` 停止。
- 唯一本地工具：每个任务内的一个持久 Python 3 解释器。
- 唯一协议：HTTP/1.0 上非流式 Anthropic Messages。
- 无沙箱、权限 UI、成本核算、重试、压缩、Provider 抽象、专用文件/shell 工具或 Python 进程恢复。

### 环境变量与进程接口

| 变量 | 含义 |
|---|---|
| `HARNESS_MODEL` | 必填，magpie 的 `provider/model` 拼法 |
| `ANTHROPIC_BASE_URL` | 出站 Anthropic 网关；默认 `http://127.0.0.1:3425`，Mei 不监听此地址或 `2501` |
| `ANTHROPIC_API_KEY` | 网关 key；默认 `magpie` |
| `HARNESS_EXTRA` | 启动时浅合并一次的一个 JSON object；拒绝 `stream: true` 和 `tools` |
| `MEI_TOOLS` | 可选 JSON array，追加在本地 Python 声明之后；Provider 支持时设置 `max_uses` |
| `HARNESS` | 递归调用的可执行文件；未预设时才由 Mei 填为 `current_exe()` |

独立命令的语法是 `mei [-p PROMPT] [--resume JSONL]`；没有 `-p` 时，由非终端 stdin 提供完整任务。恢复只按文件顺序导入 role/content；当前环境重新决定 model、system、tools 与网关。

成功 stdout 只有最终文本且不补换行。成功的 stderr 段先写新 user prompt，再写 assistant/tool-result JSONL；assistant 观察行另带 `stop_reason`，恢复时剥去。fatal error 仍是纯文本；首次传输失败不会在原始服务商错误前写入 prompt。

### 必须知道的边界

- Python 单次调用的 250 秒闹钟只是尽力超时；代码可改掉或忽略它，C 调用也可能越过它。硬性截止时间要由外部进程管理器执行。
- 传输对每个网关地址有 10 秒连接预算、600 秒读写预算；停滞会报 `E_MEI_TRANSPORT`，不会重试。
- 成功收尾时解释器有 5 秒退出宽限，随后会被 kill 并回收；这是正常清理，不是任务失败。
- 输出增量解码，只保留解码块及首尾各 8000 字符；超过 16000 字符会截断并返回完整临时文件路径。
- 工具波先整波验证再产生副作用；重复 id、未知客户端工具、畸形调用和空波都是协议错误。
- 解释器或私有信道死亡会结束任务；已有副作用不回滚，也不伪造完整结果。
- 递归子进程共享文件系统和外部隔离边界，但不共享父级 messages 或 Python namespace。
- 透明服务端工具可返回任意大的块，绕过 Python 输出窗口并在每轮重传；声明时应限制用量。

### 规格与检查

核心合同见 [`Spec.lean`](Spec.lean)，串行反馈模型见 [`spec/Agent.lean`](spec/Agent.lean)，显式 JSONL 恢复模型见 [`spec/Session.lean`](spec/Session.lean)，终端适配合同见 `crates/sprawling/spec/Console.lean` §8-174。

```sh
cargo nextest run -p sprawling-mei
cargo test -p sprawling-mei --test cli e2e_beginner_workflow_from_first_prompt_to_project_change
cargo test -p sprawling --lib console::mei
cargo test -p sprawling --lib console::ui::mei
cargo clippy -p sprawling-mei --all-targets --all-features -- -D warnings
lake build Mei.Spec Mei.spec.Agent Mei.spec.Session
```

第二条是本文主流程的 E2E：真实 `mei` 进程、真实 Python、临时项目和本地模拟网关完成首次计算、项目修改与项目测试，不访问真实模型，也不需要 API key。恢复测试覆盖完整历史导入与副作用前拒绝；终端测试覆盖 `/new` 文件唯一、旧文件不变、显式 `/resume` 和成功才追加。POSIX 行为在 Unix lane 验证。

### 已通过 magpie 观察到的行为

真实 V1/V2 仅通过本机 magpie 的 `deepseek/deepseek-flash` 验证，不代表支持所有 Provider 的兼容格式：直接 Messages 请求返回 HTTP/1.0 与非流式 `end_turn`；计算任务完成 Python 调用并保留签名 thinking；另一任务通过 Python 读到文件标记。原始观察流只是本地调试记录，不是 Python 现场恢复或 Provider 无关夹具。
