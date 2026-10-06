-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# privacy::journal 的只读 history 接口
规定 `crates/sprawling/src/privacy/journal.rs`。本节是 IO 契约说明，不是 fsync 证明。
History 的行为性质引用 crates.sprawling.spec.Privacy 的任意 trace 契约。

read(path: &Path) -> Result<History, HistoryFault> 只以 File::open 打开既有文件，
无文件时返回空 history，不创建目录、lock 文件、JSONL 或补写终态。现有文件
以 try_lock_shared 取得 OS-backed 同一文件锁，WouldBlock 返回 Busy，其他错误
保留 source。读完并完成验证前持有该 File，释放通过 handle drop 完成。
未来 writer 必须对同一文件取得 exclusive lock，不能创建第二套锁路径。

读取文件以容量界加一个字节的 Read::take 判超界；完整非空 history 必须以 LF
结束；空白行、坏 JSON、未知 schema/字段、无效 UTF-8、损坏末行均拒绝，不跳过、
不截断。容量是本应用恢复日志的操作界，不能宣称注册表本身受此上限约束。
只读拒绝不抹除原字节；metadata 和内容由真实临时文件检查。
拒绝诊断不复述 JSON 内容，只报告解码位置和类别，保密契约见 Privacy.State D52。

当前没有 append_durable/first-create barrier，也没有写入 coordinator；它们
必须满足 Privacy.durablePrepared 的环境假设后才可向本接口追加真实修改。
不能将此 reader 标成耐久 journal writer 已完成。

来源：https://doc.rust-lang.org/std/fs/struct.File.html#method.try_lock_shared
该锁在 Windows 用 LockFileEx，Unix 用 flock，drop 释放；它不约束不合作的进程。
验收：缺文件零创建、读取字节不变、持 exclusive lock 时 Busy、torn tail 零修复。
-/
