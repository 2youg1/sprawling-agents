<div align="center">

# sprawling

**在你自己的机器上，把许多 Agent 组织成一座城。一个 Rust 二进制，界面是浏览器里的一页。**

<p align="center">
  <a href="https://crates.io/crates/sprawling"><img alt="crates.io" src="https://img.shields.io/crates/v/sprawling?logo=rust&amp;labelColor=171717&amp;color=DEA584"></a>
  <a href="https://www.npmjs.com/package/sprawling"><img alt="npm" src="https://img.shields.io/npm/v/sprawling?logo=npm&amp;labelColor=171717&amp;color=CB3837"></a>
  <a href="LICENSE"><img alt="License" src="https://img.shields.io/github/license/2youg1/sprawling-agents?labelColor=171717&amp;color=4C8BF5"></a>
  <a href="https://github.com/2youg1/sprawling-agents/actions/workflows/ci.yml"><img alt="ci" src="https://github.com/2youg1/sprawling-agents/actions/workflows/ci.yml/badge.svg?branch=main"></a>
  <a href="https://deepwiki.com/2youg1/sprawling-agents"><img alt="Ask DeepWiki" src="https://deepwiki.com/badge.svg"></a>
</p>

</div>

给你的 Agent 一座工作的城。在 sprawling 中，项目成为一栋栋楼，Agent 在其中通信、分工，并向你报告工作进展。

计划、决定和交接保存在文件里。sprawling 依据这些记录延续跨会话工作，组织更大的任务，并执行你定义的工作流。一个 Rust 二进制在本地运行，向浏览器提供界面，并把城的历史记入只追加的 Ledger。

**状态：<!-- xtask:begin maturity:word -->alpha<!-- xtask:end -->。** 数据格式、wire 与界面在版本之间仍可能改变。

English：[README.md](README.md) · 介绍项目：[LLM.md](LLM.md) · 修改代码：[AGENTS.md](AGENTS.md)

**优点**：体积小；概念超级潮酷；面向多 Agent，而不是一个 Agent 挂一圈扩展。

**缺点数不胜数**：不由中转站资助、也不由实验室维护的学生项目；没有二次元形象；WebUI 想做好，能力实在差点；功能的稳定性与可用性都还要调。

## 快速开始

选择一种安装渠道。npm/Bun 与 cargo-binstall 下载预编译二进制，`cargo install` 在本地编译；shell 安装器不需要 JavaScript 或 Rust 工具链。

npm，需要 Node.js 与 npm：

```sh
npm install --global sprawling@latest
```

Bun，需要 Bun：

```sh
bun install --global sprawling@latest
```

crates.io，需要发布包要求的 Rust 编译器和平台原生构建工具；已装 cargo-binstall 时可用 `cargo binstall sprawling` 下载发行归档：

```sh
cargo install sprawling --locked
```

macOS/Linux shell：

```sh
curl -fsSL https://raw.githubusercontent.com/2youg1/sprawling-agents/main/install.sh | sh
```

Windows PowerShell：

```powershell
irm https://raw.githubusercontent.com/2youg1/sprawling-agents/main/install.ps1 | iex
```

预编译归档支持 Windows x86-64、Apple 芯片的 macOS 与 Linux x86-64；手动下载见[发行列表](https://github.com/2youg1/sprawling-agents/releases)，版本选择、验证与 Nix 源码构建见[安装指南](docs/getting-started.zh-CN.md#1-安装)。

所有渠道安装后使用同一条命令：

```sh
sprawling up ./cities/first
```

终端成为城的控制台并打印服务地址，浏览器打开页面。在页面接上 provider 或本地模型，给 `main` 选一个模型，然后告诉 Mayor 要完成什么、怎样算完成。Mayor 规划，楼里的居民执行；你看进展、回答提问并检查结果。控制台按 `Ctrl-C` 停城。

Agent 在自己的楼里默认拥有全部权限：用 `minimal` 模板盖的楼，Agent 能写楼内的每个文件，工作落地前不经审查。怎样打开审查、限制写入，见[一栋你自己的楼](docs/getting-started.zh-CN.md#一栋你自己的楼)。

sprawling 不会自动更新；需要时在设置中检查版本，并按[更新指南](docs/getting-started.zh-CN.md#更新)操作。

## 能做什么

**长程任务与自动化。** 计划、决定和交接保存在可读文档中，让 Agent 跨会话接着推进；分层计划协调大规模任务，角色、skills 和工具接入让你定义工作流。常驻目标派出就绪的计划节点，并等待进行中的 run；怎样引导、暂停和停止工作见[日常操作](docs/operating.md)。

**社会模拟。** Agent 可以寻找彼此、交换消息、协调任务和等待回复，无需你逐次转述；主 Agent 解释工作并汇报进展，记录下来的通信让你观察群体如何互动，[playback](crates/city/skills/playback/SKILL.md) 可以导出历史并对照 Ledger 检查。

**容易上手。** 对话、skills 与工具接入沿用其他 Agent 中熟悉的方式。[上手指南](docs/getting-started.zh-CN.md)分别提供已有 Agent 用户迁移配置与 Chat 用户首次工作的路线，覆盖首次任务、查看报告和停止工作。

**性能。** 一个进程同时提供页面并运行整座城，没有数据库，也没有另外的服务；城的历史是磁盘上只追加的 Ledger。设置 → 性能里可以选 CPU 放置方式和核心优先级，也可以给每个 run 设内存上限，这个上限只由你填写，默认没有（[城怎样使用你的硬件](docs/performance.md#choose-how-the-city-uses-the-machine)）。命令以低于城自身的优先级启动，所以一次构建不会拖慢页面。命令输出在交给模型之前，按产生它的命令裁剪，完整原文仍可取回（[sieve](crates/runtime/spec/Sieve.lean)）。监视器和 `sprawling gauge` 在你自己的硬件上显示 run 耗时、模型调用、token、成本与资源占用；[性能登记表](tools/xtask/budgets.toml)记录项目的预算和已有读数（[性能文档](docs/performance.md)）。

**隐私。** 你在消息里粘贴的 Key 会先进 Vault，模型只看到它的引用（[custody](crates/accounting/src/worker/dispatching/custody.rs)）。模型回复和工具结果里形状像密钥的值，在写入永久历史之前被替换成标记（[redact](crates/runtime/src/redact.rs)）。除了你接入的模型调用和工具，以及默认的网页搜索（关掉之前会把搜索词发给 Exa），没有东西离开你的电脑；保密楼不调用任何远程 provider。在 Windows 上，设置里提供 88 项可选的隐私控制，从诊断数据、语音输入到应用权限和 Windows AI 功能。每一项都显示当前值、它改变什么、代价是什么，逐项应用或恢复，每次写入后都读回核对（[Windows 隐私控制](docs/operating.md#windows-privacy-controls)）。隐私不等于安全，也不一定和便利冲突，但其中许多设置确实要牺牲一些便利；页面给出你逐项权衡所需的信息。

**可重放的历史。** 城做的每个决定都是 Ledger 里的一行，同样的行在任何机器上逐字节重放出同样的结果，因为决策路径把时间当参数传入、不用随机源、顺序固定（[确定性](ARCHITECTURE.md#10-determinism-and-hardening)）。重放时不会再次执行工具或调用模型，而是读回记录下来的结果。城只提供事实和边界，方法交给模型（[LLM First](ARCHITECTURE.md#llm-first-mechanism-from-the-city-method-from-the-model)）。

**定制与二次开发。** 用角色文档、项目规则和 skills 定义 Agent 如何工作，接入所需模型与 MCP 工具（一个 provider 可以按你定的顺序登记多个账号），或通过 ACP 使用支持的 harness；基于 wire 构建自己的界面，沿架构中的 seams 修改运行机制。如果工作流或 AgentOS 需要常驻项目团队、文档交接和同一台机器上的共享历史，可以用这些部件搭建；现有连接见[接入文档](docs/integrations.md)，运行机制的修改位置见[架构](ARCHITECTURE.md#8-where-to-change-what)。

## 为什么做它

我不想24/7守在电脑面前，直到5h额度撞墙再去睡觉，你也不想。

我换过很多Harness，有些理念落后，有些超出实际：就以RSI来说，在LLM本身脱离无状态之前，Harness能做的只是不断地针对最新的模型做适配和学习公司的现有业务流程并更高速地运行，前者的趋势是消融实验，后者则需要隐私。

越来越多的小规模公司正在出现，它们是有着大量Agent开发在线服务的小型团队，其中99%就是Markdown集+几个天才。

因而我想要制作一个跟上新生的多Agent（Graph engineering）又同时能务实地处理RSI和记忆相关概念风潮的Harness，我将务实的可拓展、节省用户精力、实验性的agent规模化的成本控制、隐私与可靠性、长时运行能力这些放在了设计的核心，并结合了一些城市学与社会学的内容设计了sprawling。

Agent的能力和规模越强，人的注意力就越贵，我不想sprawling成为无数希望劫持你注意力应用中的一个。sprawling区别于常规Hanress聚焦于编写Prompt，最佳使用实践应是转向Loop，安排工作流，让Agent开发sprawling，接管你的固定工作……让你本人专注于新业务的设计，新技能的学习，偶尔回来看一眼跑的怎么样。

诚实地说目前还没有一种多Agent方案提升的性能对得起规模化提升的成本，但探索这项技术在自动化业务，社会模拟以及AI对齐方面的研究刚刚起步，我们还需要花费很多精力和资源探索Agent集群场景下模型的交互行为、协作效率与社会性表现。

Agent记忆的确是实现RSI很重要的途径，但不是依靠Harness做注入，你的文件、代码，文档库就是记忆，Agent真正和你一起成长的尝试在LLM脱离无状态之前大多是对模型的拖累。

如果你想继续用自己喜欢的Harness，sprawling已经支持通过ACP接入（[接入文档](docs/integrations.md)）。我正在准备开发的kasanagi是一个辅助搭建聊天软件的项目，以后会作为MCP解决多个sprawling实例之间的远程协作。sprawling主要面向为小团队持久化运营和学术（无论计算机还是人文社科）研究平台，目前还处于研究与开发阶段，欢迎一起开发，也欢迎和我联系/讨论。

除了迁移必要的业务skill/MCP/ACP之外，推荐暂时保持精简，在使用中遇到问题时再手动追加内容，即使是相同的模型搭配不同的Harness都会有完全不同的行为。

我用的电脑不好，所以我不会放任多Agent产生性能开销指数增长的问题，也适合部署在你的旧电脑或云电脑上。

我不卖API也买不起装你信息的硬盘，因而数据都留在本地，我设计了专门的保密楼，配上本地模型完全可以用于处理隐私数据，但这也意味着我没法运行巨大规模的测试。

## 文档

除了两版 README、两版上手指南和 crate 规格的注释，其余文档使用英文。

| 文档 | 内容 |
|---|---|
| [`docs/getting-started.zh-CN.md`](docs/getting-started.zh-CN.md)（[English](docs/getting-started.md)） | 新手指南：从安装到一次审过的合并，路上遇到的每个概念 |
| [`docs/operating.md`](docs/operating.md) | 日常使用：引导和停下工作、回答居民、换 provider 与 MCP server、远程访问、出了问题怎么办 |
| [`docs/glossary.md`](docs/glossary.md) | 代码、页面与文档所用的每个词，各只有一个意思 |
| [`LLM.md`](LLM.md) | 供模型介绍 sprawling 的能力、边界与阅读路径 |
| [`docs/wire.md`](docs/wire.md) | 程序控制所用的 CLI、帧、回答与退出码 |
| [`docs/integrations.md`](docs/integrations.md) | ACP harness、MCP 工具服务与已有 Agent 的 CLI 接入 |
| [`docs/performance.md`](docs/performance.md) | 监视读数、可复现负载与测量来源 |
| [`ARCHITECTURE.md`](ARCHITECTURE.md) | 各个 crate 与依赖规则、缝、一次派活从头到尾、磁盘上有什么、怎样验证、每类改动该去哪里 |
| [`AGENTS.md`](AGENTS.md) | 每次改动要守的规则与检查它们的命令，人和 agent 都先读它 |
| [`docs/CONTRIBUTING.md`](docs/CONTRIBUTING.md) | 贡献准备、功能准入与提交流程，以及仓库规则入口 |
| [`docs/frontend-method.md`](docs/frontend-method.md) | 一个屏怎样搭建与验收，以及已批准的视觉设计 |
| [`crates/README.md`](crates/README.md) | 每个 crate 管什么、规格在哪；规格 `crates/<dir>/Spec.lean` 写着该 crate 的接口、决定与证明，注释用中文 |
| [`crates/city/templates/`](crates/city/templates/) | 城写进每栋楼的文档，agent 读它们，你也可以读 |
| [`crates/city/skills/`](crates/city/skills/) | 随发布分发的 skill |
| [`docs/logging.md`](docs/logging.md) | 什么进诊断日志、什么进 Ledger，以及两者为什么分开 |
| [`docs/third-party.md`](docs/third-party.md) | 本仓库跟随的上游事实、致谢与许可义务 |
| [`CHANGELOG.md`](CHANGELOG.md) | 每一版改了什么，以及留下了哪些已知未修的问题 |
| [`SECURITY.md`](SECURITY.md) | 怎样报告漏洞 |

## 来源与致谢

厂商接口与 harness 启动方式跟随 ACP registry 和厂商文档；需要精确核对时读取厂商客户端中的事实，不复制其代码。界面控件由本仓库实现，键盘行为参照 W3C ARIA Authoring Practices、Kobalte 与 Ark UI 文档。来源版本和完整许可见 [third-party](docs/third-party.md)。

随附的 `sdd`、`tutor`、`translation` 是作者中文 skills 的英文改编；`why`、`how`、`blast-radius` 改编自 Lauren Tan（poteto）的 [pstack](https://github.com/cursor/plugins/tree/main/pstack)，`authority-review` 改编自同一仓库的 Thermos，四者保留 MIT。每个 skill 的署名与许可见 [skills/LICENSES.md](crates/city/skills/LICENSES.md)。

报告缺陷或提出功能请求，请使用 [Issue 表单](https://github.com/2youg1/sprawling-agents/issues/new/choose)。修改代码请按 [AGENTS.md](AGENTS.md) 与 [CONTRIBUTING](docs/CONTRIBUTING.md) 进行。漏洞使用 [SECURITY.md](SECURITY.md) 的私密报告入口。


本项目采用 [MPL-2.0](LICENSE)；随附 skills 各自的许可随文件分发。
