# *Sprawling* 🏙️

**Let agents live like citizens in the city of sprawling, where nothing is hidden: prompts are plain files you own, and every context is a line in a hash-chained, offline-verifiable Ledger you can replay.**

<p><a href="README.md">English</a> · <a href="docs/README.zh-CN.md">简体中文</a></p>

<p>
  <a href="https://crates.io/crates/sprawling"><img src="https://img.shields.io/crates/v/sprawling?color=9f5f37&amp;labelColor=3d444d&amp;style=flat-square" alt="crates.io"></a>
  <a href="https://www.npmjs.com/package/sprawling"><img src="https://img.shields.io/static/v1?label=npm&amp;message=sprawling&amp;color=a45953&amp;labelColor=3d444d&amp;style=flat-square" alt="npm"></a>
  <a href="https://docs.rs/sprawling"><img src="https://img.shields.io/docsrs/sprawling?label=docs.rs&amp;style=flat-square&amp;labelColor=3d444d" alt="docs.rs"></a> ·
  <a href="https://zread.ai/2youg1/sprawling-agents"><img src="https://img.shields.io/static/v1?label=Ask&amp;message=Zread&amp;color=00837e&amp;labelColor=3d444d&amp;style=flat-square" alt="Ask Zread"></a>
  <a href="https://deepwiki.com/2youg1/sprawling-agents"><img src="https://img.shields.io/static/v1?label=Ask&amp;message=DeepWiki&amp;color=3877a8&amp;labelColor=3d444d&amp;style=flat-square" alt="Ask DeepWiki"></a>
  <a href="LICENSE"><img src="https://img.shields.io/github/license/2youg1/sprawling-agents?color=6f6f6f&amp;labelColor=3d444d&amp;style=flat-square" alt="License"></a>
  <a href="https://github.com/2youg1/sprawling-agents/releases"><img src="https://img.shields.io/github/v/release/2youg1/sprawling-agents?include_prereleases&amp;style=flat-square&amp;labelColor=3d444d&amp;color=6f6f6f" alt="Release"></a>
</p>

<a name="what-it-does"></a>

Sprawling runs a team of collaborating agents on your computer. A workspace is a city, projects are buildings, and agents in rooms can divide work, collaborate, and report back to you. One Rust executable serves the browser workspace and terminal console and records runs in a Ledger you can verify offline.

Start with [your first city](docs/start/getting-started.md) for models, write boundaries and review rules. This source tree is at <!-- xtask:begin workspace_version -->0.18.2<!-- xtask:end -->.

<p align="center">
  <a href="docs/images/city-overview.webp"><img alt="A demo city overview with City Hall, two project buildings and completed runs" src="docs/images/city-overview.webp" width="100%"></a>
  <br><sub>The city overview follows work across two demo projects.</sub>
</p>

## Quick start

With Bun installed, run:

```sh
bun install --global sprawling@latest
sprawling up ./cities/first
```

In the browser, connect a model provider, select a model for `main`, and give the Mayor a first task. The terminal starts as the quiet host. Press Esc to enter the CLI, then type `/web` to reopen the page and return to the quiet host, or `/quit` to stop the city. The [installation guide](docs/start/getting-started.md#1-install) also covers standalone installers, prebuilt archives and release verification.

## Projects

| Project | Owns |
|---|---|
| [Sprawling](Sprawling/README.md) | The city host, command line and product Rust crates. |
| [RefRain](RefRain/README.md) | The browser workspace for files and conversations. |
| [Calyx](Calyx/README.md) | The native terminal shell shared by Sprawling and Mei. |
| [Mei](Mei/README.md) | A standalone agent harness with terminal and ACP entry points. |
| [Kasanagi](Kasanagi/README.md) | A planned remote collaboration project. |
| [Tools](Tools/README.md) | Development checks, simulation, testing and packaging tools. |

## Documentation

- [Getting started](docs/start/getting-started.md) ([简体中文](docs/start/getting-started.zh-CN.md)) and the [documentation index](docs/README.md).
- [Operating a city](docs/operate/operating.md), [integrating tools and agents](docs/integrate/integrations.md), and [architecture](docs/ARCHITECTURE.md).
- [Contribution guide](docs/CONTRIBUTING.md), [agent instructions](AGENTS.md), and [reading guide for models](LLM.md).
- <a name="sources-and-credits"></a> [Release history](docs/CHANGELOG.md), [security reporting](docs/SECURITY.md), and [credits and licences](docs/third-party.md).
- <a name="why-i-built-this"></a> [Project background](docs/reference/background.md)
- <a name="code-signing-policy"></a> [Release verification and code signing](docs/start/getting-started.md#select-and-verify-a-version)
