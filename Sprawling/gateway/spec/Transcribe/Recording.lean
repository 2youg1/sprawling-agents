-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# gateway::transcribe::recording

规定 `transcribe::recording`（`crates/gateway/src/transcribe/recording.rs`）：一段录音的字节与它的容器；放在文件里的一段录音。本文件是 `crates/gateway/Spec.lean` 的一个分部；下面每一节保留它在 gateway 规格里的标签 §8-n，别处引作 `crates/gateway/Spec.lean §8-n`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `gateway::transcribe::recording` 旁的测试守住。
-/

/-!
### 8-33 放在文件里的一段录音（`transcribe::recording` 的 `of_file_name` 与 `read_from`）

城的工具 `transcribe`（`crates/sprawling/Spec.lean` §8-131）读的是读界之内的一个文件，或连接器存进 CAS 的一个块（§8-34）；文件带着的只有名字和字节。本节是它进这个模块的那扇门；判「这条路径能不能读」是调用方的事，gateway 不认识城的地址。

- **容器从文件名的扩展名读，扩展名表不另写。** 磁盘上的文件没有 content-type，扩展名是写下它的人或程序对容器的唯一声明。`of_file_name` 拿扩展名（不分大小写）去比 `ALL` 里每一种的 `file_name()` 的扩展名，所以 `.mp3` 对 `Mpeg`，与请求体里写的 `filename` 是同一个事实。认不得就是 `E_INVALID_ARGS`，拒词列出五个扩展名；`of_media_type` 的拒词同样由 `ALL` 列出，于是「这座城发得出去哪几种」只在枚举里写一次。被否：再写一张扩展名到 `AudioType` 的表——第二张表会在第六种容器进来时少一行。
- **`read_from` 收一个 `Read`，读到 `RECORDING_MAX_BYTES` 多一字节就停。** 多出的那一字节由 `new` 拒，拒词照旧说出上限；一个几个 GB 的文件因此不会先整个进内存。上限只在本模块：调用方若自己先读全再交 `new`，它要么不设界，要么得知道上限，那就是上限的第二个权威。读失败＝`E_STORAGE_FATAL`，subject 是读者给的错误，恢复语让人换一个可读的文件。被否：`Recording::from_file(path)`——gateway 会开始自己打开路径，而一条路径是否可读（读界、reserved subtree、link）是城的判定，放进 gateway 就多出一处看路径的地方。
-/
