<div align="center">

# sprawling

**在你自己的机器上，把许多 Agent 组织成一座城。一个 Rust 二进制，界面是浏览器里的一页。**

<p align="center">
  <a href="https://crates.io/crates/sprawling"><img alt="crates.io" src="https://img.shields.io/crates/v/sprawling?logo=rust&amp;labelColor=171717&amp;color=DEA584"></a>
  <a href="https://www.npmjs.com/package/sprawling"><img alt="npm" src="https://img.shields.io/npm/v/sprawling?logo=npm&amp;labelColor=171717&amp;color=CB3837"></a>
  <a href="LICENSE"><img alt="License" src="https://img.shields.io/github/license/2youg1/sprawling-agents?labelColor=171717&amp;color=4C8BF5"></a>
  <a href="https://deepwiki.com/2youg1/sprawling-agents"><img alt="Ask DeepWiki" src="https://deepwiki.com/badge.svg"></a>
</p>

</div>

> **状态：<!-- xtask:begin maturity:word -->alpha<!-- xtask:end -->，研究与开发中。** 可以拿来干真活，但数据格式、协议和界面在版本之间仍可能变。主回路是通的：接上一家 provider 或一个 harness、盖一栋楼、派活，几个 agent 就同时在楼里调用工具、写文件。把真活交给它之前，请先读[现在能做什么，还不能做什么](#现在能做什么还不能做什么)。
>
> English: [README.md](README.md) · 给从外面驱动一座城的 Agent 看的：[LLM.md](LLM.md) · 想改代码：[AGENTS.md](AGENTS.md)

## 它是什么

Claude Code、Codex CLI 这类终端 agent 一次给你一个 agent：一个模型、一套工具、一个工作目录、一段对话。sprawling 在一台机器上把许多个组织成一座城：**城**（city）是一个目录，城里每个项目是一栋**楼**（building），每个 agent 在楼里的一个**房间**（room）干活，每件活是一次有始有终的 **run**。你说清要什么结果、怎样算做完，agent 就去读文件、跑命令、写代码和文档，只在你写下的规则定不了的地方来问你。它们做的每件事都记在一本只追加的 **Ledger** 里，你在二进制送到浏览器的页面上看着这一切。

房间的居民有两种。一种是城自己驱动的模型：用一把 API key 接任何说 OpenAI 或 Anthropic 兼容格式的 provider，或者接一个本地模型服务。另一种是官方 harness，比如 Claude Code：城经 Agent Client Protocol（ACP）把它起起来，你在里面用自己的订阅登录。sprawling 负责排活、发工具、记历史并摆给你看，思考交给模型。

它面向两类人：想让一组 agent 长期替自己跑固定工作的小团队；想观察 agent 怎样分工、怎样互相说话的研究者，不论计算机还是人文社科。一台旧笔记本或一台便宜的云主机就放得下一座城。

## 快速开始

1. 安装：

   ```bash
   curl -fsSL https://raw.githubusercontent.com/2youg1/sprawling/main/install.sh | sh    # macOS、Linux
   irm https://raw.githubusercontent.com/2youg1/sprawling/main/install.ps1 | iex         # Windows PowerShell
   ```

   其他途径：从[最新 release](../../releases/latest) 下载你系统的归档（Windows x86-64、Apple 芯片的 macOS、Linux x86-64）；`bunx sprawling up` 或 `npx sprawling up`；`cargo binstall sprawling`，取的就是那份归档；装了 1.97 或更新的 Rust，用 `cargo install sprawling --locked`。这些二进制没有代码签名：Windows 上选 **More info → Run anyway**，macOS 上在 Finder 里右键打开一次。

2. 建一座城并打开它：

   ```bash
   sprawling up ~/cities/first
   ```

   这个终端成为城的控制台，并打印城在哪个地址上服务；浏览器随即打开页面。在控制台按 `Ctrl-C` 停城。

3. 在页面上接一家 provider：填 base URL、它说的兼容格式和一把 key，然后给 `main` 这个角色选一个模型。key 直接进操作系统的凭据库，页面之后只见到 `secret:realm/name` 形式的引用。

4. 在页面底部的输入框里把你想要的告诉 Mayor。Mayor 做计划、为活盖楼、把各自的部分交给各栋楼；你看着 run 跑，回答它们的提问，审它们改了什么。

从 git 检出构建请用 `just dist`：直接 `cargo build` 时还没有跑过 `just build-web`，嵌进二进制的只是一张空白页。完整指南是 [`docs/getting-started.zh-CN.md`](docs/getting-started.zh-CN.md)（[English](docs/getting-started.md)），从空目录一直讲到一次审过、落在你分支上的合并。

## 五个词

| 词 | 是什么 |
|---|---|
| **City** | 一台机器上的一座城：一棵目录树、一本 Ledger、一段完整历史。两座城从不互相引用。 |
| **Building** | 城里的一个项目。配置、档案与写域都以它为范围，它的规则写在 `RULES.toml`。 |
| **Room** | 楼里的一个子目录。一个 Agent 在一个房间里干活。 |
| **Run** | 一件有始有终的活。**居民是身份，run 是成本。** |
| **Ledger** | 唯一的历史。一行一条事件，只追加，可离线校验。 |

其余词汇见 [`docs/glossary.md`](docs/glossary.md)。

## 用 ACP 接入你自己的 harness

房间的居民可以是五家官方 harness 之一：Claude Code、Codex、Grok Build、Kimi Code、Pi，城把它作为 ACP agent 起起来。订阅就是这样进城的：你在 harness 里登录，城自己不登录任何东西。在一栋楼的 `.sprawling/CONFIG.toml` 里点名 harness，此后这栋楼新开的房间都以它为居民：

```toml
[resident]
harness = "claude_code"   # 或 "codex"、"grok_build"、"kimi_code"、"pi"
```

设置里的**官方 harness** 一组列出启动每家 harness 的命令（其中四家用 Node 的 `npx`，Kimi Code 用 `kimi` 二进制）、这台电脑能不能跑它，以及厂商讲怎样登录的那一页。harness 自己执行工具，所以城管得了它写在哪，管不了它做什么：harness 在房间自己的 git worktree 里干活，城记下它汇报的内容；遇到停摆或楼的 `harness_minutes` 上限，城发出 ACP 取消；harness 回答之后，城提交那棵树，把这件活交去审查，审过才合并。保密楼不收 harness 居民，因为 harness 会把房间里的内容送到它自己的厂商那里。

## 它的行为

- 一个地址同时回答三个问题：`lab/room1` 是磁盘上的一个位置，这个位置决定那里的 agent 能写什么、带着哪些文档开工、向谁汇报。
- agent 自己找到彼此并说上话，不用你传话。对方正在干活，话落在它下一次工具结果的末尾；对方闲着，城为它开一次 run。只有 User 自己的入口能以 User 的名义说话，守住这一条的是类型。
- Ledger 是唯一的历史：每个效果先写成事件再发生，页面上的每个视图都能从它逐字节重建，日志被改动一个字节，验链就停在那一行。
- 每次删除都自带退路，回收站点一下就把文件放回去。
- 每个 agent 在自己的 git worktree 上干活，改动要经另一位居民验证才能合并；自己验收自己的活是编译错误。
- 你的 git 历史保持原样。检查点放在 `refs/sprawling/runs/` 下，只有空仓库的第一个提交和审过的活合并进来才会移动分支。`Sprawling-*` trailer 写明每个提交出自哪次 run、哪位居民、哪个模型，`sprawling whose <city> <commit>` 从 Ledger 回答某个提交是哪次 run 写的。
- 成本可以按 run、居民、prefix 段、工具和 skill 分开看，取 provider 的账单，没有账单就按价目表算。根本没有价格时（比如订阅），页面直接说没有价格，而不是打印 `$0.00`。
- 浏览器通知只为需要你做的决定而弹。
- 有些错误写不出来：把凭证发上线路、删除却不留退路、没有证据就宣称完成、给计划的一部分比父节点更多的权重、自己验收自己的活，这些在类型里都无法表示，<!-- xtask:begin compile_fail_cases -->19<!-- xtask:end --> 个编译失败用例证明了这一点。

## 现在能做什么，还不能做什么

| 方面 | 能做什么 |
|---|---|
| 居民 | 任何说 OpenAI 或 Anthropic 兼容格式的 provider、本地模型服务，以及经 ACP 接入的五家官方 harness。 |
| 干活 | 几个 run 同时跑，各在自己的 git worktree 上，审过才合并；常驻目标推进计划里所有就绪的部分；居民互相说话、互相叫醒：信号与派活一发出就生效，发信的一方也可以停下来等回信。 |
| 工具 | 内置工具、楼准入的 skill、任何走 stdio、HTTP 或 SSE 的 MCP server、城驱动的浏览器，以及 allowlist 约束下的 Windows 桌面。 |
| 运行 | 内存吃紧时新 run 排队，盘快满时派活在写任何东西之前就被拒；城可以暂停，`up --supervise` 在崩溃后把它拉起来。 |
| 恢复 | 回收站、崩溃后的 `resume`、离线验链、导出一座城并在另一台机器上还原。 |
| 页面 | 与任何房间的对话、有自己名字和标签的 session、城、楼、run、记录、成本、MCP、性能监视器、可折叠的设置树、配色页，以及文档工作区：编辑 Markdown 与纯文本，预览 PDF 与 DOCX，看版本和 diff。 |
| 连接 | 同一网络里的机器凭配对钥匙连进来，网络之外经远程门和你自己选的通路连进来。城钥匙存在城的保险库里，Windows 上是凭据管理器，macOS 上是钥匙串，城重启后配好的设备不用重新配对；Linux 的内核密钥环只保存到电脑重启，用加密的保险库文件则跨重启也保留。 |

| 还没做到 | 现状 |
|---|---|
| Windows 与 macOS 上的操作系统沙箱 | 命令在工作树的一份拷贝里执行：你的文件伤不着，但网络是敞开的。装了命名空间封装的 Linux 还会关掉网络、收住整个进程树。今天的承诺是「删除可以撤销」，不是「删除不会发生」。沙箱臂的名字与每个平台的缺省已经定了（[`docs/operating.md`](docs/operating.md) 的 *How `exec` is confined* 一节），Windows 与 macOS 的臂还没做。 |
| harness 居民用上城的工具 | harness 拿不到城的协作工具，也拿不到楼里的 MCP server；它一回合进行中收到的 steer 会记下来，但送不到；它请求权限时，城一律答第一个 allow-once 选项，范围限在它自己的 worktree 里。 |
| skill 审核 | 账本里有记 skill 审核的那一行，skill 的审核状态也从它读出，但还没有任何代码去问 skills.sh 或 SkillSpector，所以每件 skill 都显示未审。skill 页与 MCP 页按账本折出每一件被用过几次。 |
| 经过检查的手机布局 | 窄于 768 px 时页面排成一栏，但渲染门不检查比 768 px 更窄的宽度。 |
| CI 里的浏览器端到端测试 | CI 用真实浏览器引擎在样例上渲染每个定稿的屏，但没有哪个作业去驱动一座活着的城。 |
| 跨机器逐字节一致的构建 | 同一台机器上构建两次结果一致，夜间作业会查。跨机器时记下的源码路径不同，要等 `trim-paths` 进入本项目钉住的 stable 工具链才能去掉。 |
| 把花费归到 skill | skill 是 prefix 里的一行，不是调用上下文，所以每次调用都落进 `no_skill` 那一格。 |

## 在终端里

页面不是唯一的门。下面几个命令都跟同一座城说话：

```bash
sprawling up [city-dir] [addr]        # 没有城就先建，然后服务，并打开页面
sprawling dispatch <addr> <task>      # 给一座正在服务的城派一件活，打印它的事件直到 run 结束
sprawling view <city>                 # 只读地读一座城的 Ledger 行或它的 run 树
sprawling doctor [<city>]             # 这台电脑有的，对照一座城需要的
sprawling resume <city-dir>           # 崩溃之后：验链、关掉丢了结果的工具调用、报告谁在等你
sprawling export <city> <bundle-dir>  # 打包整座城；`restore` 在另一台机器上解开
sprawling help [<verb>]               # 全部命令，或解释其中一个
```

没有东西会自动更新：`sprawling status --check`，或设置里的那个按钮，告诉你有没有新版本发布，替换二进制始终由你来做。[`LLM.md`](LLM.md) 为脚本或另一个 agent 写明了这套线协议。

## 安全与隐私

- 默认只监听回环地址。绑定别的地址就要一把配对钥匙：设了 `SPRAWLING_PAIRING_TOKEN`，城就用它；没设，城现铸一把，并打印带着它的地址。
- 你网络之外的手机或另一台电脑经远程门连进城。在设置 → 远程里可以开门、关门、换城钥匙；开门与换钥匙要你把城印在它自己控制台上的码输进页面才做，所以驱动页面的工具两样都做不成；配对设备只在控制台上做。通路由你选：Cloudflare 命名隧道，或你自己写的一条命令，例如包一层 `tailscale serve`。配对与每一帧都端到端加密，通路从不持有密钥，所以你托付给通路的只剩一件事：原样送达页面。[`docs/operating.md`](docs/operating.md) 写了两种通路。
- key 放在操作系统的凭据库里，配置只留 `secret:realm/name`。模型的输出、日志、每个 git 检查点的内容，写下之前都要过一遍密钥扫描。
- 没有账号，没有遥测，没有托管服务。保密楼不调用远程 provider，不起外部工具服务，也不收 harness 居民，配上本地模型就能处理隐私数据。

## 为什么做它

我换过很多 Harness，有些理念落后，有些超出实际。就以 RSI 来说，在 LLM 本身脱离无状态之前，Harness 能做的只是不断针对最新的模型做适配，并学习公司现有的业务流程、更快地运行它们；前者的趋势是消融实验，后者需要隐私。

越来越多的小公司是用大量 Agent 开发在线服务的小团队，其中大多数就是一堆 Markdown 加几个天才。我想要一个跟得上多 Agent 工作、又能务实对待 RSI 与记忆风潮的 Harness：务实的可扩展、节省 User 的精力、Agent 规模化时的成本控制、隐私与可靠、长时运行，这些放在设计的核心，再结合一些城市学与社会学的内容。

Agent 越强，User 的注意力就越贵，sprawling 不打算成为又一个抢你注意力的应用。用它最好的方式是从写 Prompt 转向设计 Loop：排好工作流，让 Agent 接管固定工作，你偶尔回来看一眼跑得怎样。你的文件、代码、文档库就是记忆；在无状态的模型上由 Harness 注入「记忆」，大多只会拖累模型。

诚实地说，目前还没有哪种多 Agent 方案带来的提升对得起规模化的成本，而研究 Agent 集群里模型如何交互、协作、表现出社会性，才刚刚开始。建议先让一座城保持精简，只放业务需要的 skill、MCP server 与 harness，遇到具体问题再加东西：同一个模型换一个 Harness，行为就完全不同。

**优点**：体积小；概念好玩；从一开始就为许多 Agent 设计，而不是一个 Agent 挂一圈扩展。**缺点**：没有实验室或赞助方支持的学生项目；页面离它想成为的样子还有距离；稳定性与可用性都还要调。

两个姊妹项目暂时搁着。[RefRain](https://github.com/2youg1/RefRain) 是一个写作工作台，agent 提修改、只有你能合并，它延迟开发，文档能力正在迁进本项目的页面；[kusanagi](https://github.com/2youg1/kusanagi) 是不需要可信服务器的 Agent 通信，暂缓开发。

## 文档

除了这一页、上手指南和 crate 规格的注释，文档都是英文。

| 文档 | 内容 |
|---|---|
| [`docs/getting-started.zh-CN.md`](docs/getting-started.zh-CN.md)（[English](docs/getting-started.md)） | 新手指南：从安装到一次审过的合并，路上遇到的每个概念 |
| [`docs/operating.md`](docs/operating.md) | 日常使用：引导和停下工作、回答居民、换 provider 与 MCP server、远程访问、出了问题怎么办 |
| [`docs/glossary.md`](docs/glossary.md) | 代码、页面与文档所用的每个词，各只有一个意思 |
| [`LLM.md`](LLM.md) | 线协议与命令行，写给从外面驱动一座城的 agent 或脚本 |
| [`ARCHITECTURE.md`](ARCHITECTURE.md) | 各个 crate 与依赖规则、缝、一次派活从头到尾、磁盘上有什么、怎样验证、每类改动该去哪里 |
| [`AGENTS.md`](AGENTS.md) | 每次改动要守的规则与检查它们的命令，人和 agent 都先读它 |
| [`docs/CONTRIBUTING.md`](docs/CONTRIBUTING.md) | `AGENTS.md` 的长版：每条规则和守住它的门 |
| [`docs/frontend-method.md`](docs/frontend-method.md) | 一个屏怎样搭建与验收，以及已批准的视觉设计 |
| [`crates/README.md`](crates/README.md) | 每个 crate 管什么、规格在哪；规格 `crates/<dir>/Spec.lean` 写着该 crate 的接口、决定与证明，注释用中文 |
| [`crates/city/templates/`](crates/city/templates/) | 城写进每栋楼的文档，agent 读它们，你也可以读 |
| [`crates/city/skills/`](crates/city/skills/) | 随发布分发的 skill |
| [`docs/logging.md`](docs/logging.md) | 什么进诊断日志、什么进 Ledger，以及两者为什么分开 |
| [`docs/third-party.md`](docs/third-party.md) | 本仓库跟随的上游事实、致谢与许可义务 |
| [`CHANGELOG.md`](CHANGELOG.md) | 每一版改了什么，以及留下了哪些已知未修的问题 |
| [`SECURITY.md`](SECURITY.md) | 怎样报告漏洞 |

## 参与开发

从 [`AGENTS.md`](AGENTS.md) 开始：

```bash
cargo install just --locked
just prereqs        # 这个回路需要的其余工具，缺哪个就给出哪个的安装命令
just check          # 绿了，一次改动就算完成
```

PR 描述、issue、评审意见都可以用你自己的语言写。附一份对照译文更好，母语不是英文就附英文，是英文就附中文：两种语言并排，人和 Agent 都读得更快，误译也藏不住。其余见 [`docs/CONTRIBUTING.md`](docs/CONTRIBUTING.md)。

## 致谢

厂商相关的事实，比如 API 挂在哪条路径下、说哪种兼容格式、每家官方 harness 怎样启动，取自 ACP registry；厂商自己的客户端比文档写得更准时，就跟随那个客户端，但不抄它的代码。读的是哪个仓库、哪条路径、哪个提交，都写在 [`docs/third-party.md`](docs/third-party.md) §1，每天有一个工作流去问每个上游有没有变。页面上的每个控件都是本仓库自己写的，键盘行为参照 W3C 的 ARIA Authoring Practices 以及 Kobalte 与 Ark UI 的文档，读的是文字。

[`crates/city/skills/`](crates/city/skills/) 下，`sdd`、`tutor`、`translation` 是我此前以 AGPL-3.0-or-later 发布的中文 skill 的英文改编（translation 的原署名还有 Claude Fable 5），在这里用 MPL-2.0。`why`、`how`、`blast-radius` 是我对 Lauren Tan（poteto）的 [pstack](https://github.com/cursor/plugins/tree/main/pstack) 所做的修改改编，`authority-review` 改编自同一个 `cursor/plugins` 树里的 Thermos，四个都保留 MIT。条款见 [`docs/third-party.md`](docs/third-party.md) §5。

## 基于 sprawling 开发

sprawling 欢迎在它之上开发。它的大部分部件要么在一条缝后面，要么集中在一个文件里，换掉一处不必碰其余部分：

| 层 | 可以替换的部分 |
|---|---|
| 页面 | 整个客户端：`crates/wire` 里的线协议就是全部 API，任何语言写的客户端都能顶替随附的这一个；也可以保留它，只改颜色与动效（`client/src/theme.css`）或页面上的每一个词（`client/src/lang.json`）。 |
| 居民 | 官方 harness 及其启动方式（`agent_protocols::harness`）、盖楼时写进楼里的文档（`crates/city/templates/`）、楼准入的 skill。 |
| 工具 | `kernel::tool` 后面的内置工具、任何 MCP server、`browser::port` 后面的浏览器驱动、Windows 桌面 server（`crates/desktop`）。 |
| 模型 | 已知 provider 主机表（`gateway::provider::preset`）、请求格式（`gateway::dialect`），以及端点本身，本地模型也算在内。 |
| 执行 | `runtime::sandbox` 后面的 WebAssembly 沙箱，以及宿主命令所受的约束（`runtime::tools::exec::confinement`）。 |
| 历史与连接 | `kernel::ledger` 后面的 Ledger 存储，以及 `remote_access::route` 后面的远程通路，比如你自己的隧道。 |

每一处怎么换，见 [`ARCHITECTURE.md`](ARCHITECTURE.md)：§4 列出每条缝和它已有的第二个实现，§8 写明每类改动先打开哪个文件、改错了哪项检查会变红。把这份文件、对应 crate 的规格和 [`AGENTS.md`](AGENTS.md) 连同你的需求一起交给 agent，它通常就能把改动做完。

## 许可

MPL-2.0，见 [`LICENSE`](LICENSE)。[`crates/city/skills/`](crates/city/skills/) 下每个 skill 各自写明许可，[`crates/city/skills/LICENSES.md`](crates/city/skills/LICENSES.md) 随发布归档一起分发，给出每一个的条款与署名。

---

问题、bug 报告、不同意见都欢迎：开一个 issue，或者写信到我主页上的地址。
