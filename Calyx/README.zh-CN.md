# Calyx

[English](README.md)

**读完长输出，手还在键盘上。** Calyx 是原生 Agent 控制台。 它是 [Sprawling](../Sprawling/README.zh-CN.md) 和 [Mei](../Mei/README.zh-CN.md) 的交互界面，不是通用终端模拟器。

- **读长输出，不丢位置。** 在 Mei 界面里，输入框始终贴底，转写向上滚动；改变窗口宽度时重新折行，保留阅读位置。推理和 Python 输出可以折叠，也能看 Markdown 表格、公式、图表和图片预览。
- **键盘就能操作。** 城市控制台输入斜杠命令的前缀后按 Tab 补全。Mei 中用 `/e` 选择下一轮的思考强度，Esc 能停下当前运行，不退出 Calyx。
- **界面归界面，执行归执行。** Zig 接管输入、编辑和绘制，渲染器接收场景、返回一帧；窄 Rust 接口连接城市。Mei 仍是单独的无头进程，脚本或 ACP 不必打开界面。

## 快速上手

先打开 [Magpie](https://usemagpie.ai) 并配好模型路由。已有 Bun 时，先安装 Mei：

```sh
bun install --global @sprawling/mei@latest
```

安装后打开新终端。如果找不到 `calyx`，运行 `bun pm bin --global`，并把返回的目录加入 `PATH`，再运行：

```sh
calyx
```

请在一次性工作区试用：[Mei 生成的 Python 以当前账户权限运行](../Mei/README.zh-CN.md)。用发布包或源码构建时，也直接运行 `calyx`；crates.io 的 Mei 只装无头核心。若已有 Sprawling 城市，在主机终端按 Esc，再输入 `/w` 并按 Tab，就能试试命令补全；选 `/web` 可回到同一座城的 RefRain。

<a name="布局"></a>

## 技术栈、目录与文档

| 路径 | 责任 |
|---|---|
| `shell/` | Zig 壳：交互面的唯一实现 |
| `build.zig` | Calyx 下 Zig 的构建权威（壳、叶链接、测试） |
| `ffi/`（`sprawling-console-ffi`） | Zig 叶与 scene 词表；宿主/测试用的安全 Rust `draw` |
| `src/` | 仅 Rust 城/线/核宿主，经窄 ABI 进入 Zig 壳 |
| `Spec.lean` | 壳合同与决定 |

包名仍为 `sprawling-calyx` 与 `sprawling-console-ffi`。唯一的 `calyx` 二进制是 `build.zig` 产出的 Zig 面；`Calyx/build.rs` 把它装进 cargo 目标目录以便 sibling/`CALYX_BIN` 解析。与 `mei` 一并放入 `@sprawling/mei`（npm shim：TTY 且没有 `-p`、`--acp` 或 `--resume` 时 exec `calyx` 并设置 `MEI_BIN`；否则走无头核）。

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

`just check-console` 跑帧叶 Zig 测试（`Calyx/ffi/zig/leaf.zig`）。壳覆盖是 `cd Calyx && zig build test`。用 `zig fmt` 格式化 `Calyx/ffi/zig` 与 `Calyx/shell`（见根 `justfile`）。

## 城与 Mei

| 面 | 如何启动 |
|---|---|
| Mei 交互 | `calyx`（或 TTY 上的 Mei npm shim） |
| 城控制台 | `sprawling` 服务路径，经 Rust 宿主进入同一套 Zig 壳 |
| Mei 无头 | `mei -p`、`mei --resume` 或非 TTY stdin — 不涉及 Calyx |

在 Sprawling 控制台输入 `/w` 并按 Tab 打开补全菜单。

[![输入 `/w` 再按 Tab 之后的斜杠菜单：光标在 `/web` 上，下面是 `/wire`，以及在其中移动的按键](../docs/images/console-slash-menu.webp)](../docs/images/console-slash-menu.webp)

见 [`Spec.lean`](Spec.lean) 与 [`ffi/Spec.lean`](ffi/Spec.lean)。
