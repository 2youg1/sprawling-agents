# Tools

[English](README.md) · [仓库首页](../README.zh-CN.md)

Tools 是维护者的工具台。它能列出 CI 门禁、重放确定性场景、分析性能样本和制作发布包，但不是用户运行城市的依赖。

## 快速上手

按[贡献指南](../docs/CONTRIBUTING.md#0-development-tools)配好工具链后，在仓库根目录运行：

```sh
cargo xtask gates --list
cargo test --locked -p xtask
just --list
```

只想知道一处改动该做哪些检查，从[验证指南](../docs/maintain/verification.md)开始。

## 技术栈和目录

| 入口 | 用途 |
|---|---|
| [xtask](xtask/Spec.lean) | Rust 实现的门禁、构建与发布命令，通过 Cargo 启动。 |
| [citysim](citysim/Spec.lean) | 确定性城市场景与基准。 |
| [benchstat](benchstat/README.md) | Rust 基准数据分析器，校验并导出报告使用的数据。 |
| [fixtures](fixtures/) | 测试使用的固定输入与预期结果。 |

`just` 汇总常用命令；Rust 和 Lean 的检查分工见[测试方法](../docs/maintain/testing.md)。发布边界、夹具归属与更细的源码入口见[英文说明](README.md)。

## 文档

[门禁参考](../docs/gates.md) · [验证指南](../docs/maintain/verification.md) · [性能基准方法](../docs/benchmarks/methodology.md) · [贡献指南](../docs/CONTRIBUTING.md)
