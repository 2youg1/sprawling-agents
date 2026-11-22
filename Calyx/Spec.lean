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
每次 run 以独占创建取得独立捕获目录；名字含进程号与进程内单调计数，同名既有目录保留并尝试下一候选。
Calyx 通过 `MEI_CAPTURE_CREATED=1` 把空目录交给 Mei。成功段保留 session 所有的 spilled 文件，
失败或取消段整目录回收，只有确认未写盘时才删除。
-/

/-! ## 2 验收标准

- 交互：`calyx` 在工作区打开 Mei 后端；`MEI_BIN`、同目录 sibling、或 PATH 名 `mei` 解析无头核；
  `/new`/`/n`、`/resume [jsonl]`/`/r [jsonl]`、`/quit`/`/q`、Esc 保持 Console §8-174 的会话边界；
  无参 `/resume` 恢复刚离开的非空会话。`/e`/`/effect` 按 Magpie `reasoning_levels` 循环切换下一次请求的 effort。
  横幅为 cwd；composer 显示实际或 requested model/effort、`Agent Step`、已用上下文/最大上下文及两位小数百分比；
  已用量是 `input_tokens + cache_creation_input_tokens + cache_read_input_tokens`，不是累计多轮账单。
  运行中显示 working；`mei.obs` pulse/model/route/usage 不落成 Note；图片块只显示简短图片提示，不显示 base64；Esc 在 Windows 上结束子树。
  Esc 仅清草稿或停止当前 run，空闲时不动作，绝不退出 Calyx；只有 `/quit`/`/q` 关闭终端面并回收其子树。
  任务进行中提交的普通文本是 Steer：写入该 run 的 `MEI_STEER_PATH` JSONL，不启新进程、不杀组；
  Mei 在下一波工具结果末尾消费它。斜杠命令在运行中仍被拒绝（Esc 才是停止）。
- 城：`sprawling` 经宿主 ABI 进入同一套 Zig 壳；斜杠、远程钩子、生命周期与今日 Console 合同一致。
- 无头 `mei -p`、`mei --resume` 与管道 stdin 不依赖 Calyx 进程。
- 每个交互 run 的 capture 目录只经子进程环境传入；成功时保留 spilled，失败或取消时删除整个未提交目录。
  删除只发生在本次 run 独占创建的目录上，既有同名目录永不删除；日志已写盘或回滚失败时一律保留文件。
  会话追加先在内存中预留行，再持锁检查日志基线并写盘，因此写盘成功后不再有分配失败；
  基线已被另一实例推进时拒绝旧段并要求重新 resume；写盘失败而回滚成功才算"确认未写入"，回滚失败为不确定。
  导入在测量文件长度与读取历史期间持共享锁，于是历史与 `log_len` 来自同一快照，导入后的 stale 检查仍然有效；导入返回前释放该锁。
  追加诊断可持有静态文本，完全耗尽的分配器也不会使错误路径 panic。
- 读行：实时 stderr、`--replay` 与会话导入共用一个可增长读行器（`Calyx/shell/mei_line.zig`）与同一上限
  32 MiB；超过上限是一次带 `E_MEI_LINE` 诊断的失败，不是静默停止，也不是被截断的行。
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
/-! D5 Mei 状态行：cwd、实际/requested model 与 effort、Agent Step、上下文两位小数百分比及 used/max；pulse/model/route/usage 观测吞掉不刷 Note。 -/
/-! D5a Calyx 拥有交互 session 的 spilled captures，并按 run 回收未提交目录。理由：只有 Calyx 持有正式日志的追加结果，能区分成功、确认未写入与回滚后不确定；Mei 在进程边界前无法作出最终保留决定。路径可跨 Python 进程与 resume 使用；Python 退出不等于 session 所有权结束。目录由 Calyx 独占创建，只有这个创建事实授权失败、取消与析构时的递归删除；回滚失败的追加视为"可能已写盘"，不删除任何捕获文件。被否：由 Mei 在解释器退出时决定交互 session 的最终保留（看不到其后的日志追加结果），以及只凭进程号、时间和计数生成的路径删除既有目录（可能删除其他 run 的内容）。 -/
/-! D6 `calyx` 二进制是壳的装配根；墙钟采样在进入壳时注入，库面仍可接收宿主提供的时刻。 -/
/-! D7 整壳 Zig；Rust 只做城/线/核宿主

决定：`Calyx/shell` 是终端壳的唯一实现（输入、生命周期、编辑器、菜单、屏面编排、按键映射、
Mei 与城交互面、启动参数与 `mei` 程序解析、Mei 子进程生命周期）。`Calyx/build.zig` 是 Calyx
下全部 Zig 的构建权威，产出 `calyx` 可执行文件与供宿主链接的静态库。帧叶仍在 `Calyx/ffi`，
壳在进程内直接调用叶，不再经 Rust 画帧。Rust 仅实现窄宿主：把 `wire`/accounting/kernel 的
记录与动作接到壳的 C ABI；既有 `unsafe` 仍只出现在 `Calyx/ffi`（及若壳静态库由该包链接时的同包
包装）的未触及旧文件中，新增与变更文件由 `xtask unsafe` 判零，迁移契约见 `Calyx/ffi/Spec.lean` D13。壳侧 C ABI 使用调用方持有缓冲与显式长度；可分配逻辑取 `Allocator` 参数。Zig 钉保持
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

/-! D9 Mei 短命令与无参恢复归 Calyx：`/n` 等价 `/new`，`/r` 无参恢复刚离开的非空会话，`/q` 等价 `/quit`；`/e` 只改后续请求，不伪造当前 route 已确认。模型切换不另做 `/m`，由 Magpie 路由拥有。Esc 只停止，不退出。 -/

/-! D11 任务进行中的普通提交是 Steer，经 `MEI_STEER_PATH` 投递给无头核

决定：Calyx 在每次 run 的 capture 目录**旁**独占创建 `{capture}.steer.jsonl`（兄弟文件，
不写入 capture 目录内），经子进程环境 `MEI_STEER_PATH` 交给 Mei；人在进程仍在时提交的
非斜杠文本追加一行 `{"text":"…"}` 并 flush，不 spawn 第二个 `mei`、不关闭当前组。斜杠在
运行中仍提示「Esc 才停止」。Esc 仍是杀组取消，与 Steer 分立。理由：核的落点是「接到最新
tool_result 末尾」（`Mei/Spec.lean` D21），面只需提供不打断的投递缝；文件 JSONL 跨平台、
可与 capture 同生命周期回收；capture 目录必须保持空才能被 `MEI_CAPTURE_CREATED` 采纳，
因此 steer 不得落在目录内。被否：①保持「运行中拒绝一切输入」——人无法改道；②经仍打开的
stdin 混写 prompt 与 steer——无头 `read_to_string` 合同与 argv `-p` 长度限制都会被撕开；
③打断当前 python 调用——取消已有指令；④steer 放进 capture 目录——破坏空目录采纳。
-/

/-! D10 实时、回放与会话导入共用一个可增长读行器，单行上限 32 MiB

决定：`Calyx/shell/mei_line.zig` 拥有「一条 Mei 消息行」的读取、上限与超限诊断；实时 stderr、
`--replay` 与 `Session.import` 都经它，整行读出后才算一条消息，超限以 `E_MEI_LINE` 结束该次运行、
回放或导入。理由：一条消息行是一个完整 tool_result，可以携带图片的 base64（单图上限 5 MiB，
base64 约 6.7 MiB），而固定的 4 KiB/64 KiB 缓冲、两个互不相干的上限与「读不全就 break」会让实时面
与回放在含图会话上静默停住，`/resume` 另有整文件 64 MiB 的第三个上限。上限 32 MiB 覆盖一次
python 调用允许的图片总量与 JSON 信封。被否：①加大固定缓冲——仍是静默悬崖，且每条行都占满；
②无上限——内存无界；③按行截断——伪造观察；④保留整文件上限——同一规则的第二份副本。
-/

/-! D11 Mei 面流式呈现：活动区草稿 + ≤360 Hz 合并提交

场景：1k+ TPS 的模型（或同等速率的 `mei.stream` 增量行、突发回放）在 Calyx CLI 上输出；
人在 360 Hz 显示器上阅读。若每个增量立刻整帧重画，终端写入积压、撕裂，键入被拖住。

决定：
1. 进行中的助手正文只进活动区 `Replying`（`Calyx/ffi` 词表），不写转写；完整答复、
   `mei.stream` 的 `commit`、或 run 结束时的未提交草稿，才写成一次 `Entry.Reply` 进回滚。
2. `Calyx/shell/mei_present.zig` 拥有呈现节拍：脏状态由事件（记录、流增量、键、粘贴、
   缩放、结束）置位；呈现至多 `MAX_HZ = 360`（最短间隔 `1_000_000_000 / 360` 纳秒）。
   键/粘贴/缩放/结束强制立即呈现。中间帧不排队：只保留最新草稿与尚未呈现的转写条目。
   入队时深拷贝条目字符串到独立 `queue_arena`，呈现前不借用 `frame_arena` 或 `Seen` 的
   可变字段（含 `model`）；呈现后才释放该副本。
3. 线型：`{"type":"mei.stream","kind":"delta","text":…}` 追加草稿；
   `{"type":"mei.stream","kind":"commit"}` 把草稿提交为 `Reply`。完整
   `role`/`content` assistant 行仍按既有投影；若当时有草稿则先丢弃草稿再投影完整正文。
4. 会话 JSONL 只收完整消息行；`mei.stream` 不进 `pending` 追加。
5. 本决定只约束 Calyx CLI（Mei 面）；不改 RefRain；不强制 Mei 核改 D2 非流式 HTTP——
   核仍可发完整消息，壳侧节拍对突发完整行与增量行同样生效。

理由：显示刷新是观察上限；超过 360 Hz 的帧对人不可见却占用写入与排版。草稿进活动区
守住「转写只写一次」（`Calyx/ffi/Spec.lean`）。节拍与脏位在壳，叶子仍无时钟（D1）。

被否：①每行一帧——高 TPS 下必然积压；②把增量写入转写——污染回滚且与 write-once 冲突；
③用动画/跳秒制造「在动」——违背「不发明运动」；④无上限帧队列——延迟无界；
⑤在 RefRain 或 Mei 核内做帧合并——锁定范围是 Calyx CLI。

验收：`mei_present` 的节拍与深拷贝单测（源 `model`/临时缓冲释放后副本仍可读）；
`mei_observation` 对 delta/commit 与完整行的投影；含 `Replying` 的叶/vt100 帧；
`zig build test`（`Calyx/`）与受影响的 `console_ffi` 测试绿色。
-/
