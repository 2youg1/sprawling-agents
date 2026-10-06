-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::command::tuning

规定 `command::tuning`（`crates/wire/src/` 下同名的文件）。一个人对一个端点定下的、地址之外的规矩。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的帧形状由 Rust 的类型与 `crates/wire/tests/wire_contract.rs` 钉住的 wire schema（`wire::schema_hash`）守住。
-/

/-!
### 8-29 一个端点带着人给它定的规矩上线

```rust
pub struct EndpointTuning {
    pub accounts: Option<Vec<kernel::event::record::ProviderAccount>>, // 顺序决定优先级
    pub label: Option<String>,               // 显示名，缺省即 id
    pub timeout_ms: Option<u64>,             // 一次已结请求的期限
    pub request_max_retries: Option<u32>,    // 值得再问一次的失败再问几次
    pub stream_idle_timeout_ms: Option<u64>, // 一次流式请求的期限（命名同 Codex）
    pub headers: Vec<HeaderPair>,            // { name, value }，value 可为 `secret:` 引用
    pub overrides: Vec<BodyOverride>,        // { pointer, value }，JSON pointer → 值的文本
    pub proxying: Option<Proxying>,          // 这个端点的调用走不走这台电脑的代理，缺席即城自己的规则
    pub max_in_flight: Option<u32>,          // 同时在飞的调用至多几个；缺席与零即这一类连接的缺省
    pub account_retries: Option<AccountRetries>, // 同一账号上再发几次（one｜two，kernel §8-86）；缺席即城的缺省
}

ProbeEndpoint  { name, base_url, dialect, secret, auth_header, tuning: EndpointTuning, idem }
AttachEndpoint { name, base_url, dialect, secret, auth_header, admit, tuning: EndpointTuning, idem }
EndpointSummary { name, label, base_url, dialect, connection_kind, models, local, has_credential, tuning, account_status }   // 后两件见 §8-85
```

- **一个值而不是六个字段**。它们在同一张表单上被填，被同一次调用一起读；分开传就给了 probe 与它之后的 attach 三次机会对「我们在跟什么说话」产生分歧——`Entered` 当初收成一个值正是这个理由。
- **probe 也带 tuning**。一个需要自定义请求头的网关，在 probe 不带那个头时答 401；人于是读到「密钥无效」，而那把密钥是好的。probe 与 call 因此按同一套头、同一个期限发出。
- **覆盖的值走文本，不走 `serde_json::Value`**。`Command` 派生 `Eq`，而 JSON 没有全序相等；更要紧的是 `/temperature` → `0.2` 一旦成为值就是一个浮点，而它随 `endpoint_attached` 进账本——这座城把浮点挡在账本之外。文本原样往返，「这段文本作为 JSON 是什么」只有一个权威：`gateway::EndpointTuning::applied_overrides`，规则是「解析得出就是那个 JSON，解析不出就是它看上去的那个字符串」，于是 `/reasoning/effort` → `high` 不必要求人自己加引号。**败给的方案**：帧上直接放 `Value`——那要求 `Command` 放弃 `Eq`，并把浮点写进账本。
- **零即缺省**。清空一个数字框到达线上是 `Some(0)`，而没有请求能在 0 ms 内完成；装配层把零读成「没说」（`accounting::tuning::tuning_of`），于是清空一个框等于回到城自己的值，而不是让此后每一次调用立刻失败。
- **`stream_idle_timeout_ms` 是一个沉默上限，线上与 gateway 同名**：流式应答多久没有一个字节到达就放弃，而不是整段应答的总期限——一直在写的模型不会因为写得长被截断，写到一半停下的在最后一个字节之后这么久被放弃（`gateway::endpoint::stream` 按字节到达计时）。名字取自人在自己 `config.toml` 里写熟的 Codex 的那个词，装配层只把零读成缺席（上一条），不改名。
- **`Turn` 携 `thought`**。thinking 块为供应方的签名校验端到端携带，看起来属于传输；但对一个把大部分调用花在推理上的模型，挡住它就是让人先对着空线程等几分钟，再读到两句话。所以推理以自己的字段作答，页面把它折起来放在散文旁边，两者永不混进同一个缓冲区。`RedactedThinking` 仍然不出现：它的载荷是加密的，里面没有人能读的东西。
- **`max_in_flight` 是第八件**（gateway D21）：`Option<u32>`，缺席与零都是「没人定过」，由 gateway 读成这一类连接的缺省；1 到 `IN_FLIGHT_MAX`（256）之外由装配层以 `E_CONFIG_INVALID` 拒（与同一张表单上读作凭据的请求头同一个码），恢复语说出合法域。它随 `endpoint_attached` 的载荷进账本，旧账本里没有这把键的一行读作缺席，所以旧城重开后每个端点取缺省，而不是被拒。帧形、golden 与 `client/src/wire.ts` 随本版 `WIRE_V` 的那一次进位（D22）一起改。
- **`account_retries` 是第九件**（kernel D54）：`Option<kernel::account_recovery::AccountRetries>`，线上拼作 `"one"`／`"two"`，缺席即「没人定过」，由 gateway 读成 `EndpointTuning::DEFAULTS` 的值。它是一个枚举而不是一个数，所以线上拼不出零或三，装配层没有一个范围要拒；只在名册有两个以上账号时被读，表单在账号少于两个时不显示它。旧客户端不发这把键，读作缺席。
- **`EndpointSummary` 长出 `label`**：缺省即 `name`，所以页面永远不必替一个没写显示名的端点决定显示什么。
- **`proxying` 跟着 tuning 走，因而探测与调用恒用同一个决定**（WIRE_V 27→28）。一个只在调用时生效的代理设置，会让表单上那份分段读数描述一条真正的调用不会走的路，而那份读数存在的全部意义就是告诉人调用停在了哪一段。`Option` 而非值：线上的缺席是「没人定过」，装配层把它翻成城的默认值（`ExceptLocal`），于是 `gateway` 一侧拿到的是一个已经定下来的值，没有第三种状态要每一个调用方再答一次。
-/

/-! EndpointTuning 的可缺席 accounts 按输入顺序替换完整账号表；每行 id 为 ServerLabel，
reference 为可缺席 SecretRef，header 为可缺席字符串。无列表保留旧登记，显式非空表
由 gateway::router::tuning::validate_accounts 校验。凭据原文只走 enrolment。 -/
