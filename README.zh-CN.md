# sprawling

**在你自己的机器上，把许多 Agent 组织成一座城。一个 Rust 二进制，界面是浏览器里的一页。**

[![npm](https://img.shields.io/npm/v/sprawling?logo=npm&labelColor=171717&color=CB3837)](https://www.npmjs.com/package/sprawling)[![License](https://img.shields.io/github/license/2youg1/sprawling-agents?labelColor=171717&color=4C8BF5)](LICENSE)[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/2youg1/sprawling-agents)[![zread](https://img.shields.io/badge/Ask_Zread-_.svg?style=flat-square&color=00b0aa&labelColor=000000)](https://zread.ai/2youg1/sprawling-agents)

> **状态：pre-alpha，研究与开发中。** 主回路是通的：在浏览器里注册一个 provider、盖一栋楼、派一件活，模型会调用工具并把文件写进那栋楼。多个 Agent 各在各的房间里开工；同一栋楼的几轮活可以同时跑，而写进 Ledger 的只有一条记账线程。还没做到的写在[现在能做什么、还不能做什么](#现在能做什么还不能做什么)那一节，把真活交给它之前请先读。
>
> English: [README.md](README.md) · 给从外面驱动一座城的 Agent 看的：[LLM.md](LLM.md) · 想改代码：[AGENTS.md](AGENTS.md)

## 它是什么

用过 Claude Code、Codex CLI 这类终端里的 agent，你就熟悉一个 agent 的样子：一个模型、一套工具、一个工作目录、一段对话。sprawling 保留这个样子，把许多个放在一处。一座**城**（city）是你机器上的一个目录；城里的每个项目是一栋**楼**（building），每个 agent 在楼里的一个**房间**（room）干活，每一件活是一次有始有终的 **run**。你通过二进制在 `127.0.0.1:8787` 上提供的页面与城打交道。

如果你只用过聊天窗口，区别在于 agent 是干活，不是回答。你说清要什么结果、怎样算做完；模型接着读文件、跑命令、写代码和文档，只有遇到你写下的规则替它定不了的决定时才来问你。它做过的每件事都有记录，它删掉的每个文件都能放回去。

sprawling 自己不思考，它要一个能调用的模型：一个说 OpenAI 或 Anthropic 兼容格式的 provider 的 API key，或者一个本地模型服务。它负责给 agent 排活、发工具、记历史，并把这一切摆给你看。

**给谁用。** 想让一组 agent 替自己长期跑固定工作的小团队；想观察 agent 如何分工、如何互相说话的研究者，不论计算机还是人文社科。它面向配置一般的机器设计，一台旧笔记本或一台便宜的云主机就放得下一座城。

## 快速开始

1. 从[最新 release](../../releases/latest) 下载你系统的归档——Windows（x86-64）、macOS（Apple 芯片）或 Linux（x86-64）——解压到任意位置。或者一行装好：

   ```bash
   curl -fsSL https://raw.githubusercontent.com/2youg1/sprawling/main/install.sh | sh    # macOS、Linux
   irm https://raw.githubusercontent.com/2youg1/sprawling/main/install.ps1 | iex         # Windows PowerShell
   ```

   已经装了 bun 或 node 的话，`bunx sprawling up` 或 `npx sprawling up` 会从 npm 取来同一个二进制。

2. 建一座城并打开它：

   ```bash
   sprawling up ~/cities/first
   ```

   这个终端成为城的控制台，浏览器打开 `http://127.0.0.1:8787`。在控制台按 `Ctrl-C` 停城。什么参数都不带地运行，它会先说出打算新建的文件夹，等你同意。

3. 在页面上**接上一家供应商**：一个 base URL、它说的兼容格式、一把 key。key 直接进操作系统的凭据库，页面之后只见到 `secret:realm/name` 形式的引用。给 `main` 这个角色选一个模型。

4. 在页面底部的输入框里把你想要的告诉 Mayor。Mayor 做计划、为活盖楼、把各自的部分交给各栋楼；你看着 run 跑、回答它们的提问、读它们改了什么。

这些二进制没有代码签名，第一次运行会触发警告：Windows 上选 **More info → Run anyway**，macOS 上在 Finder 里右键打开一次。**不要用 `cargo install` 装它**：页面由 bun 构建再嵌进二进制，只跑 cargo build 得到的页面一片空白。请用 release 归档，或者用 `just dist` 构建。

**完整指南是 [`docs/getting-started.zh-CN.md`](docs/getting-started.zh-CN.md)**（[English](docs/getting-started.md)）：新手会遇到的每个概念——harness、provider、模型角色、skill、模板、会话、审批——以及从空目录到一次经过审查、落在你分支上的合并的整条回路。

## 五个词

| 词 | 是什么 |
|---|---|
| **City** | 一台机器上的一座城：一棵目录树、一本 Ledger、一段完整历史。两座城从不互相引用。 |
| **Building** | 城里的一个项目。配置、档案与写域都以它为范围，它的规则写在 `RULES.toml`。 |
| **Room** | 楼里的一个子目录。一个 Agent 在一个房间里干活。 |
| **Run** | 一件有始有终的活。**居民是身份，run 是成本。** |
| **Ledger** | 唯一的历史。一行一条事件，只追加，可离线校验。 |

其余词汇见 [`docs/glossary.md`](docs/glossary.md)。

## 它的行为

**一个地址同时回答三个问题。** `lab/room1` 指磁盘上的一个位置，这个位置决定了那里的 agent 能写哪些文件、带着哪些文档开工、向谁汇报。三个答案不需要任何机制维持一致，因为它们读自同一个事实。

**Agent 自己找到彼此并说上话，不需要你在中间传话。** 一次 run 可以问本楼还有谁，拿回它够得着的每一个地址，以及那位居民自己的 `URBANITE.md` 写的「什么样的活该拿给我」。对方正在干活，话从门缝塞进去，落在他下一次工具结果的末尾；对方没在干活，城为他开一次 run。两条路上这句话都带着发件人的地址。一位居民永远冒充不了你：只有人自己的入口能造出一句以人的身份说的话，这是类型的性质，不是一条约定。

**Ledger 是唯一历史。** 任何效果先成为一条事件，再成为效果。页面上的每一个视图都是这条事件流的 projection：删掉一个，从 Ledger 重建，字节一致。日志被改动一个字节，验链会报出那一行并拒绝往下走。

**删除自带退路。** 表示「丢弃一个文件」的类型没有不带 Restoration 的构造函数，所以「删了永远回不来」写不出来。回收站里每一行都写着自己的退路，点一下就把文件放回原处；你之后在那个路径上新建了文件的话，它就不放。

**你合并的活，是别人审过的。** 一座城里多个 agent 各在自己的 git worktree 上干活，改动要经另一位居民验证才能合回去。自己验收自己的活是一个编译错误，而不是一条需要谁记住的规则。

**成本按五刀切开**：按 run、按居民、按 prefix 段、按工具、按 skill。每一笔是 provider 报出的计费额；provider 不报时，用价目表算出的数。provider 根本不给价格时（比如订阅），页面直接说没有价格，而不是打印 `$0.00`，并数出没有定价的调用次数与 token 数。

**页面的设计是不抢你的注意力。** 浏览器通知只为一类事而弹：一个需要你来做的决定。进度通过标题和图标传到隐藏的标签页，其余的都在你会去找的地方等着。

**有些状态不是被校验，而是不可表示。** 把凭证发上线路、丢弃一个文件却不留退路、把封存的凭证塞进 Ledger 载荷、没有证据就声称活已完成、给计划的一部分比它父节点更多的权重、自己验收自己的活——这些在类型系统里写不出来。测试里有 <!-- xtask:begin compile_fail_cases -->18<!-- xtask:end --> 个编译失败用例，因为「写不出来」本身就是一个需要被证明的断言。

## 现在能做什么，还不能做什么

**能做**：注册 provider 并选模型；盖楼、派活，让模型调用工具并把文件写进那栋楼；居民互相找到、说话、叫醒，不用人传话；给一栋楼接一个外部 MCP server；一座城里多个 agent 同时开工，各自有自己的 git worktree，改动经另一位居民审过才合回去；一栋有常驻目标的楼一次推进它全部就绪的活；没指定模型的 run 沿用它所在房间开始时的那个模型；内存吃紧时新 run 排队，城所在的盘快满时派活在写任何东西之前就被拒；暂停一座城再放开；从回收站放回丢弃的文件；把一栋楼移出城，文件留在保留子树、历史留在 Ledger；离线验链；导出一座城并在另一台机器上还原；把监视器里显示的文件在 VS Code、VS Code Insiders、VSCodium、Cursor、Windsurf 或 Zed 里打开到那一行。

页面上有：与任何一个房间的对话（默认是 Mayor 的）、城、每一栋楼、每一次 run、记录（Ledger、档案、回收站、日志）、成本、登记簿、MCP、性能监视器，以及设置。

**还没做，以及原因**：

| 缺的部分 | 原因 |
|---|---|
| 每个平台上的操作系统沙箱 | agent 跑的命令由平台提供的手段约束，exec 工具自己的说明写着这一次用的是哪一条、这一条挡不住什么。装了命名空间封装的 Linux 上，命令在自己的命名空间里跑；Windows 与 macOS 上，它在工作树的一份拷贝里跑，你的文件伤不着，但网络对它敞开。没人验证过的隔离比没有更糟，因为人会把它当防线，所以今天的说法是「删除可以撤销」，而不是「删除不会发生」。 |
| 为手机排版的页面 | 渲染门在 768、1280、2560 三个 CSS 像素宽度上判页面，所以竖着拿的平板是它已知放得下的最窄屏幕。手机比那道门查的任何宽度都窄；而从你自己的网络之外连进一座城，需要一条你自己选的隧道（见下文）。 |
| CI 里的浏览器端到端 | CI 用真实浏览器引擎在样例上打开每一个定稿的屏（`cargo xtask render`），但没有哪个 CI 作业用浏览器去驱动一座活着的城。 |
| 跨机器逐字节一致的构建 | `cargo xtask repro` 用同一棵树构建两次发布二进制并比对字节，夜间作业跑它。两台机器构建同一棵树仍会记下不同的源码路径，去掉它们需要一个本项目钉住的工具链不提供的编译开关。 |
| 把花费归到 skill | 工具调用并不发生在某个 skill「之下」：skill 是 prefix 里的一行披露，不是调用上下文。成本页保留按 skill 这一刀，每次调用都落进 `no_skill` 这一格。 |

## 在终端里

页面不是唯一的门。下面每一个命令都跟同一座城说话，[`LLM.md`](LLM.md) 为 agent 或脚本写明了这套线协议。

```bash
sprawling up [city-dir] [addr]        # 没有城就先建，然后服务，并打开页面
sprawling init <city-dir>             # 建城；城名写进创世记录
sprawling serve <city-dir> [addr]     # 服务一座已经存在的城；默认只监听回环地址
sprawling dispatch <addr> <task>      # 给一座正在服务的城派一件活，打印它的事件直到 run 结束；-m <id> 指定模型
sprawling call '<frame>'              # 发一帧线协议，把回来的每一帧打印出来；退出码就是答案
sprawling gauge -- <program> [arg...] # 把一条命令跑 --samples 次并计时；--pid 看一个进程，--at 看一座正在服务的城（别名 top）
sprawling view <city>                 # 只读地读一座城的 Ledger 行或它的 run 树
sprawling check <city>                # 读城里的每个 TOML 文件，把每个错误打印成 path:line:column
sprawling doctor [<city>] [--install] # 这台电脑有的，对照一座城需要的
sprawling enrol <realm>/<name>        # 从 stdin 读一个凭据，交给一座城
sprawling resume <city-dir>           # 崩溃之后：验链、关掉丢了结果的工具调用、报告谁在等你
sprawling fork <city> <run> <seq> <addr>  # 从一次 run 的某一步分出一条支线
sprawling adopt <city> <addr>         # 把城里已有的一个目录收为一栋楼
sprawling whose <city> <commit>       # 这座城做的某个提交是哪一次 run 写的，从 Ledger 回答
sprawling replay <ledger-dir>         # 离线验链，只读
sprawling export <city> <bundle-dir>  # 打包整座城；`restore` 在另一台机器上解开
sprawling install [--uninstall]       # 把 `sprawling` 放上 PATH，或撤下来
sprawling status [--check]            # 这份二进制；--check 去 npm 问有没有更新的版本
sprawling help [<verb>]               # 全部命令，或解释其中一个
```

`up --supervise` 在子进程里服务这座城，崩溃之后自动 resume 再服务，直到崩溃来得太密、不值得再试。没有东西会自动更新：`sprawling status --check`，或**设置**里的那个按钮，告诉你有没有新版本发布，替换二进制始终由你来做。

## git 里的历史

城通常在一个属于你的 git 仓库里干活，它让你的历史保持你留下的样子。每一波工具调用之前，它把现状提交一次，所以任何消失的东西都有提交可以找回；但这些检查点提交没有任何人的 `HEAD` 指着，保存在 `refs/sprawling/runs/` 下。只有两件事会移动分支：一个原本没有提交的仓库的第一个提交，以及落下一件审过的活的那次合并。

城做的每个提交都写明是哪位居民做的，并带着 `git interpret-trailers --parse` 不用帮忙就读得懂的 git trailer：

```
Sprawling-Run: <run id>
Sprawling-Actor: lab/parser
Sprawling-Model: <model id>
Sprawling-Effort: high
Sprawling-City: <city hash>
```

trailer 是给城外读者的 projection；trailer 与 Ledger 不一致时，错的是 trailer。这个问题也能反着问：`sprawling whose <city> <commit>` 从 Ledger 回答某个提交是哪次 run 写的，所以一座被还原到别处、旁边没有 `.git` 的城照样答得出。

## 它在哪里监听，凭据放在哪里

**默认只监听回环地址。** 要让同一网络里的另一台机器连进来，就绑定一个非回环地址。这样的地址总要一把配对钥匙：设了 `SPRAWLING_PAIRING_TOKEN`，城就采用它；没设，城为这一次服务现铸一把，并打印出带着它的地址。没有钥匙，端口不会打开。本仓库不附带隧道或中继，因为每一种都有自己的信任模型，替你选一种就是替你做一个安全决定。

**凭据明文不进任何文件、任何事件、任何日志。** key 进操作系统的凭据库，配置里只留 `secret:realm/name`。模型说的话在成为 Ledger 载荷之前要过一遍密钥扫描，日志过同一遍扫描，为 git 检查点暂存的内容在提交之前也要扫，所以模型碰巧复述出来的 key 不会变成永久历史。

**一切都留在你的机器上。** 没有账号，没有遥测，没有托管服务。**保密楼**在任何一次远程 provider 调用之前就把 run 停下，也不起任何外部工具服务，配上本地模型就能处理隐私数据。

## 哪些部件可以换

所有对外的部分都在一条缝上，换掉它不碰其余部分：

| 部件 | 住在哪里 | 怎么换 |
|---|---|---|
| 模型端点与兼容格式 | `gateway::endpoint`、`gateway::dialect` | 在设置页填 base URL 与兼容格式。跑在同一台电脑上的模型默认直连、不走系统代理，有一个设置可以改。 |
| SaaS 与外部工具（[Composio](https://composio.dev) 是众多 MCP server 之一） | `agent_protocols::mcp` 的 `Outbound` 缝及其 stdio、HTTP、SSE 适配器；楼的 `CONFIG.toml` | 改一个 URL 或一条命令就换一个 server；保密楼一个也不起。 |
| 沙箱 | `runtime::sandbox` 缝（今天是带燃料预算的 wasmtime）；宿主命令由 `runtime::tools` 约束 | 实现这条缝并通过它的一致性测试。 |
| 客户端 | `sprawling-wire` crate 是全部 API | 对着这套线协议写第二个客户端；[`LLM.md`](LLM.md) 是同一个面，写给 agent 看。 |

每个部件住在哪、怎么换，见 [`ARCHITECTURE.md`](ARCHITECTURE.md)。

## 为什么做它

我换过很多 Harness，有些理念落后，有些超出实际。就以 RSI 来说，在 LLM 本身脱离无状态之前，Harness 能做的只是不断针对最新的模型做适配，并学习公司现有的业务流程、更快地运行它们；前者的趋势是消融实验，后者需要隐私。

越来越多的小公司是用大量 Agent 开发在线服务的小团队，其中大多数就是一堆 Markdown 加几个天才。我想要一个跟得上多 Agent 工作、又能务实对待 RSI 与记忆风潮的 Harness：务实的可扩展、节省人的精力、Agent 规模化时的成本控制、隐私与可靠、长时运行，这些放在设计的核心，再结合一些城市学与社会学的内容。

Agent 越强，人的注意力就越贵，sprawling 不打算成为又一个抢你注意力的应用。用它最好的方式是从写 Prompt 转向设计 Loop：排好工作流，让 Agent 接管固定工作，你偶尔回来看一眼跑得怎样。你的文件、代码、文档库就是记忆；在无状态的模型上由 Harness 注入「记忆」，大多只会拖累模型。

诚实地说，目前还没有哪种多 Agent 方案带来的提升对得起规模化的成本，而研究 Agent 集群里模型如何交互、协作、表现出社会性，才刚刚开始。除了业务需要的 skill、MCP 与 ACP 部件，建议先让一座城保持精简，遇到具体问题再加东西：同一个模型换一个 Harness，行为就完全不同。想用自己喜欢的 Harness，可以试试 RefRain。

**优点**：体积小；概念好玩；从一开始就为许多 Agent 设计，而不是一个 Agent 挂一圈扩展。**缺点**：没有实验室或赞助方支持的学生项目；页面离它想成为的样子还有距离；稳定性与可用性都还要调。

## 姊妹仓库：[kusanagi](https://github.com/2youg1/kusanagi)

在一座城里，历史是一条链。每个效果先成为唯一一本只追加 Ledger 上的一条事件，而唯一的全序让一些普通的问题答得出来：谁认领了这件活、谁的改动撞了、哪个目标优先、这条消息送到了没有、我读过之后有没有人写过。在机器之间，这个全序恰恰是不能有的东西。`kusanagi` 是一个去中心化的 Agent 协作网络，每一对之间一条链，每个地址都经过派生，使承载它的主机无法把同一段对话的两次投递关联起来。城内一条链，城与城之间每对一条链：两个仓库合起来，是「Agent 怎样保有一段可以信任的历史」这个问题的一个答案。

## 文档

除了这一页和上手指南，文档都是英文。

| 你想 | 读 |
|---|---|
| 知道这是什么 | 这一页，然后是 [`docs/glossary.md`](docs/glossary.md) |
| 让它干活 | [`docs/getting-started.zh-CN.md`](docs/getting-started.zh-CN.md)，然后是 [`docs/operating.md`](docs/operating.md) |
| 用脚本或另一个 agent 驱动一座城 | [`LLM.md`](LLM.md) |
| 改它 | [`ARCHITECTURE.md`](ARCHITECTURE.md)、[`AGENTS.md`](AGENTS.md)、[`docs/CONTRIBUTING.md`](docs/CONTRIBUTING.md) |
| 改一个屏 | [`docs/frontend-method.md`](docs/frontend-method.md) |

另有：[`CHANGELOG.md`](CHANGELOG.md)（每一版改了什么）、[`SECURITY.md`](SECURITY.md)（怎样报告漏洞）、[`docs/logging.md`](docs/logging.md)（为什么日志不是历史）、[`docs/third-party.md`](docs/third-party.md)（站在谁的肩膀上，以及许可义务）。[`docs/City.md`](docs/City.md) 与 [`docs/templates/`](docs/templates/) 是城写进各栋楼的文档：agent 读它们，你也可以读。

## 参与开发

从 [`AGENTS.md`](AGENTS.md) 开始。简短版：

```bash
cargo install just --locked
just prereqs        # 这个回路需要的其余工具，缺哪个就给出哪个的安装命令
just check          # 绿了，一次改动就算完成
```

PR 描述、issue、评审意见都可以用你自己的语言写。能的话附一份对照译文——母语不是英文就附英文，是中文就附中文：两种语言并排，人和 Agent 都读得更快，误译也看得见，而不是悄无声息。其余见 [`docs/CONTRIBUTING.md`](docs/CONTRIBUTING.md)。

## 站在别人的肩膀上

按主机名接上一家厂商，要知道它的 API 挂在哪条路径下、说哪种兼容格式。这些是事实；厂商自己的客户端写得比文档更准时，跟随的是那个客户端，而不是抄它的代码：读的是哪个仓库、仓库里哪条路径、哪个提交，只写在一个地方：[`docs/third-party.md`](docs/third-party.md) §1，每天有一个工作流解析它，去问每个上游有没有变。不跟随任何厂商的登录。这座城不登录任何订阅，只收 API key，凭据托管在这里实现。

浏览器页面站在同一类东西上。它的运行时依赖恰为 `tools/xtask/src/npm.rs` 的 `RUNTIME` 所列——Svelte、Effect 与 `@lezer` 语法高亮器，页面第一次显示代码时才下载后者；其中没有组件库，`client/src/views/parts/` 里的每一个控件都是本仓库自己的。从 W3C 的 ARIA Authoring Practices 以及 Kobalte 与 Ark UI 的文档里取来的，是以文字发表的行为：一个控件实现哪种模式、每个键做什么、关闭时焦点回到哪里。它们的代码一行都不在这棵树里，所以不欠什么；读它们得出的键盘表写在 [`client/client-SPEC.md`](client/client-SPEC.md)。

[`skills/`](skills/) 下的 skill 站在更早的工作上，并且写明了。其中三个——`sdd`、`tutor`、`translation`——是我此前以 AGPL-3.0-or-later 开源发布的中文 skill 的英文翻译与改编（translation 这一件的原署名还有 Claude Fable 5）；在这里它们与这棵树其余部分一样是 MPL-2.0。另外三个——`why`、`how`、`blast-radius`——是我对 [pstack](https://github.com/cursor/plugins/tree/main/pstack)（Lauren Tan (poteto)，MIT）的修改改编；它们保留原许可，每个文件都写明由我修改。`authority-review` 是对同一个 `cursor/plugins` 树里 Thermos 的修改改编（MIT）：它的第二遍被改写成我为这座城所依据的配置写的「一个事实一个权威」审查。`skills/LICENSES.md` 随这个目录走，也在发布归档里。这一段是致谢；条款见 [`docs/third-party.md`](docs/third-party.md) §5。

连接外部应用同样外包：城对任何 MCP server 说 MCP，Composio 是其中之一。本仓库不带任何人的 key，不替任何人付钱，也不当任何代理。代码依赖的许可由 `cargo deny` 逐个检查，允许清单是 [`deny.toml`](deny.toml)。

## 许可

MPL-2.0，见 [`LICENSE`](LICENSE)。[`skills/`](skills/) 下，我的三个 skill 与这棵树其余部分一样是 MPL-2.0，对 `cursor/plugins` 的四个改编保留 MIT；七个都随 `skills/LICENSES.md` 进发布归档。条款与致谢：[`docs/third-party.md`](docs/third-party.md) §5。

---

问题、bug 报告、不同意见都欢迎：开一个 issue，或者写信到我主页上的地址。
