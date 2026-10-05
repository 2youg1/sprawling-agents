-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# privacy::cli 的只读入口
规定 `crates/sprawling/src/privacy/cli.rs`。本文件是接口说明，不是形式证明。

pub fn status() -> Result<String, AxError> 通过 Home::detect().privacy_history()
调用 journal::read 后序列化 History::statuses；没有 city 参数，没有网络或 wire。
main::verbs 的唯一 spelling 为 privacy status，Effect::ReadsOnly；main::router
直接调用 status，输出一行 JSON，失败输出 AxError 与 recovery 并退出失败。
纯状态摘要不包含 owner、raw bytes、绝对 home 路径；unresolved 不是失败或成功。
HistoryFault 的稳定 code/action/recovery 映射归 state，不在路由器重写。

当前入口不检查实时 OS，也不产生确认或修改。inspect/confirm/apply/restore 尚未
接入，读取已有 history 不能冒充首个控制完成；未来入口仍走同一个 coordinator。
-/
