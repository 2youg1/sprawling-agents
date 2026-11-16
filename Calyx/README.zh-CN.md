# Calyx

面向 [sprawling](https://github.com/2youg1/sprawling-agents) 与 [Mei](../Mei/) 的生产终端壳。人在这里键入与阅读；harness 核保持无头。

英文：[`README.md`](README.md)。

## 它是什么

Calyx 是城控制台与 Mei 进程 Face 的交互面。它拥有原始终端模式、按键与粘贴、行编辑器、斜杠菜单、面生命周期、屏面编排（含 OSC 7501 程序状态），以及无头 `mei` 的启动。帧渲染器是纯 Zig 叶：一份 scene 进，一帧终端字节出。

WebUI（未来 `RefRain/`）是另一消费者。Calyx 不实现模型环、不声明工具、不注入 AGENTS。

## 布局

| 路径 | 责任 |
|---|---|
| `shell/` | Zig 壳：交互面的唯一实现 |
| `build.zig` | Calyx 下 Zig 的构建权威（壳、叶链接、测试） |
| `ffi/`（`sprawling-console-ffi`） | Zig 叶与 scene 词表；宿主/测试用的安全 Rust `draw` |
| `src/` | 仅 Rust 城/线/核宿主，经窄 ABI 进入 Zig 壳 |
| `Spec.lean` | 壳合同与决定 |

包名仍为 `sprawling-calyx` 与 `sprawling-console-ffi`。唯一的 `calyx` 二进制是 `build.zig` 产出的 Zig 面；`Calyx/build.rs` 把它装进 cargo 目标目录以便 sibling/`CALYX_BIN` 解析。与 `mei` 一并放入 `@sprawling/mei`（npm shim：TTY 且无 `-p`/`--resume` 时 exec `calyx` 并设置 `MEI_BIN`；否则走无头核）。

## 所有权

- **Zig** 端到端拥有壳（`Calyx/Spec.lean` D7）。
- **Rust** 只保留城宿主：订阅 `wire` 记录、执行须触达 accounting/kernel/wire 的斜杠/房间动作，并桥进壳 ABI。
- **Mei** 仍是独立无头进程；脚本化 `mei -p` 与管道 stdin 不依赖 Calyx。

壳实现不得回流 `Calyx/src`；残留门禁拒绝该形状。

## 内存与 ABI

落在 C 边界上的 Zig 使用调用方持有的缓冲与显式长度。凡分配者取 `Allocator` 参数。临时工作优先固定缓冲，不用隐藏全局分配器。帧叶保持纯：不读时钟、不开文件、调用间不留状态。

## 构建与检查

Zig 钉在 `ffi/zig-version`（与 `Sprawling/desktop/ffi/zig-version` 同钉）。

```sh
cargo test -p sprawling-calyx --lib
cargo test -p sprawling-console-ffi --lib
cd Calyx && zig build test
```

`just check-console` 跑 Calyx 的 Zig 测试。用 `zig fmt` 格式化 `Calyx/ffi/zig` 与 `Calyx/shell`（见根 `justfile`）。

## 城与 Mei

| 面 | 如何启动 |
|---|---|
| Mei 交互 | `calyx`（或 TTY 上的 Mei npm shim） |
| 城控制台 | `sprawling` 服务路径，经 Rust 宿主进入同一套 Zig 壳 |
| Mei 无头 | `mei -p`、`mei --resume` 或非 TTY stdin — 不涉及 Calyx |

见 [`Spec.lean`](Spec.lean) 与 [`ffi/Spec.lean`](ffi/Spec.lean)。
