-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# privacy::journal 的 history 读写接口
规定 `crates/sprawling/src/privacy/journal.rs`。本节是 IO 契约说明，不是 fsync 证明。
History 的行为性质引用 crates.sprawling.spec.Privacy 的任意轨迹契约；writer 满足该模型
durablePrepared 的环境假设（成功返回即已持久）。

读取：read(path: &Path) -> Result<History, HistoryFault> 只以 File::open 打开既有文件，
无文件时返回空 history，不创建目录、lock 文件或 JSONL，不补写终态。现有文件
以 try_lock_shared 取得 OS-backed 同一文件锁，WouldBlock 返回 Busy，其他错误
保留 source。读完并完成验证前持有该 File，释放通过 handle drop 完成。

写入：LockedJournal::open(path) 以读与追加方式打开文件，不存在时创建它与所在目录；新建文件后同步
所在目录，目录也是新建的则再同步其上一级。Windows 上目录经带 backup semantics 与写权限打开的句柄
同步（File::sync_all 即 FlushFileBuffers），其他平台经只读打开的目录句柄同步。随后以 try_lock 对
同一文件取得 exclusive lock（WouldBlock 返回 Busy）；不创建第二套锁路径。持锁期间先读出并 fold 现有内容，
损坏末行即拒绝且不修复。LockedJournal::append_durable(line) 把一行编码为 JSON 加 LF，
先经与读取相同的 decoder 校验，再 write_all 与 sync_all，成功返回才表示该行已持久；
失败时调用者不得发起系统写入；校验拒绝或超出容量时文件不变。
history() 交出持锁时的 fold，它包含每一次成功追加的行。coordinator 在一次操作的全过程持有该锁：fold、fresh read、
Prepared、写、读回、回滚与结论（Privacy §10）。

读取文件以容量界加一个字节的 Read::take 判超界；完整非空 history 必须以 LF
结束；空白行、坏 JSON、未知 schema/字段、无效 UTF-8、损坏末行均拒绝，不跳过、
不截断。容量是本应用恢复日志的操作界，不能宣称注册表本身受此上限约束；
追加会越界时 writer 拒绝写入并报告容量。
只读拒绝不抹除原字节；metadata 和内容由真实临时文件检查。
拒绝诊断不复述 JSON 内容，只报告解码位置和类别，保密契约见 Privacy.State D52。

来源：https://doc.rust-lang.org/std/fs/struct.File.html#method.try_lock_shared
该锁在 Windows 用 LockFileEx，Unix 用 flock，drop 释放；它不约束不合作的进程。
验收（真实临时文件）：缺文件时纯读零创建、读取字节不变、读者持锁时写者 Busy、
append_durable 返回前该行已在磁盘上、损坏末行拒写且字节不变。
-/
