# Sprawling

[English](README.md) · [仓库首页](../README.zh-CN.md)

**让一群 Agent 协作，不用自己当全天候调度员。** 一座城市是一个工作区，一栋楼是一个项目，房间是 Agent 工作的位置。你用自己的文件定规则；回来看时，可以追到每轮看了什么、改了什么、谁验收了改动。

## 特色

- **工作流就在文件里。** Markdown 写项目规格、任务表和交接；TOML 规定每栋楼能写哪里、可以使用哪些能力。居民可以分工、交接，不用你在几段聊天之间搬消息。
- **能回看过程，不只看最后一句话。** 查看提示词分层、工具结果、改动和验收证据。城市的 Git 提交带有运行归属；哈希链 Ledger 可以离线验证和重放，不会再调用模型或运行工具。
- **城市主机留在自己的机器上。** Rust 主机提供 RefRain、保管城市状态，无须另外部署数据库或消息队列。本地运行不等于所有请求都留在本机，接入的模型服务和外部工具仍有各自边界。

## 快速上手

已有 Bun 时：

```sh
bun install --global sprawling@latest
sprawling up ./cities/first
```

在浏览器接入模型提供方，给 `main` 选模型，然后告诉市长你要的第一个结果。主机终端按 Esc 进入 CLI；`/web` 重开网页，`/quit` 停止城市。[安装指南](../docs/start/getting-started.zh-CN.md)还涵盖 npm、安装脚本、发布包与校验。

试试给市长一个可验收的任务，例如让它把一个小项目分成两项，并说明各自怎样算完成。到项目楼看居民如何分工；工作产出城市提交后，从该提交回查运行、工具记录和验收。

## 技术栈

Rust 负责调度、权限、进程边界和记账；Markdown 与 TOML 表达工作流；Git 把变更关联到运行；Lean 规格记录各 crate 的合同。[RefRain](../RefRain/README.zh-CN.md) 是浏览器工作区，[Calyx](../Calyx/README.zh-CN.md) 是原生终端。

从仓库根目录构建：`just build-web`，再运行 `cargo run --locked -p sprawling -- up ./cities/first`。各 crate 的职责和依赖见[英文页的生成表格](README.md#the-packages)，工具链见[贡献指南](../docs/CONTRIBUTING.md#0-development-tools)。

## 文档

- [第一次建城](../docs/start/getting-started.zh-CN.md) · [日常运维](../docs/operate/operating.md) · [ACP、MCP 和脚本](../docs/integrate/integrations.md)
- [CLI 与 wire](../docs/wire.md) · [架构](../docs/ARCHITECTURE.md) · [安全边界](../docs/threat-model.md)
- [为什么做 Sprawling](background.zh-CN.md)
