-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# privacy::state 的磁盘投影接口
规定 `crates/sprawling/src/privacy/state.rs`。这是磁盘编码与投影说明，
执行 trace 性质由 crates.sprawling.spec.Privacy 保持，不另定义执行授权；
磁盘事件 fold 的任意 trace 对应证明尚未提供，不能把 reader 算作 coordinator 验收。

Line 是 schema 和 Event；Prepared 保存不可变 Intent，Finished 只引用 operation
和 Outcome。Intent 保存闭集 Control、definition、owner 的 SecretRef、original、modified、
recommendation 和 restore_of。RawValue 的 Absent 保留 key_existed，Present 保留
kind 与 bytes；不展开、不 trim、不 lossy decode。唯一闭集对象是 PowerShell
用户 telemetry 变量，目标路径由后续平台目录决定，不接受任意磁盘 target。

History::fold(Vec<Line>) 返回投影或 HistoryFault：schema/definition 未知、operation
重复或倒退、无效 owner 引用或跨 owner 引用、缺少 Prepared 的 Finished、交错的未结意图和越层恢复
均拒绝。恢复 Prepared 必须引用栈顶，original 等于被撤销项 modified，modified
等于被撤销项 original；Applied/Restored 与 apply/restore 匹配后才变拥有栈。
Prepared 没有 Finished 时状态是 unresolved，status 不能自动结算或推断失败。
本投影只能报告磁盘事实，不能证明当前 OS 值或授权；调用者不能据此执行写入。

History::statuses 返回 operation 与 Outcome 摘要，不输出 raw bytes 或 owner。
数据编码是 serde 的带标签闭集 enum 与 JSON byte array，未知字段拒绝；schema
与容量上界由 Rust 一处定义，未来 writer 必须在系统写入前通过同一 decoder。
验收：未知字段/坏版本、重复 id、非法 phase、越层恢复和原始字节 roundtrip。
-/

/-! D52 恢复日志只保存身份凭据引用（人的保密裁决）
注册表原值及当前值可明文保存；SID 与其他非注册表值交给现有
`gateway::credential` Vault，日志只保存 `kernel::SecretRef`。owner 使用该类型，
不重新实现其语法，也不把身份编码在引用名称中；引用相等只能验证日志一致性，
不能代替从 Vault 解析真实 OS identity 后的拥有关系检查。
磁盘 schema 由 Rust authority 升级，旧明文身份记录拒绝且原字节不变；
禁止自动迁移，因为没有经核对的 Vault 绑定就不能把旧身份当作可执行恢复授权。
当前只读入口没有 Vault 解析、身份授权或 writer；这些消费者仍是执行接口缺口。
serde 拒绝无效 SecretRef 时可能在错误文字中复述输入，HistoryFault 的公开拒绝
仅携带 JSON 解码位置及类别，不复述 owner、值、未知 enum 或未知字段；
Decode 在构造时移除输入文字，内部 Debug 与公开 AxError 都遵守此边界。
验收：明文 owner 拒绝且文件不变，旧 schema 拒绝，合法引用 roundtrip，
畸形身份/字段/enum 的诊断不泄露输入；注册表原字节的既有检查保持。
-/
