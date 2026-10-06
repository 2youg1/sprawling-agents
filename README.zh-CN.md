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

sprawling 不会自动更新；需要时在设置中检查版本，并按[更新指南](docs/getting-started.zh-CN.md#更新)操作。

## 五项能力

**长程任务与自动化。** 计划、决定和交接保存在可读文档中，让 Agent 跨会话接着推进；分层计划协调大规模任务，角色、skills 和工具接入让你定义工作流。常驻目标派出就绪的计划节点，并等待进行中的 run；怎样引导、暂停和停止工作见[日常操作](docs/operating.md)。

**社会模拟。** Agent 可以寻找彼此、交换消息、协调任务和等待回复，无需你逐次转述；主 Agent 解释工作并汇报进展，记录下来的通信让你观察群体如何互动，[playback](crates/city/skills/playback/SKILL.md) 可以导出历史并对照 Ledger 检查。

**容易上手。** 对话、skills 与工具接入沿用其他 Agent 中熟悉的方式。[上手指南](docs/getting-started.zh-CN.md)分别提供已有 Agent 用户迁移配置与 Chat 用户首次工作的路线，覆盖首次任务、查看报告和停止工作。

**内置监视器。** 工作进行时查看 run 耗时、模型调用、token、成本与资源读数；结合监视器和 `sprawling gauge` 为自己的负载建立性能 Eval，在自己的设备上比较结果。[性能文档](docs/performance.md)说明计数器、复现方法与测量来源。

**定制与二次开发。** 用角色文档、项目规则和 skills 定义 Agent 如何工作，接入所需模型与 MCP 工具，或通过 ACP 使用支持的 harness；基于 wire 构建自己的界面，沿架构中的 seams 修改运行机制。如果工作流或 AgentOS 需要常驻项目团队、文档交接和同一台机器上的共享历史，可以用这些部件搭建；现有连接见[接入文档](docs/integrations.md)，运行机制的修改位置见[架构](ARCHITECTURE.md#8-where-to-change-what)。

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
