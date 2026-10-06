# 上手指南

这份指南带你从什么都没装，走到一座替你干活的城，并把路上遇到的每个概念讲清楚。它写给两类读者：已经在用 Claude Code、Codex CLI 这类终端 agent 的人，以及只用过聊天窗口、从没把一个文件夹交给模型去干活的人。第一部分讲概念，第二部分从安装到一次落在你分支上、经过审查的合并完整走一遍，第三部分讲城跑起来之后你日常做什么。

查单个词的意思看 [`glossary.md`](glossary.md)；日常操作与出错后的处置看 [`operating.md`](operating.md)；设计看 [`../ARCHITECTURE.md`](../ARCHITECTURE.md)。English: [`getting-started.md`](getting-started.md)。下文按钮与页面的名字取界面的中文；界面切到英文时，两份文档一一对应。

---

# 第一部分　概念

## 从聊天窗口到 agent

在聊天窗口里，你写、模型答，你再把它说的复制进自己的文件。模型只看得到你贴进去的东西，除了写字什么也做不了。

**agent** 是拿到了**工具**的模型：工具是它可以调用的函数，用来读文件、搜目录、跑命令、改文件、打开网页。调用哪个工具由模型决定，外面的程序执行这次调用、把结果交还给它，模型再决定下一步，如此反复，直到模型说活干完了。模型走一遍是一个**回合**（turn），一件活是许多个回合。

模型外面的那个程序叫 **harness**。它决定模型拿到哪些工具、上下文里放什么、能往哪里写、什么事必须先问、记下什么。Claude Code、Codex CLI、Cursor 的 agent 都是一次管一个 agent 的 harness。**sprawling 是同时管许多 agent 的 harness**，在你自己的机器上把它们组织成一座城。

对只用过聊天的人，这带来两点不同。第一，你描述的是结果和怎样算做完，而不是步骤：「让导入器能读大于 2 GB 的文件，并加一个测试证明它能」是好的请求，一串修改清单不是。第二，一件活要几分钟而不是几秒，你也不必盯着：需要你的时候，城会告诉你。

## 模型、provider 与端点

**模型**是负责思考的那个东西——`claude-sonnet-…`、`gpt-…`、`qwen…`，你付费用的或自己跑的都算。**provider** 是提供它的一方：Anthropic、OpenAI、某个中转服务，或者你自己机器上的一个服务。**端点**是城记下的一家 provider：一个 base URL、它说的**兼容格式**、一份凭据。

兼容格式决定每次请求的形状。sprawling 说两种：OpenAI 的 chat 格式（设置页上的 `chat`），大多数 provider、中转和本地服务都接受；以及 Anthropic 的 messages 格式（`messages`）。第三种形状，OpenAI 的 `responses`，列着但会被拒绝，因为城不调用它。

为模型付费有两种方式，城都接受：**API key**，按 token 计费；**本地模型**，在你自己的电脑上服务，不需要 key。城不登录任何订阅。

城给模型分**角色**，免得贵的模型去干便宜的活：

| 角色 | 做什么 |
|---|---|
| `main` | 思考：每次 run 都用它，除非这次 run 指定了别的模型 |
| `digest` | 替 `main` 读长文档与搜索结果；你没另指之前，它跟着 `main` 走 |
| `transcribe` | 把录音转成文字；可选，城里没有这样的模型时页面不画麦克风 |

**强度**（effort）是模型作答前推理多久，档位是 `none`、`low`、`medium`、`high`、`xhigh`、`max`，或者**未指定**——不写这个字段，由 provider 决定。强度越高，每次作答越贵，对做计划、对必须一次做对的活越有用。

## 城

一座**城**是你机器上的一个目录，其余一切都在它里面。把目录拷到另一台机器，它还是同一座城；把它删掉，城外什么都不变。

| 词 | 是什么 |
|---|---|
| **楼**（building） | 一个项目，是城的一个子目录。它的规则、配置、以及 agent 可以写的范围都以它为界。 |
| **房间**（room） | 楼里的一个子目录，一个 agent 在里面干活。它的地址，比如 `lab/parser`，决定了那里的 agent 能写哪些文件、带着哪些文档开工、向谁汇报。 |
| **居民**（resident） | 一个地址上的常驻身份，带一份 `URBANITE.md` 说明自己是谁、什么样的活该拿给它。它跨越多次 run 存在。 |
| **run** | 一件有始有终的活：一个地址、一项任务、一个完成标准、一个模型。居民是身份，run 是成本。 |
| **会话**（session） | 一个房间里的一段对话。往一个房间派活会接着它的会话；`/new` 在同一个地址开一段新的。 |
| **账本**（Ledger） | 城唯一的历史，在 `<city>/.sprawling/ledger`。每个效果发生之前先作为一条事件写进去，你读到的每个页面都由它重建。 |

每座城建起来时就已经有一栋楼：`hall`，即市政厅，两位服务所有楼的居民住在这里。`hall/mayor` 是 **Mayor**，城的规划者：它把一个想法变成计划、为活盖楼、把各自的部分交给各栋楼。它只写 Markdown，因为一个能跑代码的规划者会停止阅读各栋楼给出的证据，转而自己生产证据。`hall/clerk` 是 **clerk**，你把回答委托给它时，它替你回答，并把理由写进账本。

## 脚手架：盖楼时附带的表单

一个 harness 的*脚手架*（scaffolding），是它围在模型四周、让长时间的活保持连贯的结构：agent 开工前要读的文件、收工前要写的文件、以及它改不了的规则。在 sprawling 里，这套脚手架是一组 Markdown 与 TOML 表单，一栋楼盖起来时就落在楼里（模板在 [`crates/city/templates/`](../crates/city/templates/)）。表单靠形状教人：标题都对的一张空表，连小的本地模型也能填对。

| 文件 | 位置 | 谁写 | 用来做什么 |
|---|---|---|---|
| `SPEC.md` | 楼根目录 | 你或 agent | 项目是什么、有哪些决定，先于代码 |
| `RULES.toml` | `<building>/.sprawling/` | 你 | 这栋楼能做什么：agent 能写哪里、活要不要审、准入哪些 skill、能访问哪些网络、能不能用浏览器与桌面、是否保密 |
| `Roadmap.md` | 楼根目录 | agent，经 `plan` 工具 | 唯一的任务表，也是每个进度数字的分母 |
| `Memo.md` | 楼根目录 | agent | 没有别处可放的笔记 |
| `Handoff.md` | 楼根目录 | agent | 下一个会话接着干需要知道的东西 |
| `JOB.md` | 房间 | 你或派活的一方 | 一个会话的任务；agent 读它、不改它 |
| `URBANITE.md` | 居民那里 | 你 | 一位居民是谁、怎么干活 |
| `MAYOR.md`、`CLERK.md` | `<city>/.sprawling/` | 你 | Mayor 与 clerk 是谁 |

`RULES.toml` 和任何 `.sprawling/` 目录下的东西，都在 agent 可写的范围之外。agent 读得到自己的规则、改不了；它可以提议修改，由你回答。

## 工具、skill 与 MCP

**工具**是内建的，由楼的规则发放：`read`、`search`、`edit`、`exec`（跑一个程序或一行 shell）、`plan`、`status`、`neighbours`、`delegate` 等。规则里写了 `browser = true` 的楼，还会拿到一个由城启动、归城所有的浏览器。

**skill** 是一个放着 `SKILL.md` 的文件夹：一组指示，有时附带脚本，教 agent 做好某一类活。在 Claude Code 里用过 skill 的话，文件格式是一样的；区别在准入。城把 skill 放在**书架**上——楼自己的书架 `<building>/.sprawling/skills/`、城的藏书、以及城只读挂载的外部文件夹，比如另一个 harness 已经在用的 skills 目录——但 agent 只看得见它所在的楼在 `RULES.toml` 里按名字准入的那几件：

```toml
reading_room = ["tutor", "blast-radius"]
```

清单上有、书架上没有的名字不会被许诺，而是报出来。这样一栋楼的上下文不会随着磁盘上的 skill 越来越多而膨胀。外部文件夹在城自己的 `<city>/.sprawling/CONFIG.toml` 里挂载：

```toml
[skills]
shelves = ["~/.claude/skills"]
```

**设置**里的 **技能** 一组列出每一格书架、每栋楼准入了什么、每件 skill 被多少次 run 用过。本仓库在 [`../crates/city/skills/`](../crates/city/skills/) 下附带九件 skill；二进制带着它们，新建的城在自己的书架上就能找到每一件。

**MCP**（Model Context Protocol）是一栋楼通往外部应用——邮件、GitHub、Figma、文档转换器——的路，经由一个工具服务。一个 server 是楼的 `CONFIG.toml` 里的一条：电脑上的程序写 `command`，托管服务写 `url`；**MCP** 页替你写同样的条目。保密楼一个也不起。

## 规则、提问与刹车

大多数事情，城按你写下的规则自己决定。agent 的每个动作都要过一道**门**（gate），门放行或拒绝；拒绝总是说清三件事：拒了什么、为什么、换成什么可以做。agent 读了就接着干。

会送到你面前的是**提问**：一位居民凭规则定不下来的设计决定。它在 **等你的事** 页上等着，带 **允许** 与 **拒绝** 两个按钮，也是城唯一会为之弹浏览器通知的一类事。

要让活停下，你有三个动词。**`/steer <文字>`** 给正在跑的 run 加一句指示而不打断它，这句话落在它下一次工具结果的末尾。**`/stop`** 取消你眼前这次 run。**`/halt`** 让一栋楼、或者带 `--all` 让整座城停下，直到 `/release`。这些背后没有花费上限，也没有回合上限：一次 run 一直跑到它得出结论，或者有人叫停。

## 从 Claude Code 过来

| Claude Code 里的 | sprawling 里的 |
|---|---|
| 终端里的会话 | 浏览器里的一页，由二进制提供；终端变成城的控制台 |
| 工作目录 | 一栋楼里的一个房间；它的地址决定 agent 能写哪里 |
| `CLAUDE.md` | 楼的 `SPEC.md` 与 `RULES.toml`、居民的 `URBANITE.md`，以及项目自带的 `AGENTS.md`，城会把它交给在那个项目里干活的 agent |
| `/clear`、`/compact` | `/clear` 与 `/new` 开一段新会话；`/new --carry` 把房间的 `Handoff.md` 带过去 |
| `/model` | `/model <id>`、输入框下的 **模型** 药丸，或设置里的模型表 |
| 思考预算 | **强度** 药丸，从 `none` 到 `max` |
| plan 模式 | 去问 Mayor，做计划本来就是它的活 |
| subagent | 其他居民：一次 run 可以 `delegate`、跟邻居说话或叫醒邻居，每一位都是你能打开看的一次 run |
| 权限提示 | 由 `RULES.toml` 决定；到你面前的只有设计问题，在 **等你的事** 上 |
| `~/.claude/skills` 里的 skill | 同样的文件夹，挂成一格书架，按楼准入 |
| MCP server | 同样的 server，按楼配置，写在 `CONFIG.toml` 或 **MCP** 页上 |
| agent 做过什么的 `git` 历史 | 账本，加上城做的每个提交上的 git trailer |
| 按 Esc 打断 | `/stop`，或 Ctrl+. |

最大的不同是，这里没有哪样东西是一段对话。Mayor 做计划，各栋楼并行干活，你在它们之间走动。

---

# 第二部分　第一个小时，从头到尾

## 你需要什么

需要桌面浏览器和一个能调用的模型：兼容的 provider 或本地模型服务。独立发行二进制不需要 JavaScript 运行时或数据库；npm 需要 Node.js，Bun 渠道使用 Bun。从 crates.io 编译还需要发布包要求的编译器和平台原生构建工具。

## 1 安装

本指南讲的是 sprawling <!-- xtask:begin workspace_version -->0.0.10<!-- xtask:end -->，处于 <!-- xtask:begin maturity:word -->alpha<!-- xtask:end --> 阶段。数据格式、配置、wire 和界面在版本之间仍可能改变；更新前保留可恢复的备份。下列命令安装该渠道实际提供的版本，可能与本指南描述的源码版本不同。

选择一种渠道：

| 渠道 | 安装 | 前提与结果 |
|---|---|---|
| npm | `npm install --global sprawling@latest` | Node.js 与 npm；下载预编译二进制。 |
| Bun | `bun install --global sprawling@latest` | Bun；下载预编译二进制。用 `bun pm bin --global` 找到全局 bin 目录，将它加入 PATH。 |
| crates.io | `cargo install sprawling --locked` | 发布包支持的 Rust 和平台原生构建工具；在本地编译，包内包含构建好的浏览器客户端。 |
| cargo-binstall | `cargo binstall sprawling` | [cargo-binstall](https://github.com/cargo-bins/cargo-binstall)；按已发布 crate 的元数据下载发行归档。 |
| 手动归档 | 在[发行列表](https://github.com/2youg1/sprawling-agents/releases)选择平台 | Windows x86-64、Apple 芯片的 macOS 或 Linux x86-64；解开归档并运行其中的二进制。 |

shell 安装器下载归档，不需要 JavaScript 或 Rust 工具链。macOS/Linux：

```sh
curl -fsSL https://raw.githubusercontent.com/2youg1/sprawling-agents/main/install.sh | sh
```

Windows PowerShell：

```powershell
irm https://raw.githubusercontent.com/2youg1/sprawling-agents/main/install.ps1 | iex
```

脚本打印所选 tag、平台与下载大小，将归档 SHA256 与发行公布的摘要比较，不一致就拒绝。最后调用 `sprawling install`，把二进制复制到你的程序目录并把目录加入 PATH；`sprawling install --uninstall` 撤销这次安装。npm/Bun 和 Cargo 各自管理 bin 目录。PATH 变更尚未进入当前终端时，打开新终端，再检查：

```sh
sprawling version
sprawling help
```

### 选择与验证版本

[发行列表](https://github.com/2youg1/sprawling-agents/releases)包含预发布版本；GitHub 的 `releases/latest` 入口排除它们。选择实际发布的 tag，使用该版的资产名、CHANGELOG 和校验摘要。运行前须替换下文的 `<release-tag>`、`<npm-version>`、`<crate-version>`、`<archive.zip>` 等占位符。Git tag、npm 版本与 crate 版本采用不同拼法，发布流水线通过 [kernel::Release](../crates/kernel/src/release.rs) 派生它们。

npm/Bun 用 `sprawling@<npm-version>` 固定版本；Cargo 用 `cargo install sprawling --locked --version <crate-version>`；binstall 用 `cargo binstall sprawling --version <crate-version>`。shell 安装器仅为本次调用设置完整 tag：

```sh
SPRAWLING_VERSION='<release-tag>' sh -c 'curl -fsSL https://raw.githubusercontent.com/2youg1/sprawling-agents/main/install.sh | sh'
```

PowerShell 保留已有设置：

```powershell
$previousVersion = $env:SPRAWLING_VERSION
try {
    $env:SPRAWLING_VERSION = '<release-tag>'
    irm https://raw.githubusercontent.com/2youg1/sprawling-agents/main/install.ps1 | iex
} finally {
    if ($null -eq $previousVersion) {
        Remove-Item Env:SPRAWLING_VERSION -ErrorAction SilentlyContinue
    } else {
        $env:SPRAWLING_VERSION = $previousVersion
    }
}
```

手动下载时，将 SHA256 与该资产公布的摘要比较。SHA256 核对字节与摘要是否一致，不确认构建者。发布流水线还提供 Sigstore 构件来源证明，可用 [GitHub CLI](https://cli.github.com) 检查：

```sh
gh attestation verify <archive.zip> --repo 2youg1/sprawling-agents
```

操作系统代码签名与这两项检查分别成立。签名申请或流水线配置不能证明下载的二进制已有签名；检查所选资产的签名与发行说明。这些检查均不保证杀毒产品接受文件。操作系统发出警告时，先核对来源，再决定是否运行。

从 checkout 构建完整交付物使用 `just dist`；运行 `just build-web` 之前直接 `cargo build`，嵌入的页面只说明客户端 bundle 缺失。源码构建前提见[贡献指南](CONTRIBUTING.md)。

### 用 AUR

AUR 包尚未发布。发行构建在 Arch Linux 上生成并验证包；发布需要 AUR 账号与 `AUR_SSH_KEY` secret，缺少凭据时以 notice 跳过发布。

发布后，Arch Linux x86-64 用户装好 `base-devel`、Git 与 unzip，可先检查 PKGBUILD，再以普通用户构建：

```sh
git clone https://aur.archlinux.org/sprawling-bin.git
cd sprawling-bin
makepkg -si
```

包验证发行归档 SHA256 并安装二进制、资源与许可，不调用 `sprawling install`，不改 shell 配置。生成的 PKGBUILD 决定安装目录，并将二进制链接到 `/usr/bin`；卸载用 `sudo pacman -R sprawling-bin`。版本检查辨认包的安装布局并显示更新命令；GitHub 新归档可能早于 AUR 同步，更新前仍需检查包版本。

### 用 Nix

Linux x86-64 上可以用 Nix 从本仓库构建完整应用。sprawling 不在 nixpkgs 中打包，Nix 用户使用本仓库的 flake。启用 Nix 的 `nix-command` 和 `flakes`，先检出 `flake.nix` 先构建客户端再构建 Rust、且已提交 `flake.lock` 的 tag 或 commit，再构建：

```bash
nix build .#default --no-update-lock-file
nix run .#default --no-update-lock-file -- status
nix run .#default --no-update-lock-file -- up ./cities/first
```

[flake](../flake.nix) 从 `rust-toolchain.toml` 读取固定的编译器，从 `Cargo.lock` 读取 Rust 依赖，从 `client/bun.lock` 读取客户端依赖；Nix 构建工具由 `flake.lock` 固定。Nix 先获取锁定的依赖，再在网络沙箱中构建应用。构建会调用既有客户端命令并嵌入产物，所以应用带着页面、随附的 skills、模板和许可。首次构建可能需要先编译依赖。

版本行由二进制报告。没有发布来源信息的 checkout 构建会显示 `built from source`，单凭版本号不能确定某个已发布 Git tag。更新时，选择另一个实际 tag 或 commit，再运行上述命令。`nix develop` 打开开发环境，不会启动城。

## 更新

更新由你手动执行。在设置中或用 `sprawling status --check` 检查版本，使用最初安装这份程序的渠道。照提示运行前，先核对它识别的渠道；复制过的二进制或含义不明确的 bin 目录可能无法确定安装器。阅读所选发行 tag 或发行页的 CHANGELOG，重点检查数据格式、配置与 wire 变化。主分支准备中的 CHANGELOG 不证明该版本已经发布。

替换二进制前，完成或取消正在执行的工作，在控制台按 `Ctrl-C` 停城。保留旧二进制或足够重新安装的包版本信息，记下 `sprawling version`。用旧版导出到城外的新目录，再恢复到另一处未使用的目录：

```sh
sprawling version
sprawling export ./cities/first ./backups/before-update.bundle
sprawling restore ./backups/before-update.bundle ./cities/restore-check
sprawling replay ./cities/restore-check/.sprawling/ledger
```

每条命令都成功后再更新。bundle 是目录，仅确认目录存在不能证明备份有效。restore 核对链、清单计数、链头和随行 git 历史；replay 再独立验证恢复后的 Ledger。也要检查恢复副本中的重要项目文件与历史。后续备份和验证使用新的目标目录，导出期间保持原城停止。

导出带走 Ledger、内容对象、城与项目文件及支持的 git 历史；不带凭证明文、派生视图、git hooks/configuration 和城根层的保留配置。用自己的受保护备份流程另存主机配置和城外的项目仓库。凭证仍在主机 vault 中，换主机需要重新登记；远程配对也依赖主机持有的城钥匙。备份包含项目内容和对话历史，应保持私密。确切内容与验证规则见 [Bundle](../crates/storage/spec/Bundle.lean)。

通过原渠道更新：

| 原安装渠道 | 更新 | 重装选定版本 |
|---|---|---|
| npm | `npm install --global sprawling@latest` | `npm install --global sprawling@<npm-version>` |
| Bun | `bun install --global sprawling@latest` | `bun install --global sprawling@<npm-version>` |
| Cargo | `cargo install sprawling --locked` | `cargo install sprawling --locked --version <crate-version>` |
| cargo-binstall | `cargo binstall sprawling` | `cargo binstall sprawling --version <crate-version>` |
| shell/PowerShell 安装器 | 停城后重跑原安装器。 | 按前文将 `SPRAWLING_VERSION` 设为实际发布的完整 tag。 |
| 手动归档 | 下载并验证所选归档；此前安装过时，运行该归档内二进制的 `install` 命令。 | 保留或下载所选 tag 的归档。 |
| AUR sprawling-bin | 在原 checkout 中运行 `git pull --ff-only && makepkg -si` | 检查并构建所选发行对应的 PKGBUILD 修订。 |
| 仓库 Nix flake | 检出所选 tag 或 commit，再运行 [Nix 命令](#用-nix)。 | 使用原固定 checkout 和 lockfile；固定 commit 不会跟随新版。 |

更新后运行 `sprawling version` 与 `sprawling help`，确认 PATH 上解析到预期的二进制，再用 `sprawling up ./cities/first` 打开城。核对页面能连接、配置能读取、项目文件和历史仍在，并执行一件小任务。验证成功之前保留旧版和备份。

### 安全回退

停下更新后的城，通过原渠道恢复旧二进制并检查版本；用旧二进制把更新前的 bundle 恢复到新的城目录，再对它的 Ledger 运行 `replay`。按需恢复另存的主机配置、重新登记凭证，然后启动恢复的城。除非所选发行明确说明兼容，否则不要让旧二进制读取已由新版迁移的城。单独保留更新后的城供检查；回退到备份不会保留备份之后完成的工作。

## 2 建一座城，并把它打开

```bash
sprawling up ./cities/first
```

目录里还没有城时，这一条先把城建起来，然后在 `127.0.0.1:8787` 上服务，并用操作系统打开链接的那个浏览器打开页面。这个终端成为城的控制台：`/help` 列出它接受的命令，`/serving` 重报一遍城在哪里监听，`Ctrl-C` 停城。控制台为城记下的每条记录打一行：它在账本里的位置、事件种类和地址，从不打出你输入的话或模型的回答，因为终端谁站在旁边都看得见；`sprawling up --whole-records` 才把每条记录整条打出来。Windows、macOS、Linux 上打出来的一样。什么参数都不带地跑 `sprawling`，它先给出打算建城的文件夹，按 Enter 就在那里建。

要在已有的项目上干活，就在装着这些项目的文件夹里建城，或者把项目挪进城的文件夹，再用 `sprawling adopt ./cities/first myproject` 把它收为一栋楼。收编不覆盖任何文件：它在你的工作旁边放下城的表单，并加一条 `.gitignore`，让城的笔记不进你项目的历史。

`sprawling doctor` 分两层检查运行它的电脑。第一层是城要用的东西，比如浏览器工具要的浏览器引擎。第二层是改这份代码要的工具，`just prereqs` 读的就是这一层：一台没有管理员权限的新机器，这一层的必需项全部就位，开发环境就算装齐。Windows 上其中一项是 bash，`just` 的每一个配方都在 bash 里跑。请在 Git Bash 里运行 `just`，因为别的终端先找到的 `bash` 可能是 `C:\Windows\System32\bash.exe`，它启动的是 WSL，不是 shell。贡献者不用新机器也能核这一趟：`gh workflow run on-demand.yml -f job=fresh` 在 Windows、macOS、Linux 的 runner 上解开这棵树的发布归档，用新账户的环境跑 `sprawling doctor`，从安装走到第一次派活，把清单与日志上传为 `fresh-<os>-<tree>`。runner 的账户是管理员，所以这个作业核的是新账户的环境，不是没有管理员权限的账户。

## 3 接一家 provider

城还没有能调用的模型时，打开的是 **欢迎** 页；第一张卡 **接上一家供应商** 通往 **设置** → **账户与供应方**。

下一组 **官方 harness** 是订阅进城的地方：城自己从不登录任何订阅。这一页列出 Claude Code、Codex、Grok Build、Kimi Code 与 Pi，写出这台电脑能不能跑起各家的那条命令、那条命令本身，以及厂商自己的登录说明链接；登录是你在 harness 里做的。

已接上的列表下面是一张收 key 的表。它的第一个控件列出城按主机名认得的供应方：选一家，base URL 与兼容格式就替你填好。下面三栏是：

| 字段 | 它要什么 |
|---|---|
| **base_url** | provider 文档里给的 base URL。不写协议头时按 `https://` 读，这台电脑上的地址按 `http://` 读；路径留空时，城按自己的已知主机表补上。表单会显示城实际要调用的地址 |
| **接口形态**（wire_api） | 兼容格式：`chat` 是 OpenAI 的形态，`messages` 是 Anthropic 的形态 |
| **密钥** | provider 的 key；本地服务留空 |

**获取模型列表** 去问端点提供哪些模型，把找到的逐个列出；**添加** 把端点连同你勾选的模型一起登记。表单旁边列出这一步会写下的 `config.toml` 和会发出的请求。密钥不会成为命令或帧的一部分：它进操作系统的凭证服务，此后配置、事件、日志里只有一个 `secret:realm/name` 形式的引用。

然后给模型分角色。模型表有一栏角色，模型可以在添加时就领到角色；之后 **哪个模型来想** 下面每个角色一个框。没有模型担任 `main` 时，派活会被拒，错误码是 `E_MODEL_UNCHOSEN`。上下文窗口和输出上限两栏可以空着：空着的上限先取 provider 声明的数，再取城的已知模型表，最后取默认值。

本地服务也可以在开城之前指定：在 `sprawling serve` 的环境里设好 `SPRAWLING_MODEL_URL` 与 `SPRAWLING_MODEL`，一座还什么都没登记的城就把那个服务不带 key 地接上，并让指定的模型担任 `main`。

## 4 跟 Mayor 说话

**市长** 就是与 `hall/mayor` 的对话；选好模型之后，城打开的就是这一页。Enter 发送，Shift+Enter 换行。

框下面四颗药丸说明这条消息怎么跑，点一下就能改：

| 药丸 | 它定什么 |
|---|---|
| **模型** | 谁来回答；一个会话固定用它开始时的模型，换模型就开一个新会话 |
| **工作区** | 哪个房间在听 |
| **强度** | 模型先推理多久 |
| **模式** | 这次 run 拿你的话做什么：**对话** 回答你说的话，别的什么都不改；**工作** 朝你说的目标把这件事做完 |

用一句话写下想法——*把账本读取重写一遍，让一百万条事件的冷读稳定在一秒以内，并写下你测出来的数*——工作区保持 `hall/mayor`，按 Enter。

Mayor 先读每一栋楼的 `Roadmap.md`、`Memo.md`、`Handoff.md`。它用 `plan` 工具把城的计划写进 `<city>/hall/Roadmap.md`，一行一条工作线。没有哪栋现成的楼该接这件事时，它用 `city` 工具盖一栋，或者收编你指给它的目录，一个项目从不盖两栋楼。它把各自那一份交给各栋楼，用 `pursue` 让那栋楼一直做到这一份做完，回报之前把它决定了什么记进 `<city>/hall/Memo.md`。

你也可以绕过 Mayor。`/raise lab` 按 `minimal` 模板盖一栋叫 `lab` 的楼（`/raise vault confidential` 盖一栋保密楼），再把 **工作区** 药丸切到这栋楼的某个房间，你的话就直接送到那里。

## 5 看着这座城干活

**城市** 一栋楼画一块：亮着的窗是一次 run 在干活，门边的灯表示有东西卡住了，屋顶的旗子是一条长期目标，基座是计划完成了多少。选中一栋楼，旁边列出它的进度、可以开工的、卡住的，以及在这里干过活的 run；再点 **进这栋楼**。

楼的页面显示 **计划**，画的是它的 `Roadmap.md`，一行的状态是 **未开始**、**就绪**、**在做**、**卡住**、**等审批** 或 **做完** 之一；同一页还列出它的 **房间**、**文件**、**提交**、**变更** 和 **技能**。顶上的框给这栋楼定一个长期目标：**设为常设目标** 让它自己不断派出就绪的活，直到没有就绪的活、也没有在途的 run。

每次 run 有自己的页面，有七个视图：**时间**（时间按回合花在哪里）、**回合**、**监视器**（终端、改过的文件和 run 正在处理的文件，实时跟随）、**提示**（发给模型的原文）、**上下文**、**改动** 和 **证据**。在 **监视器** 里，一个改动块可以撤回，也可以 **写意见**，意见作为一句 steer 送到这次 run。

**居民之间写信。** 一个居民写给另一个房间的话叫一封 **信**（letter）。在收信房间的对话里，信是左侧的一张卡片：写明从哪个房间来、链到那个房间的会话，并给出信的种类、时间和正文；你自己说的话从不画成信。在发信一方的工具行里，一次发送读作「send to @room: …」，并写明信落在哪里：交给正在干活的 run、进了队列，或敲门开了一个新 run。等回信的 run 会写明它在等哪个房间、这次等待怎么结束。信只带写信居民的身份，从不带你的身份，即使信里引了你的话；每个居民读到的 `City.md` 都要求它：信里说的某个决定，先到 hall 的 `Memo.md` 或计划里核对，再动手。

**在你自己的编辑器里打开文件。** 在 **设置** → **高级** → **用我的编辑器打开** 里，选这台电脑装的编辑器——VS Code、VS Code Insiders、VSCodium、Cursor、Windsurf 或 Zed——并填上城在这台电脑上的文件夹的绝对路径。此后监视器里显示的每个文件都是一个链接，点开就在那一行打开。链接由浏览器交给编辑器，城不启动任何程序。

## 6 回答居民的提问

**等你的事** 在页面第一栏底部的信箱键后面，里面是居民提出、非等你回答不可的设计问题。同样的问题合成一张卡，**允许** 或 **拒绝** 一次回答整组；被卡住的活会带着你的答案重新派出。起点带着城外内容——一个网页、一个工具结果——的问题标为 **城外内容**，从不合并，好让你知道这段文字从哪里来。

**设置** → **运行** → **审批** 定下谁来回答：**我**，或者 **clerk 代答**；它的决定列在 **代为答复的记录** 里。

## 7 读改动，读那次落地的合并

要落地的活在城内走 pull request，而写这份活的居民不能验证它：没有别人验证过的请求，根本没有「合并」这个方法。按这个顺序读：

1. run 的 **改动** 视图：动过的文件一行一个，点开一行给出那个文件的补丁。被凭证扫描命中的行只报行号与理由，不回显原文。
2. 那栋楼里的 `git log`。城在每一波工具调用前写的检查点，是没有任何 `HEAD` 指向的提交，挂在 `refs/sprawling/runs/` 下，所以你的历史仍是你离开时的形状。
3. 合并提交的 trailer——`Sprawling-Run`、`Sprawling-Actor`、`Sprawling-Model`、`Sprawling-Effort`、`Sprawling-City`，接替另一次 run 的还多一条 `Sprawling-Predecessor`。你复核过的合并另带 `Reviewed-by`，前提是这个仓库的 git config 里有 `user.name` 与 `user.email`。
4. `sprawling whose ./cities/first <commit>` 从账本反过来回答同一个问题，要给完整的四十位提交 id。退出码 1 表示这座城没有写过那个提交的记录。加上 `--trace`，会列出那次 run 从上一个提交以来做过的调用，每条带时间、工具与结局；同一栋楼里在这一段调用过工具的别的 run 作为候选，只给出调用次数。

没有页面替你合并，也没有页面推翻已经合并的活。你的退路是 git 和 **回收站**，那里每个被丢弃的文件都写着怎么取回；你的刹车是 `/halt --all`。

## 8 读花了多少、发生了什么

**成本** 是钱与 token，按 run、按居民、按 prefix 段、按 skill、按工具各切一刀，每一刀加起来都等于同一个总数。provider 没报价格的（比如本地模型），页面数出调用次数和 token，而不是打印 `$0.00`。

**记录** 是同一段历史的四个视图：**账本**（每一条事件，筛选时会说藏了几行）、**归档**（在你问的那一刻，搜遍每一栋楼保存的东西）、**回收站**，以及 **日志**（这个进程的诊断日志）。在终端里，`sprawling view ./cities/first` 不需要城在服务就能读同一本账，`--runs` 打印 run 树。`--since 2026-05-14T09:00:00Z --until 2026-05-14T10:00:00Z` 只留自身时间落在那一小时里的行：UTC，到秒，以 `Z` 结尾，不含终点。

## 9 停下，再开始

在控制台按 `Ctrl-C` 停城。`/halt --all` 停下所有活，城照常服务。崩溃之后：

```bash
sprawling resume ./cities/first
```

这一步验链，关掉结果已丢失的工具调用，报出谁在等你。`sprawling up --supervise` 在每次崩溃后替你做这一步，直到崩溃挨得太近。

---

# 第三部分　与一座城长期相处

## 输入框、命令与键盘

以 `/` 开头的一行是命令，框上方的菜单把它们都列出来：

| 命令 | 做什么 |
|---|---|
| `/dispatch <task>` | 在输入框对着的房间开一次 run |
| `/steer <text>` | 给正在跑的 run 加一句指示，不打断它 |
| `/stop` | 取消你眼前这次 run |
| `/halt [addr\|--all]`、`/release [addr\|--all]` | 让一栋楼或整座城停下，再放开 |
| `/raise <addr> [minimal\|confidential\|hall]` | 按模板盖一栋楼 |
| `/new [--carry]`、`/clear` | 在这个房间开一段新会话；`--carry` 带上房间的 `Handoff.md` |
| `/fork [addr]` | 从一个房间最新的 run 分出第二条对话线 |
| `/model <id>`、`/effort <档位>` | 让 `main` 换一个模型；设定强度 |
| `/diff`、`/go <页面>`、`/mcp`、`/doctor`、`/help` | 打开改动、一个页面、MCP 页、电脑环境检查、命令列表 |

Ctrl+K（Mac 上 ⌘K）给出全部命令，旁边是每一个页面、每一栋楼和每一个房间。自带的快捷键：Ctrl+1 到 Ctrl+6 依次是市长、城市、MCP、记录、成本、登记簿；Ctrl+, 是设置；Ctrl+Shift+A 是 **等你的事**；Ctrl+B 打开信箱；在输入框外按 `\` 换页面画多少城；Ctrl+. 停下你眼前这次 run，页面上没有在跑的 run 时它什么都不停，只提示停整座城要用 `/halt --all`；在输入框外，`/` 把焦点移到输入框，`?` 列出快捷键，`f` 从指针下的那一条分叉。**设置** → **快捷键** 可以改其中任何一个。

## 会话

一个房间保持它的会话，直到你开一段新的，所以发给同一个房间的第二条消息会用同一个模型和强度接着干。`/new` 把这两样都忘掉，你可以重新选，并且什么都不带；`/new --carry` 带上房间的 `Handoff.md`，也就是上一段会话写给后来者的五节摘要。`/fork` 开一段新会话，它从另一次 run 的对话截至某一行开始，这样你可以试第二种做法而不丢掉第一种。一次上下文窗口快满的 run 会得到提醒，剩下的预算还够它写一份交接、把活交给同一地址上的继任者；这条链上的每一环都是你能打开的一次 run。

## 一栋你自己的楼

按 `minimal` 盖的楼是普通的：它的 agent 能写楼里的每个文件，活不经审查，不准入任何 skill。在你的编辑器里打开 `<building>/.sprawling/RULES.toml` 就能改；每个键在文件里都有说明，城不认识的键会被拒绝而不是被忽略。人们最先改的几个键：

```toml
does = "实验室仪器日志的导入器。"          # 写给初到这里的 agent 的一段话
conventions = "Rust 2024；每次改动都要通过 cargo test。"
review = true                  # 活在落地之前要经过审查
reading_room = ["tutor"]       # 这栋楼准入的 skill
egress = ["api.github.com"]    # 这里的活可以访问的域名
browser = true                 # 浏览器工具
```

`sprawling check ./cities/first` 读城里的每个 TOML 文件，把每个错误打印成 `path:line:column`。

**保密楼**（`/raise vault confidential`）用于不能外流的数据：在那里，run 在任何一次远程 provider 调用之前就被停下，不起任何 MCP server，往别的楼写会被拒绝。配本地模型用。

## 外部工具

**MCP** 页可以用一条命令、一个 URL 或一段粘贴的 JSON 给一栋楼加一个工具服务，并显示每个 server 是否应答。同样的条目写在 `<building>/.sprawling/CONFIG.toml` 里：

```toml
[[mcp]]
label = "docs"
command = "markitdown-mcp"
```

[`operating.md`](operating.md) 完整讲了这个 server，还讲了用第二座城的一栋楼驱动浏览器去看第一座城。

## 用终端或脚本驱动一座城

```bash
sprawling dispatch lab "给 report 命令加一个 --json 参数"   # 打印事件直到 run 结束
sprawling dispatch lab "…" --detach                          # 打印 run id 就返回
sprawling top                                                # 在终端里看监视器
sprawling call '{"ask":{"ask_id":1,"query":"city_view"}}'    # 发一帧线协议
```

不带帧的 `sprawling call` 列出线协议上的每个命令和查询。它的退出码就是答案——0 答了、1 被拒、2 你的命令行有问题、3 时限内什么都没回来、4 那个地址上没有城——所以脚本按退出码分支，不必解析 JSON。[`wire.md`](wire.md) 是整套线协议，写给从外面驱动城的 agent。

## 同一网络里的另一台机器

```bash
sprawling serve ./cities/first 0.0.0.0:8787
```

伸出这台电脑的地址需要一把配对钥匙。设了 `SPRAWLING_PAIRING_TOKEN`，城就采用它，并且从不打印；没设，城为这一次服务现铸一把，只在启动横幅里印一次，旁边附一个带着它的地址。下一次启动换一把新的。从你的网络之外，设备经远程门与一条你选的通路够到这座城。设置 → 远程可以按你选的时长开门、关门、换城钥匙；开门与换钥匙会在跑 `sprawling serve` 的终端上印一个码，你在两分钟之内把它输进页面，Windows、macOS、Linux 上都一样。配对设备仍在控制台上做：[operating.md](operating.md) 的 *Reaching the city from another device* 一节写了怎么做，以及通路被托付了什么。

## 搬一座城

```bash
sprawling export ./cities/first city.bundle
sprawling restore city.bundle ./cities/copy
sprawling replay ./cities/copy/.sprawling/ledger
```

还原在报告成功之前拿账本链对照包里的清单，`replay` 离线、只读地验链。

## 出了问题

| 你看到 | 它是什么意思 |
|---|---|
| 页面说它说的 wire 版本和服务端不同 | 二进制和浏览器里的页面不是一起构建的。刷新。 |
| 按了 **获取模型列表**，列表是空的 | provider 答了，但答的内容这个构建读不出来。表单下的报告写着停在哪一步：主机名、连接，还是应答。 |
| 还没开跑，派活就被拒 | 没有模型担任 `main`：`E_MODEL_UNCHOSEN`。给一个模型这个角色。 |
| 一个 run 停下来要东西 | 它在 **等你的事** 上。 |
| **计划** 上有一行是 **卡住** | 它依赖的东西还没完成，或者某项检查没过；那一行写着它在等什么。 |
| 一次合并被拒 | 这份活分叉之后主干动过了。那栋楼把活重建在当前主干上，再验一次。 |
| 一个文件不见了 | 去 **回收站**：每一行都写着怎么把那个文件取回来。 |
| 一个设置没生效 | `sprawling check <city>` 打印城里 TOML 文件的每个错误。 |
| 编辑器链接点了没反应 | **设置** → **高级** 里选的编辑器不是这台电脑注册的那个，或者填的文件夹不是这台电脑上的绝对路径。 |
| 这台电脑缺城需要的东西 | `sprawling doctor` 列出来，`--install` 逐项征得同意后安装。 |

更多这类情况和各自的处置在 [`operating.md`](operating.md)（英文）。
