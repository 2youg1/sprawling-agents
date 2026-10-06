-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# privacy::cli 的只读入口
规定 `crates/sprawling/src/privacy/cli.rs`。本文件是接口说明，不是形式证明。

pub fn status() -> Result<String, AxError> 通过 Home::detect().privacy_history()
先读取真实 Windows identity，失败时不打开日志；调用 journal::read 后通过
平台 Vault 核对每一条 Prepared 的 owner，再序列化 History::statuses。
空历史不访问 Vault，没有 city 参数，没有网络或 wire。
main::verbs 的唯一 spelling 为 privacy status，Effect::ReadsOnly；main::router
直接调用 status，输出一行 JSON，失败输出 AxError 与 recovery 并退出失败。
纯状态摘要不包含 owner、raw bytes、绝对 home 路径；unresolved 不是失败或成功。
HistoryFault 的稳定 code/action/recovery 映射归 state，不在路由器重写。

身份查询使用受保护的 Windows 安装目录下 PowerShell，关闭 profile、非交互，
复用 doctor::asking 的时间和行长度界，只保留第一行；拒绝诊断不回显 SID。
非 Windows 无对应 control，返回 ToolUnavailable，不伪造身份。
Vault 查询只 get，不运行写读删除的启动 probe，也不使用环境遮蔽。
当前入口只检查实时身份，不查询当前 registry 值，也不产生确认或修改。inspect/confirm/apply/restore 尚未
接入，读取已有 history 不能冒充首个控制完成；未来入口仍走同一个 coordinator。
-/
