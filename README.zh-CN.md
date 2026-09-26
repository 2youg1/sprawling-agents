# sprawling

**在你自己的机器上，把一群 Agent 组织成一座城。一个 Rust 二进制，界面在浏览器里。**

![binary](docs/badges/release_binary.svg) ![client](docs/badges/frontend_artifact.svg)

徽章称的那个二进制就挂在[最新 release](../../releases/latest) 上；两个数都由称重产物的那道构建门算出，没有人往文档里手写体积。

> **状态：pre-alpha，研究与开发中。** 主回路是通的：在浏览器里注册一个 provider、盖一栋楼、派一件活，模型会调用工具并把文件写进那栋楼。多个 Agent 各在各的房间里开工；同一栋楼的几轮活可以同时跑，各占驾驶池的一条车道，而写进 Ledger 的只有一条记账线程。
>
> 还没做到的写在[现在能做什么](#现在能做什么还不能做什么)那一节。把真活交给它之前，请先读那一节。
>
> English: [README.md](README.md) · 给 Agent 看的：[LLM.md](LLM.md)

**优点**：体积小；概念超级潮酷；面向多 Agent，而不是一个 Agent 挂一圈扩展。

**缺点数不胜数**：不由中转站资助、也不由实验室维护的学生项目；没有二次元形象；WebUI 想做好，能力实在差点；功能的稳定性与可用性都还要调。

---

## 为什么做它

我换过很多Harness，有些理念落后，有些超出实际：就以RSI来说，在LLM本身脱离无状态之前，Harness能做的只是不断地针对最新的模型做适配和学习公司的现有业务流程并更高速地运行，前者的趋势是消融实验，后者则需要隐私。

越来越多的小规模公司正在出现，它们是有着大量Agent开发在线服务的小型团队，其中99%就是Markdown集+几个天才。

因而我想要制作一个跟上新生的多Agent（Graph engineering）又同时能务实地处理RSI和记忆相关概念风潮的Harness，我将务实的可拓展、节省用户精力、实验性的agent规模化的成本控制、隐私与可靠性、长时运行能力这些放在了设计的核心，并结合了一些城市学与社会学的内容设计了sprawling。

Agent的能力和规模越强，人的注意力就越贵，我不想sprawling成为无数希望劫持你注意力应用中的一个。sprawling区别于常规Harness聚焦于编写Prompt，最佳使用实践应是转向Loop，安排工作流，让Agent开发sprawling，接管你的固定工作……让你本人专注于新业务的设计，新技能的学习，偶尔回来看一眼跑的怎么样。

诚实地说目前还没有一种多Agent方案提升的性能对得起规模化提升的成本，但探索这项技术在自动化业务，社会模拟以及AI对齐方面的研究刚刚起步，我们还需要花费很多精力和资源探索Agent集群场景下模型的交互行为、协作效率与社会性表现。

Agent记忆的确是实现RSI很重要的途径，但不是依靠Harness做注入，你的文件、代码，文档库就是记忆，Agent真正和你一起成长的尝试在LLM脱离无状态之前大多是对模型的拖累。

如果你想要用自己喜欢的Harness可以试一下RefRain。sprawling主要面向为小团队持久化运营和学术（无论计算机还是人文社科）研究平台，目前还处于研究与开发阶段，欢迎一起开发，也欢迎和我联系/讨论。

除了迁移必要的业务skill/MCP/ACP之外，推荐暂时保持精简，在使用中遇到问题时再手动追加内容，即使是相同的模型搭配不同的Harness都会有完全不同的行为。

sprawling 面向配置一般的机器设计，所以我不会放任多Agent产生性能开销指数增长的问题，也适合部署在你的旧电脑或云电脑上。

我不卖API也买不起装你信息的硬盘，因而数据都留在本地。我设计了专门的保密楼：它在任何一次远程 provider 调用之前就把那一轮活停下，也不起任何外部工具服务，所以配上本地模型就能处理隐私数据。代价是我没法运行巨大规模的测试。

---

## 它是什么

一个二进制，一页浏览器界面，页面在构建时嵌进这个二进制。**客户端可以换**：`client/` 是 TypeScript，用 Svelte 与 Effect 写成，由 bun 安装和构建。它对着 `crates/channels` 那套 WebSocket 协议写；凡能说这套协议的都是客户端，用你与你的 Agent 写得最好的语言写即可。

磁盘上的目录树就是空间：一座 **City** 是一棵目录树，一个项目是一栋 **Building**，一个 Agent 的工位是一个 **Room**。

**一个地址同时回答三个问题。** `lab/room1` 指的是磁盘上的一个位置，这个位置决定了这个 Agent 能写哪些文件、它带着哪些文档开工、它向谁汇报。三个答案不需要任何机制维持一致，因为它们读自同一个事实。

**Agent 自己找到彼此并说上话，不需要你在中间传话。** 一轮活可以问本楼还有谁，拿回每一个它够得着的地址，以及那位住户自己的 `URBANITE.md` 写的「什么样的活该拿给我」，于是「该找谁」有了一个不靠猜的答案。对方正在干活，话从门缝塞进去，落在他下一次工具结果的末尾；对方没在干活，城市为他开一轮活。两条路上这句话都带着 `@` 与发件人的地址，那也正是回信要填的地址。**一位居民永远冒充不了你**：只有人自己的入口能造出一句以人的身份说的话，这是类型的性质，不是一条约定。

**Ledger 是唯一历史。** 任何效果先成为一条事件，再成为效果。界面上的每一个视图都是这条事件流的 projection：删掉一个，从 Ledger 重建，字节一致。日志被改动一个字节，验链会报出那一行并拒绝往下走。

**删除自带回退路径。** 表示「丢弃一个文件」的类型没有不带 Restoration 的构造函数，所以「删了回不来」不是运行时被拒绝，而是根本写不出来。回收站里每一行都带着自己的退路，点一下就把文件放回原处；如果你在丢弃之后又在那个路径上建了文件，它就不放。

**成本按五刀切开**：按 run、按居民、按 prefix 段、按工具、按 skill。每一笔是 provider 报出的计费额；provider 不报时，用价目表算出的数。provider 根本不给价格时（比如订阅），界面直接说没有价格，而不是打印 `$0.00`，并数出没有定价的调用次数与 token 数：零和未知是两件不同的事。

**界面的设计目标是别烦你。** 只有一类事会变成浏览器通知：需要你来决定的事。几轮活的进展只体现在后台标签页的标题和图标上，其余的都在你会找到的地方等着。

**每个组件都把自己的 SPEC 放在旁边。** `crates/<crate>/<crate>-SPEC.md` 写着那个 crate 的接口与它们背后的理由，先于代码存在、也先于代码改动。于是人和 Agent 改这个项目时读的是同一份文件。`specalign` 这道门拒绝变体与 SPEC 表格对不上的 kernel 枚举。

**有些状态不是被校验，而是不可表示。** 把凭证发上线路、丢弃一个文件却不留退路、把封存的凭证塞进 Ledger 载荷、没有证据就声称活已完成、给计划的一部分比它父节点更多的权重、自己验收自己的活——这些在类型系统里写不出来。测试里有 <!-- xtask:begin compile_fail_cases -->18<!-- xtask:end --> 个编译失败用例，因为「写不出来」本身就是一个需要被证明的断言。

## 跑起来

### 快速上手

1. 从 [latest release](../../releases/latest) 下载对应系统的压缩包：Windows（x86-64）、macOS（Apple 芯片）或 Linux（x86-64）。
2. 解压到任意位置。
3. 运行 **`sprawling.exe`**（Windows 直接双击）或 **`./sprawling`**（macOS 与 Linux）。它在建任何东西之前先问你一句。

这就是全部安装。不写注册表，不装服务，那个文件夹之外一字不动，删掉文件夹即全部清除。会弹出一个控制台窗口并一直开着：**那个窗口就是那座城**。浏览器会自己打开 `http://127.0.0.1:8787`；没打开就自己输这个地址。在那个窗口按 `Ctrl-C` 停城。

二进制没有代码签名，所以第一次运行会被拦：Windows 提示「已保护你的电脑」，选**更多信息 → 仍要运行**；macOS 首次拒绝，在访达里右键打开一次即可。

也可以一行命令下载并解开；脚本最后会运行 `sprawling install`，由它决定二进制放在哪里并把它加进 PATH：

```bash
curl -fsSL https://raw.githubusercontent.com/2youg1/sprawling/main/install.sh | sh        # macOS、Linux
irm https://raw.githubusercontent.com/2youg1/sprawling/main/install.ps1 | iex             # Windows PowerShell
```

**它自己不会思考，开工前你得先给它一个模型**：一把说 OpenAI 或 Anthropic 兼容格式的 provider 的 API key，或者一个订阅登录。sprawling 负责调度 Agent、记录它们做了什么并展示给你。

### 从终端

拿到那一个二进制就够了：页面就在里面，跑它不需要任何 JavaScript 运行时。从源码构建 `client/` 需要 bun。

已经装了 bun 或 node 的话，`bunx sprawling up`（或 `npx sprawling up`）从 npm 取到的就是同一个二进制。运行时负责取，不负责跑。

启动器跑的就是下面这一条，你也可以自己跑：

```bash
sprawling up [city-dir] [addr]      # 城不在就先建，然后起服，然后开 WebUI
```

拆开来写：

```bash
sprawling init  <city-dir>          # 建一座城；城名写进创世记录
sprawling serve <city-dir> [addr]   # 为一座已有的城起服；默认只听回环
# 然后打开 http://127.0.0.1:8787
```

`up --supervise` 在子进程里起服；崩溃之后先 resume 再重新起服，崩得太频繁就不再重试。

> **不要用 `cargo install` 装它。** 客户端由 [bun](https://bun.sh) 先编好再嵌进二进制；单跑 cargo build 跑不了那一步，装出来的二进制页面是空白的。要么拿 release 压缩包，要么用 `just dist` 自己构建。

页面上四步：

1. **settings**——填 provider 的 base URL、兼容格式（OpenAI 或 Anthropic）和 key。key 直接进操作系统的凭证服务，页面此后只看得到 `secret:realm/name` 这样一个引用。
2. 同一页，按角色选模型：`main` 负责思考，`digest` 替它读长文档；想把录音转成文字的话，再给 `transcribe` 选一个。
3. 盖一栋楼：在页面底部的输入框里输入 `/raise lab`。
4. 在输入框里写要产出什么、什么算完成，然后发出去。**它不问预算**：一件活跑之前没人说得出它值多少钱，订阅更是没有单价；花了多少事后从记录里报出来。**对话也不限额**：居民互相叫醒之后要谈多久，是他们自己的事。单独一轮活也没有回合上限：它跑到自己结束为止，停一轮不该再跑的活用 `/stop`，停一栋楼或整座城用 `/halt`。

页面打开时是与市长的对话，市长负责跨楼排计划。还不知道一件事该归哪栋楼时，就把想法交给市长。

其余命令：

```bash
sprawling install [--uninstall]      # 让 shell 认得 `sprawling` 这个词，或把它撤掉
sprawling doctor [<city>] [--install] [--explain <code>]
                                     # 运行中的机器具备什么、一座城需要什么；--install 逐项提供缺的东西；--explain 把一个拒绝码对到这台电脑的情况上
sprawling enrol <realm>/<name>       # 从 stdin 读一个凭证交给城；它从不出现在命令行上
sprawling dispatch <addr> <task>     # 向一座正在服务的城派一件活，打印它的事件直到那一轮活结束；-m <id> 指定模型
sprawling call '<frame>'             # 发一个线帧，打印回来的每一帧（见 LLM.md）
sprawling top                        # 看一座正在服务的城的性能监视：终端上是一屏，否则每秒一行 JSON
sprawling view <city>                # 只读地看一座城的 Ledger 行或 run 树
sprawling check <city>               # 读一座城里的每个 TOML 文件，每个错误印成 path:line:column
sprawling resume <city-dir>          # 重启之后：验链、关掉结果已丢失的工具调用、报出谁在等人
sprawling fork <city> <run> <seq> <addr>  # 从某个 Run 的某一步分叉出一条谱系
sprawling adopt <city> <addr>        # 把城里已有的一个目录收编成楼，不覆盖任何文件
sprawling replay <ledger-dir>        # 离线验链，只读
sprawling whose <city> <commit>      # 这座城做的某个 commit 是哪一轮活写的，从 Ledger 里答
sprawling export <city> <bundle-dir> # 打包整座城
sprawling restore <bundle-dir> <city> # 在另一台机器上解开
sprawling status [--deps] [--check]  # 这个二进制：版本、客户端、它由什么构建；--check 去 npm 问有没有更新的版本
sprawling help                       # 所有命令，一屏列完
```

一个命令也不带地启动它，比如双击，它只给一屏：写出它打算建在哪里，等你点头才动手。建城要写创世记录，而那件事不会因为有人双击了一个文件就发生。

从空目录到第一个 Run，一步不跳的走法在 [`docs/getting-started.zh-CN.md`](docs/getting-started.zh-CN.md)。

### 怎么知道自己是不是最新版

这里没有任何东西会自己更新，也没有任何东西会在你没开口时去查版本。每一次发布都是 pre-alpha，所以两版之间改了什么，值得在更新前读一遍。

```bash
sprawling version                   # 这是哪一版，以及它是哪天切出的
sprawling status --check            # 去 npm 问有没有更新的版本
```

设置页的 **machine** 一节有同一个检查，做成了一个按钮。两边都只把该跑的命令印出来就停：更新要替掉一个二进制，而该替哪个取决于你当初怎么装的。

```bash
bunx sprawling@latest up            # 如果你是用 npm 跑的
```

用压缩包装的，就下载新的那份，再跑一次 `sprawling install`。两条路都不会动城的目录。

## 历史

一座城在一个 git 仓库里干活，而这个仓库通常是**你的**。它留下两种历史。

**Ledger 是城自己的历史**，也是唯一的历史：任何效果先成为一行再成为别的，这条链可以离线验证，你读到的每一页都是它的 projection。

**git 是文件的恢复权威。** 每一波工具调用之前，城把现有的东西提交一次，所以任何消失的东西都有一个 commit 可以取回。这些围栏不落在你的分支上：它们是没有任何 `HEAD` 指着的 commit，由 `refs/sprawling/runs/` 下的一个引用留住。所以 `git log` 不会每波工具多一行，你自己的历史保持你离开时的样子。只有两件事会移动分支：一个原本没有 commit 的仓库的第一个 commit，以及落下一件审过的活的那次合并。

**城做的每个 commit 都说明是谁做的。** 作者是居民自己的地址，邮箱域由城推出——`lab/parser@1a2b3c4d5e6f.sprawling`，故意不可投递，因为它标识的是一座城而不是一个收信处——提交信息里带着 git trailer：

```
Sprawling-Run: <run id>
Sprawling-Actor: lab/parser
Sprawling-Model: <model id>
Sprawling-Effort: high
Sprawling-City: <city hash>
```

接替了另一轮活的那一轮再加一行 `Sprawling-Predecessor: <run id>`。这是 git 自己的 trailer 语法，所以 `git interpret-trailers --parse` 不需要我们帮忙就能读。**trailer 是给城外读者的 projection，不是第二部历史**：trailer 与 Ledger 不一致时，错的是 trailer。

**这个问题也能倒过来问。** 给一个完整的四十位 commit id，`sprawling whose` 答出是哪一轮活写的；答案出自 Ledger 而不是 git，所以一座导出后在别处恢复、旁边没有 `.git` 的城照样答得出：

```
$ sprawling whose ./mycity <commit id>
run     <run id>
actor   lab/parser (session refactor-the-ledger)
model   <model id> (effort high)
ledger  seq 4127
```

退出码 1 表示这座城没有写过那个 commit 的记录。这和「什么都没变」是两个答案，审计要的是前者。

## 五个词

| 词 | 是什么 |
|---|---|
| **City** | 一台机器上的一座城：一棵目录树、一本 Ledger、一部完整历史。两座城之间恒不互相引用。 |
| **Building** | 城里的一栋楼，一栋楼一条业务线。配置、Archive、WriteDomain 都以它为范围。 |
| **Room** | 楼里的一个房间，也就是一个子目录。一个 Agent 在一个房间里干活。 |
| **Run** | 一次有始有终的工作。**Resident 是身份，Run 才是成本。** |
| **Ledger** | 唯一历史。一行一件事，只追加，可离线验链。 |

其余词汇在 [`docs/glossary.md`](docs/glossary.md)。

## 现在能做什么，还不能做什么

**能做**：注册 provider 并选模型；盖楼、派活，模型调用工具并把文件写进那栋楼；居民自己找到彼此、说上话、互相叫醒，不需要人传一句话；给一栋楼配外部 MCP server；一座城里多个 Agent 各干各的，各有自己的 git worktree，改动要别的居民审过才能并回（自己验收自己是编译错误，不是一条规矩）；一栋楼朝着目标干活时一次拿走整个 ready set；没指定模型的一轮活沿用它所在房间开始时的模型；内存紧张时新的一轮活先等着，城所在的磁盘快满时派活在写下任何东西之前就被拒绝；停城与放行；从回收站放回一个被丢弃的文件；把一栋楼从城里移走，文件保留在保留子树下，历史保留在 Ledger 里；离线验链；导出一座城并在另一台机器上恢复。

页面上有：与任一房间的对话（默认是市长）、城、每栋楼、每轮活、记录（Ledger、归档、回收站、日志）、成本、登记簿、MCP、性能监视、设置。

**没做，以及为什么**：

| 没做的事 | 理由 |
|---|---|
| 每个平台都有 OS 级 sandbox | Agent 要跑的命令只能得到平台给得起的隔离，exec 工具的说明里写着它拿到的是哪一种、哪几样没有。Linux 上装了命名空间包装程序时，命令在自己的命名空间里跑；Windows 与 macOS 上它在工作树的副本里跑，你的文件碰不到，网络却是通的。没验证过的隔离比没有隔离更坏，因为它会被当成防线。所以今天的说法是「一次删除可以被撤回」，不是「一次删除不会发生」 |
| CI 里的浏览器端到端 | CI 用真实浏览器引擎在夹具上打开每一个定稿的界面（`cargo xtask render`），但没有哪个 CI 任务通过浏览器驱动一座活的城 |
| 跨机器字节一致的构建 | `cargo xtask repro` 在同一棵树上把发布二进制构建两次并比较字节，每晚有任务跑它。两台机器构建同一棵树，记下的源码路径仍然不同，去掉它们要一个本项目钉住的工具链没有的编译开关 |
| 把花费摊到 skill 上 | 一次工具调用不发生在某个 skill「之下」：skill 是 prefix 里的一行披露，不是调用上下文。成本页保留了按 skill 的那一刀，每一次调用都落在它的 `no_skill` 格里 |

## 你可以换掉哪些零件

我不卖 API 也不代管账号，所以外面的东西全都接在 seam 上，换掉不需要改别处：

| 零件 | 住在哪 | 怎么换 |
|---|---|---|
| 订阅登录情报（跟随 [`docs/third-party.md`](docs/third-party.md) §1 列出的四个 harness 家族） | `gateway::oauth_profiles`（只有数据零分支）、`gateway::credential`（流程与续期） | 加一行 profile。**凭证保管恒不外包**：明文只到运行中的机器的凭证服务 |
| 模型 endpoint 与兼容格式 | `gateway::endpoint`、`gateway::dialect` | 设置页里填 base URL 与兼容格式。默认情况下，跑在同一台电脑上的模型直接调用、不经代理，一个设置可以改变这一点 |
| SaaS 与外部工具（[Composio](https://composio.dev) 是其中一个 MCP server） | `protocol::mcp` 的 `Outbound` seam、`protocol::mcp::stdio`、`protocol::mcp::http` 与 `protocol::mcp::sse`、`protocol::mcp::broker` 里的 broker、楼的 `CONFIG.toml` | 改一个 URL 或一条命令就换了 server；保密楼一个都不起 |
| sandbox | `runtime::sandbox` seam（今天的适配器是带燃料预算的 wasmtime）；宿主命令的隔离在 `runtime::tools` | 实现这道 seam，过它的 conformance 断言套件 |
| 客户端 | `channels::wire` 是唯一 API 面 | 想写第二个客户端，就对着这套线格式写。[`LLM.md`](LLM.md) 是同一个面，写给 Agent 看 |

每一件的位置与替换步骤在 [`ARCHITECTURE.md`](ARCHITECTURE.md)。

## 姊妹仓库：[kusanagi](https://github.com/2youg1/kusanagi)

城内的历史是**一条链**。任何效果先成为一条事件落在唯一一本只追写的 Ledger 上，再成为效果；那一份全局全序是五个平常问题能被回答的前提：这件活谁认领的、谁的改动撞上了谁、哪个目标赢、这句话是不是已经送过、我读之后还有没有人写过。五个问的都是*谁先*，而一座把历史拆开的城就再也说不出来。

机器与机器之间，同一份全序恰恰是不能有的东西。`kusanagi` 是给 Agent 的去中心化协作网络：**一对一条链**，每一个地址都推得让同一场对话的任两条投递在承载它们的主机看来无法关联，而那台主机双方都不运营、也都不信任。这里的全局全序是一件旁观者读得到的事实。

城内单链，城际分链。两个仓库是同一个回答的两半——Agent 怎么保住一份信得过的历史；单看任一个，都像缺了另一半。

## 它听在哪里，凭证在哪里

**默认只听回环。** 要让同一网络里的另一台机器连进来，就绑一个非回环地址。这样的地址一定要配对令牌：设了 `SPRAWLING_PAIRING_TOKEN` 就用它，没设就为这次起服现造一个，并把带令牌的打开地址印出来。端口不存在没有令牌就开着的时刻。再往外，这个仓库不带隧道也不带中继：那两样各有自己的信任模型，替你选一个就是替你做安全决定。

**凭证明文不进任何文件、任何事件、任何日志。** key 进操作系统的凭证服务，配置里只留 `secret:realm/name`。模型说的话在成为 Ledger 载荷之前过一道 secret 扫描，日志过同一道扫描，要提交进 git 围栏的内容在提交前也扫一遍，所以一个被模型复述出来的 key 不会变成永久记录。

## 文档

除本页与 getting-started 外，文档是英文的。

- 刚到，想知道这是什么：读完本页即可；再深一层看 [`docs/glossary.md`](docs/glossary.md)。
- 要用它干活：[`docs/getting-started.zh-CN.md`](docs/getting-started.zh-CN.md) → [`docs/operating.md`](docs/operating.md)。
- 从外面驱动一座城的 Agent：[`LLM.md`](LLM.md)。
- 要改它：[`ARCHITECTURE.md`](ARCHITECTURE.md) → [`AGENTS.md`](AGENTS.md) → 相邻模块的代码与测试。

另有 [`CHANGELOG.md`](CHANGELOG.md)（每一版改了什么）、[`SECURITY.md`](SECURITY.md)（怎么报告漏洞）、[`docs/logging.md`](docs/logging.md)（日志为什么不是历史）、[`docs/frontend-method.md`](docs/frontend-method.md)（一个界面怎么做、怎么验收）、[`docs/third-party.md`](docs/third-party.md)（站在谁的肩上、许可义务）。[`docs/City.md`](docs/City.md) 与 [`docs/templates/`](docs/templates/) 是城写进楼里的那几份文档：Agent 读它们，你也可以读。

## 参与

先读 [`AGENTS.md`](AGENTS.md)，三十秒版：

```bash
cargo install just --locked
just prereqs                        # 回路要的其余工具，缺哪个就给出哪个的安装命令
just check
```

`just check` 绿了，一次改动才算完成。**PR 正文、issue 与评审意见可以用你的母语写**；如果愿意，附一份对照译文（母语非英文就附英文，英文就附中文）：有对照，人和 Agent 都读得更快，译错了也看得出来。其余在 [`docs/CONTRIBUTING.md`](docs/CONTRIBUTING.md)。

## 站在谁的肩上

登录一个 provider 要知道一小把端点和参数。与其自己盯着那些 API 文档，我跟随各厂商自己在维护的 harness——OpenAI 的 `codex`、Anthropic 的 agent SDK、xAI 的 `grok-build`、Moonshot 的 `kimi-cli`——这座城直接登录的四个家族，一家一个。

**跟随哪个仓库、仓库里哪条路径、已经读到哪个 commit，只有一个家：[`docs/third-party.md`](docs/third-party.md) §1。** 那张表是一条每日流水线拿去问上游动没动的数据，在这里再抄一份，就等于给一件机器已经依赖的事实添第二个家，上游一动两处就分叉。

**跟随的是情报，不是代码。** 端点和参数是事实；流程与凭证保管在这里自己实现。上游是专有许可还是 Apache-2.0，这条同样成立——四家里有一家是专有的。

**浏览器那一页站的是同一类东西。** 它的运行时依赖是 `svelte`、`effect`，以及 `@lezer` 语法高亮器和它的语法包（MIT），页面第一次显示代码时才下载；里面没有任何 UI 组件库：`client/src/views/parts/` 里的控件都是这个仓库自己写的。从 W3C 的 ARIA Authoring Practices 与 Kobalte、Ark UI 的文档里取的是写成文字的行为：一个控件实现哪个模式、每个键做什么、关闭时焦点还给谁。**他们的代码一行都没有进树，所以这件事不欠任何许可义务**；读出来的那张键盘表落在 [`client/client-SPEC.md`](client/client-SPEC.md) 里。

**[`skills/`](skills/) 下的七件 skill 站在更早的工作上，件件注明。** 其中三件——`sdd`、`tutor`、`translation`——是我用中文写作并以 AGPL-3.0-or-later 开源的 skill 的英译改编版（translation 原文的署名还记着 Claude Fable 5），在本项目里与全仓一样取 **MPL-2.0**。另外三件——`why`、`how`、`blast-radius`——是我对 [pstack](https://github.com/cursor/plugins/tree/main/pstack)（Lauren Tan (poteto)，MIT）的改编版，**保持 MIT，且件件注明改编者是我**。`authority-review` 改编自同一个 `cursor/plugins` 树里的 Thermos（MIT），第二遍审查改成了我为这座城所依据的配置写的「一个事实一个权威」审查。`skills/LICENSES.md` 随目录走，发布归档里也有。这一段就是致谢，条款在 [`docs/third-party.md`](docs/third-party.md) §5。

外部应用的连接同样外包出去：城对任意 MCP server 说 MCP，Composio 是其中之一。这个仓库不带任何人的 key、不替谁付钱、不做代理。完整清单、怎么复核、许可怎么处理在 [`docs/third-party.md`](docs/third-party.md)。代码依赖的许可由 `cargo deny` 逐个核对，白名单是 [`deny.toml`](deny.toml)。

## 许可

MPL-2.0，见 [`LICENSE`](LICENSE)。[`skills/`](skills/) 下，我自己写的三件与全仓一样取 **MPL-2.0**，`cursor/plugins` 的四件改编保持 **MIT**；七件都随发布归档与 `skills/LICENSES.md` 一起走，条款与署名见 [`docs/third-party.md`](docs/third-party.md) §5。

---

问题、bug 与不同意见都欢迎：开一个 issue，或写信到我主页上的地址。
