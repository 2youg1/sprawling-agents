-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-! # read_ffi 的规格

`sprawling-read-ffi`（库名 `read_ffi`，目录 `crates/read_ffi`）是一次 open
回答四个事实的叶子：一个 Zig 函数（`crates/read_ffi/zig/window.zig`）打开一个
文件，在同一个句柄上取它的真实位置、大小、写时刻，再读调用方要的那段字节。
Rust 一侧借出一个对齐的缓冲与一个 UTF-16 路径，读回这些事实。

**它不判任何政策。** 判定「这个地址能不能读」只有一处权威——`runtime` 的
`tools::chosen_path`（`crates/runtime/spec/Tools/ChosenPath.lean` §8-30-1）。
叶子交回它打开的那个文件的真实位置，`runtime` 拿它与 `land` 判过的位置比对，
不同即拒（`crates/runtime/Spec.lean` D13）。事实可以搬到这里，政策不搬。

这个 crate 只有搬运，没有状态机：每次调用开一个句柄、用到结束、关掉。因此这里
没有需要归纳的定理，规格写的是**边界**与**零碎字节的处理**，由
`read_ffi::tests` 与 Zig 侧的单测守住。

平台的取舍：`sprawling` 优先用标准库或安全封装的接口，其次才是 Zig 叶子
（AGENTS.md *Rust* 一节）。这里走叶子，因为标准库拿不到「这个句柄的真实路径」：
`std::fs::canonicalize` 与 `same-file` 的 `Handle::from_path` 各自要再开一次文件，
一次窗口读因此在安全路径上要 4 次 open 与 2 次真实位置解析，实测 358 µs；
一次 open 加一次句柄查询是 88 µs（400 样本交错的原型）。差别在 open 的次数，
不在别的任何地方。
-/

/-! ## 1 需求分解

- **打开**（`zig/window.zig` 的 `CreateFileW`）：读权限、共享读写、只开已有；
  直通读时加 `FILE_FLAG_NO_BUFFERING`，并加 `FILE_FLAG_BACKUP_SEMANTICS`，
  这样一个目录也开得成——调用方要的正是「它是目录」这个答案。
- **事实**（`GetFileInformationByHandle`）：大小、写时刻、是否目录，都取自
  打开的那个句柄，不是取自路径上第二次查询。
- **真实位置**（`GetFinalPathNameByHandleW`）：句柄背后的真实路径。
- **窗口**（`SetFilePointerEx` 加 `ReadFile`）：从对齐的起点读一次，交给调用方。
- **Rust 面**（`read_ffi::read`）：一个安全函数，函数里恰好一个 `unsafe` 块，
  把叶子交回的状态翻成 [`Refused`] 或 [`Window`]。
-/

/-! ## 2 边界

| 情形 | 叶子的答案 | 调用方看到 |
|---|---|---|
| 文件不在 | `missing` | `Refused::Missing` |
| 路径是目录 | `directory` | `Refused::Directory` |
| 打不开（权限、被占用、设备错误） | `unreadable` | `Refused::Unreadable` |
| 打开后读不出 | `read_failed` | `Refused::ReadFailed` |
| 真实位置长过叶子的缓冲 | `path_too_long` | `Refused::PathTooLong` |
| 起点已在文件末尾之后 | `ok`，`written = 0` | 空窗口，调用方据大小作答 |
| 本平台没有叶子 | — | `Refused::Unsupported`，调用方用安全门 |

**短读即文件末尾**：叶子交回的 `written` 是 `min(实读字节, 大小 − 起点)`，
所以一个短于请求量的答案只可能来自文件自己结束。调用方据 `written` 与请求量
之比得出「读到末尾」这件事，不再问盘一次。

**目录也开得成**，是这个叶子与 `CreateFileW` 的默认行为唯一一处刻意的不同：
不要求「是不是目录」多付一次元数据调用，因为判定成功的路径上不该有这一笔。
-/

/-! ## 3 直通读与对齐

`FILE_FLAG_NO_BUFFERING` 要求缓冲地址、起点与字节数都是扇区的整数倍。调用方
（`runtime::tools::read::window`）借出一个按 4096 对齐的缓冲、按 4096 取整
起点与请求量，并据句柄交回的大小把越过文件末尾的那几个字节剪掉。

**为什么直通**：一次窗口读的字节多半不再被读第二遍，而填充页缓存要付出与文件
大小同阶的时间，还要把文件留在内存里（`crates/runtime/Spec.lean` D11、D12）。

**扇区取 4096**：这台机器的物理扇区，也是当前每个文件系统都接受的对齐。叶子
遇到直通打开失败会退回缓冲打开——一个扇区更大的卷因此仍然可用，只是失去直通
的收益。
-/

/-! ## 4 写时刻

叶子的 `Drawn.modified_nanos` 是 Unix 纪元起的纳秒数，由 `FILETIME`（1601 年起
的 100 纳秒步长）换算而来。**远未来**的写时刻会溢出 64 位纳秒，此时饱和到
`u64` 最大值：这个值只被用来与缓存里的旧值比相等，饱和不改变比较的结果。

Rust 一侧把它换回 `SystemTime`，与安全路径上 `Metadata::modified` 得到的是同
一个时刻，所以缓存校验对两条路一致。
-/

/-! ## 5 依赖与维护

- **只依赖 Zig 标准库**：叶子没有第三方依赖，`build.rs` 用 `zig build-lib`
  一次性构建它，`zig-version` 钉住编译器版本，工作区内与其它叶子一致。
- **只在 Windows 上构建**：其它目标上 `build.rs` 直接返回，`read_ffi::read`
  答 `Refused::Unsupported`，`runtime` 走 `door_open` 的安全门。这是一条装配
  上的分支，不是第二份判定。
- **唯一的 `unsafe`**：`read` 里的一处调用，`#[expect(unsafe_code)]` 带理由，
  前置条件写在紧邻的 `SAFETY:` 行：路径以 NUL 结尾、两个缓冲活着且不小于交给
  叶子的容量、`Drawn` 由这次调用独占。
- **重开参数**：某个受支持平台上句柄查询比 `canonicalize` 更贵，或叶子在 CI
  覆盖不到的目标上装配失败。
-/
