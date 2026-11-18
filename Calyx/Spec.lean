-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-! # Calyx 的规格

`Calyx/` 是生产终端壳（非 Archetype）：托住 harness 给人键入与阅读的外圈。
它同时服务 sprawling 城控制台与 Mei 无头 Face。整壳（输入、生命周期、编辑器、
菜单、屏面编排、Mei/城交互面、进程启动缝）以 Zig 为唯一实现；帧渲染叶在
`Calyx/ffi`。Rust 只保留城 / wire / kernel（及必要时的 child 宿主）适配，经窄
ABI 向壳供事件、执行壳发起的城动作。WebUI（未来 `RefRain/`）是另一消费者，
本规格不拥有它。
-/

/-! ## 1 需求分解

| 单元 | 责任 | 语言 |
|---|---|---|
| 终端壳 | 原始模式、按键/粘贴/缩放、编辑器、斜杠菜单、生命周期、屏面编排、OSC 7501、Mei 与城交互面、`--replay` 与 `mei` 解析 | Zig（`Calyx/shell`） |
| 帧叶 | scene → 一帧字节 | Zig（`Calyx/ffi`） |
| 城宿主 | 订阅 `wire` 记录、斜杠/房间动作、关闭与配对等须触达 accounting/kernel/wire 的调用 | Rust（窄 ABI） |
| Mei 核 | 无头 `mei` 进程本身 | 独立二进制；壳负责 spawn/停组 |
| 装配 | `sprawling-calyx` 链宿主；`calyx` 二进制由 `Calyx/build.zig` 产出；`@sprawling/mei` 归档携 `mei`+`calyx` | Zig + Rust 宿主 |

Calyx 不实现模型环、不声明工具、不注入 AGENTS。Mei capture 的 session owner 由 Calyx 担任：
每次 run 使用独立的捕获目录，成功段保留 session 所有的 spilled 文件，失败或取消段整目录回收。
-/

/-! ## 2 验收标准

- 交互：`calyx` 在工作区打开 Mei 后端；`MEI_BIN`、同目录 sibling、或 PATH 名 `mei` 解析无头核；
  `/new`、`/resume`、Esc 保持 Console §8-174；横幅为 cwd；composer 为 `model · usage|unknown`；
  运行中显示 working；`mei.obs` pulse/usage 不落成 Note；Esc 在 Windows 上结束子树。
- 城：`sprawling` 经宿主 ABI 进入同一套 Zig 壳；斜杠、远程钩子、生命周期与今日 Console 合同一致。
- 无头 `mei -p`、`mei --resume` 与管道 stdin 不依赖 Calyx 进程。
- 每个交互 run 的 capture 目录只经子进程环境传入；成功时保留 spilled，失败或取消时删除整个未提交目录。
- `Calyx/src` 不得再含壳实现（输入/生命周期/编辑器/菜单/屏面编排/按键/Mei 交互面）；由
  Calyx 壳残留门禁与 `zig build test`（`Calyx/`）守住。
- **兼容回归**（生产门）：Mei 交互与无头、城控制台斜杠/生命周期、帧叶 vt100 对照、生命周期 160 向量，
  在切到 Zig 壳后仍绿；删 Rust 壳模块前先有可失败的特征化/等价检查。
- **模糊测试**：对 argv/`--replay`、编辑器输入、termio 字节流、scene 写叶（含既有敌意字节）、
  城宿主→壳事件载荷，在 `ReleaseSafe` 下跑种子循环（或同等 fuzz）；结果仅为良类型事件/状态或拒绝，
  从不陷阱。
- **压力 / 内存**：编辑器与屏面重绘在可检测泄漏的分配器下循环；C ABI 缓冲越界路径只答
  BufferTooSmall；帧之间不累积泄漏。默认 `zig build test` 至少含一处 GPA/泄漏断言。
- `cargo test -p sprawling-calyx --lib`、`cargo test -p sprawling-console-ffi --lib`、
  受影响的 sprawling 装配与 Mei 测试绿色。
-/

/-! ## 3 假设与歧义

- 现代终端；Zig 钉 0.17.0（与 `Sprawling/desktop/ffi/zig-version` 同钉）。
- 不单独发布 `@sprawling/calyx` npm 根包；Mei 核进程内不链 Calyx。
-/

/-! ## 4 现状分析

Zig 壳源在 `Calyx/shell`，由 `Calyx/build.zig` 构建可执行文件与静态库；帧叶在 `Calyx/ffi`。
Rust `sprawling-calyx` 只保留城/Mei 宿主适配与对外库面（`sprawling` 的 `pub use calyx::console`
经宿主转发进 Zig）。Console 合同正文在 `Sprawling/app/spec/Console.lean`（本 Spec 引用之）。
-/

/-! ## 5 权威信源

- User ruling：顶层 `Calyx/`；生产包；服务 sprawling 与 Mei；Mei 不含壳；整棵 Calyx 迁 Zig；
  零未来债务（不得半迁、不得留下待拆脚手架）。
- `Sprawling/app/spec/Console.lean` §8-174（Mei 斜杠与会话）。
- `Mei/Spec.lean` 进程 Face。
-/

/-! ## 6–17

命名：产品名 Calyx；crate `sprawling-calyx`；二进制 `calyx`（Zig 产出）。禁止在 Mei 核内复制 TUI 类型。
-/

/-! D1 Calyx 是唯一终端壳权威；城与 Mei 后端都是它的适配。 -/
/-! D2 Mei 交互只经进程 Face spawn 无头 `mei`。 -/
/-! D3 不设 Archetype 前缀；本包为生产面。 -/
/-! D4 `@sprawling/mei` 归档与平台包同时携带无头 `mei` 与交互 `calyx`；npm shim 在 TTY 且无 `-p`／`--resume` 时 exec `calyx` 并设置 `MEI_BIN` 为同包无头核，否则 exec 无头核（`-p`、`--resume`、非 TTY stdin）。理由：一次完成要求无参 TTY 即可进入交互；无头 `--resume` 是核旗标，不得被面吞掉；「Mei 不含壳」约束的是核进程不内嵌 TUI，不是拒绝同归档并列二进制。被否：只发无头核、强制人另装 `sprawling` 并手写 `MEI_BIN`；把 Calyx 链进 `mei` 进程；TTY 上把 `--resume` 交给只认 `--replay` 的 `calyx`。 -/
/-! D5 Mei 状态行：cwd、model·usage|unknown、working；pulse/usage 观测吞掉不刷 Note。 -/
/-! D5a Calyx 拥有交互 session 的 spilled captures，并按 run 回收未提交目录。路径可跨 Python 进程与 resume 使用；Python 退出不等于 session 所有权结束。 -/
/-! D6 `calyx` 二进制是壳的装配根；墙钟采样在进入壳时注入，库面仍可接收宿主提供的时刻。 -/
/-! D7 整壳 Zig；Rust 只做城/线/核宿主

决定：`Calyx/shell` 是终端壳的唯一实现（输入、生命周期、编辑器、菜单、屏面编排、按键映射、
Mei 与城交互面、启动参数与 `mei` 程序解析、Mei 子进程生命周期）。`Calyx/build.zig` 是 Calyx
下全部 Zig 的构建权威，产出 `calyx` 可执行文件与供宿主链接的静态库。帧叶仍在 `Calyx/ffi`，
壳在进程内直接调用叶，不再经 Rust 画帧。Rust 仅实现窄宿主：把 `wire`/accounting/kernel 的
记录与动作接到壳的 C ABI；`unsafe` 仍只出现在 `Calyx/ffi`（及若壳静态库由该包链接时的同包
包装）。壳侧 C ABI 使用调用方持有缓冲与显式长度；可分配逻辑取 `Allocator` 参数。Zig 钉保持
0.17.0，除非另有 SPEC 改钉。

机器门禁：`cargo xtask residue`（或同等 Calyx 壳残留检查）拒绝 `Calyx/src` 下壳实现路径的回归
（编辑器、生命周期、按键、菜单、屏面编排、UI 主循环、Mei 交互面等）；允许的 Rust 路径仅宿主与
库根。`zig build test` 在 `Calyx/` 下覆盖壳与叶。

理由：人的立场要求整棵 Calyx 为 Zig，且禁止半迁与未来拆脚手架成本。城的 tokio/`wire` 类型留在
Rust 是宿主边界，不是壳实现。

被否：①只迁启动缝或只加深帧叶——半迁；②Rust/Zig 双实现并行——永久债务；③导出面使用隐藏全局分配器。

代价：城记录泵与斜杠动作须经 ABI；一次迁移动作面大，但完成后无第二套壳可漂移。
-/

/-! D8 console 场景条目的本地时刻经 `console_ffi::zone` 读，chrono 已出树

决定：`calyx::console::local_time::local` 把一条记录的 UTC 毫秒渲染成「本地午夜以来的秒」（`Calyx/ffi/Spec.lean` §8 的 `TimeOfDay`），经 `console_ffi::zone::seconds_since_midnight`；系统时区由 OS 按那一刻自己读，Calyx 不再依赖 chrono。

理由：RUSTSEC-2026-0543（chrono 全部版本停止维护，2026-11-17 发布）经 Calyx 的直依赖进锁，release 的 advisories 门按设计在任何暴露下保持红色；console 场景时刻是 chrono 在树里的最后一个一产业务用途，其余时间需求已由自研 UTC 时钟（`runtime::clock`）与 std 承担。

被否：①`deny.toml` 登记 ignore——清除条件在上游手里；②chrono 换 `time`/`jiff`——`runtime::clock` 已覆盖 UTC 数学，这两个只为取本地偏移而来；③「当前偏移」算术——跨夏令时边界的旧时刻差一小时，OS 按时刻换算与 chrono 旧行为等价（细节与 SAFETY 前提在 `Calyx/ffi/Spec.lean` D12）。
-/
