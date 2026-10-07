-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# privacy::state 的磁盘投影接口
规定 `crates/sprawling/src/privacy/state.rs`。这是磁盘编码与投影说明，不是形式证明：
执行轨迹的性质由 crates.sprawling.spec.Privacy 保持，fold 是该模型 journal 与拥有栈的磁盘投影，
不另定义执行授权。

磁盘 schema 3。Line 是 schema 和 Event；Event 是三种之一：
- Prepared{intent}：不可变 Intent，字段为 operation、control（闭集 PrivacyControl）、owner 的
  SecretRef、original 与 modified（Snapshot）、key_existed、restore_of。
- Finished{operation, outcome}：outcome 为 Applied、NotApplied、Restored、RolledBack、Unknown。
  Unknown 表示回滚后仍读不到原值，它不结束操作。
- Reconciled{operation, settlement}：人的核对给未结操作的结论，settlement 为 Applied、
  NotApplied、Restored、Abandoned（模型 Reconcile 与 reconciled）。
Snapshot 是 Registry(RawValue) 或 Task(TaskState)；RawValue 的 Absent 不带字段，Present 保留
kind 与 bytes；不展开、不 trim、不 lossy decode。

History::fold(Vec<Line>) 返回投影或 HistoryFault。拒绝：未知 schema、未知字段或 enum、
operation 重复或倒退、owner 引用无效或跨 owner、original 等于 modified、缺少 Prepared 的
Finished 或 Reconciled、对已结束操作的第二个结论、有未结操作时的新 Prepared（Privacy D63）、
越层恢复。拥有栈按控制分开：恢复 Prepared 必须引用同一控制的栈顶，original 等于被撤销项的
modified，modified 等于被撤销项的 original；Applied 与 apply 匹配后压栈，Restored 与 restore
匹配后弹栈，NotApplied、RolledBack、Abandoned 不改变拥有栈。fold 不查控制表（D62）。
Prepared 之后没有结论、或结论是 Unknown 时，该操作 unresolved：status 报告 unresolved，
coordinator 拒绝所有控制的新写入，只有 Reconciled 能结束它；结论为 Unknown 的操作不再接受
第二个 Finished。status 对每个操作报告 unresolved、finished(outcome) 或 reconciled(settlement)。
本投影只报告磁盘事实，不证明当前 OS 值或授权；调用者不能据此执行写入。

History::disclose(authorize) 是摘要唯一的出口：非空历史先把 owner 引用交给 authorize，
接受后才返回 operation 与结论摘要；空历史没有 owner，不询问 authorize，返回空摘要。
摘要与 owner 引用存在同一个值里，没有 owner 的非空摘要不可表示。摘要不输出 owner；
status 的 authorize 见 Privacy.Cli D54。
History::holdings(authorize) 是 coordinator 的出口：authorize 收到记录的 owner 引用（空历史为无），
返回此后写入 Intent 的 owner 引用——非空历史核对后原样返回，空历史由 Host 为实时身份建立绑定；
接受后才交出 Holdings：每个控制的拥有栈栈顶、未结操作（Prepared 之后没有结论或结论为 Unknown）
与下一个操作编号（最新编号加一，越界即拒绝）。所以身份不符时 coordinator 不看到任何历史值
（Privacy D69）。
Holdings 对 authorize 返回的 owner 类型泛型。只读者（inspect、restore-all 选控制）以 () 作 owner：
非空历史的 authorize 核对记录的引用，空历史的 authorize 收到无、不访问 Vault 也不建立绑定，
交出空的拥有关系。只读者的 Holdings 不带 owner 引用，plan 只接受带 SecretRef 的 Holdings，
所以只读者无法据此准备写入。
数据编码是 serde 的带标签闭集 enum 与 JSON byte array，未知字段拒绝；schema 与容量上界由 Rust
一处定义，writer 在系统写入前通过同一 decoder。
验收：未知字段/坏版本、重复 id、非法结论、越层恢复、跨控制恢复被拒、同控制栈顶恢复、
RolledBack 与 Abandoned 不取得拥有、原始字节 roundtrip。
-/

/-! D52 恢复日志只保存身份凭据引用（人的决定）
注册表原值及当前值可明文保存；SID 与其他非注册表值交给现有
`gateway::credential` Vault，日志只保存 `kernel::SecretRef`。owner 使用该类型，
不重新实现其语法，也不把身份编码在引用名称中；引用相等只能验证日志一致性，
不能代替从 Vault 解析真实 OS identity 后的拥有关系检查。
旧明文身份记录拒绝且原字节不变；禁止自动迁移，因为没有经核对的 Vault 绑定就不能把旧身份
当作可执行恢复授权。status 入口经 History::disclose 通过 Vault 核对身份（Privacy.Cli D54）；
writer 首次写入时建立同一绑定。
serde 拒绝无效 SecretRef 时可能在错误文字中复述输入，HistoryFault 的公开拒绝
仅携带 JSON 解码位置及类别，不复述 owner、值、未知 enum 或未知字段；
Decode 在构造时移除输入文字，内部 Debug 与公开 AxError 都遵守此边界。
验收：明文 owner 拒绝且文件不变，旧 schema 拒绝，合法引用 roundtrip，
畸形身份/字段/enum 的诊断不泄露输入；注册表原字节的既有检查保持。
-/

/-! D61 键是否存在是 Intent 的记录，不是值的一部分
Intent 带 key_existed（读取 original 时父键是否存在）；RawValue::Absent 不带字段，所以快照相等
就是模型的值相等，plan、读回判定与 fold 的撤销检查都用它。恢复删除本值而保留父键：若键的存在性
算进值，apply 时新建了父键的控制在恢复后读回 Absent{key_existed: true}，不等于原值
Absent{key_existed: false}，就会触发一次错误的回滚。key_existed 只供报告残留的空键。
被否：把 key_existed 留在 Absent 里并另写一个忽略它的比较——同一类型上有两种相等，derive 出的
那一种会在某个调用点被默默用错。
-/

/-! D62 schema 3 的 Intent 只记录写了什么，不记录控制表为什么这样写
Intent 不再保存 definition 与 recommendation：控制表在 Prepared 之前由 plan 判定一次，恢复只需要
记录下来的 original。fold 不查控制表，所以控制表修订（某控制改为不写、推荐值变化）不会使已拥有的
修改无法恢复。schema 2 不保留兼容：此前没有发布过写入器，磁盘上不存在 schema 2 写入的修改记录；
遇到其他 schema 按未知 schema 拒绝，原字节不变。
被否：保留 definition 并在 fold 时拒绝未知 definition——控制表每次修订都会让旧历史整体不可读，
人就失去恢复的路径。
-/
