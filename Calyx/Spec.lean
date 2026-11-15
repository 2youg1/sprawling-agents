-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-! # Calyx 的规格

`Calyx/` 是生产终端壳（非 Archetype）：托住 harness 给人键入与阅读的外圈。
它同时服务 sprawling 城控制台与 Mei 无头 Face。Zig 帧渲染叶在 `Calyx/ffi`。
WebUI（未来 `RefRain/`）是另一消费者，本规格不拥有它。
-/

/-! ## 1 需求分解

| 单元 | 责任 |
|---|---|
| 终端写入 / 叶 | scene → 一帧字节（`Calyx/ffi`） |
| 城后端 | City console 流：斜杠、远程钩子注入、生命周期 |
| Mei 后端 | spawn 无头 `mei`、投影 stderr JSONL、`/new` `/resume`、Esc；状态行 cwd / 模型·usage|unknown / working |
| 装配 | crates：`sprawling-calyx`、`sprawling-console-ffi`；`calyx` 二进制是 Mei 交互面；`sprawling` 依赖库面；`@sprawling/mei` 归档携 `mei`+`calyx` |

Calyx 不实现模型环、不声明工具、不注入 AGENTS。
-/

/-! ## 2 验收标准

- 交互：`calyx`（或 npm shim 在 TTY 无 `-p` 时启动的同名二进制）在工作区打开 Mei 后端；
  `MEI_BIN`、同目录 `mei`、或 PATH 上的 `mei` 解析无头核；
  `/new`、`/resume`、Esc 保持 Console §8-174；横幅为 cwd；composer 为 `model · usage|unknown`；
  运行中显示 working；`mei.obs` pulse/usage 不落成 Note。
- 无头 `mei -p` 与管道 stdin 不依赖 Calyx 进程。
- `cargo test -p sprawling-calyx --lib` 与 `cargo test -p sprawling-console-ffi --lib` 绿色。
-/

/-! ## 3 假设与歧义

- Ghostty / 现代终端；Zig 钉 0.17.0。
- 不单独发布 `@sprawling/calyx` npm 根包；Mei 核进程内不链 Calyx。
-/

/-! ## 4 现状分析

帧叶在 `Calyx/ffi`（包名 `sprawling-console-ffi`）。城与 Mei 后端在 `Calyx/src/console*`。
`calyx` 二进制用 `calyx::clock::Wall` 采样并进入 `console::mei::verb`。
sprawling 经 `calyx::console` 装配城钩子。Console 合同正文仍在
`crates/sprawling/spec/Console.lean`（本 Spec 引用之，不复制全文）。
-/

/-! ## 5 权威信源

- User ruling：顶层 `Calyx/`；生产包；服务 sprawling 与 Mei；Mei 不含壳；一次完成、不留债；
  npm 以机制装配交互面（TTY → Calyx，`-p`/管道 → 无头）。
- `crates/sprawling/spec/Console.lean` §8-174（Mei 斜杠与会话）。
- `Mei/Spec.lean` 进程 Face。
-/

/-! ## 6–17

命名：产品名 Calyx；crate `sprawling-calyx`；二进制 `calyx`。禁止在 Mei 核内复制 TUI 类型。
-/

/-! D1 Calyx 是唯一终端壳权威；城与 Mei 后端都是它的适配。 -/
/-! D2 Mei 交互只经进程 Face spawn 无头 `mei`。 -/
/-! D3 不设 Archetype 前缀；本包为生产面。 -/
/-! D4 `@sprawling/mei` 归档与平台包同时携带无头 `mei` 与交互 `calyx`；npm shim 在 TTY 且无 `-p` 时 exec `calyx` 并设置 `MEI_BIN` 为同包无头核，否则 exec 无头核。理由：一次完成要求无参 TTY 即可进入交互，且「Mei 不含壳」约束的是核进程不内嵌 TUI，不是拒绝同归档并列二进制。被否：只发无头核、强制人另装 `sprawling` 并手写 `MEI_BIN`；把 Calyx 链进 `mei` 进程。 -/
/-! D5 Mei 状态行：cwd、model·usage|unknown、working；pulse/usage 观测吞掉不刷 Note。 -/
/-! D6 `calyx` 二进制是本 crate 的装配根；墙钟采样只发生在该根（`calyx::clock::Wall`），库面仍接收注入的时钟闭包。 -/
