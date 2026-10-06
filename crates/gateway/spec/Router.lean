-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# gateway::router

规定 `router`（`crates/gateway/src/router.rs`）：已登记端点的簿与每个标签的选择；一次取模型只答一问。本文件是 `crates/gateway/Spec.lean` 的一个分部；下面每一节保留它在 gateway 规格里的标签 §8-n，别处引作 `crates/gateway/Spec.lean §8-n`。

本分部的 Accounts 模型证明显式账号在提交轨迹上保持存在，其他接口由 Rust 类型与 router 旁的回归检查守住。
-/

/-!
### 8-9 gateway::router（形状 7 projection）

```rust
pub struct AttachedEndpoint { pub name, pub base_url, pub dialect: DialectKind,
                              pub connection_kind: ConnectionKind,   // 登记时定下的连接种类（§8-18）
                              pub auth: AuthSpec, pub models: Vec<ModelFacts>,   // 对端报出的整行事实，不压成 id
                              pub probed: bool,    // 这份 models 是问出来的（true）还是人报的（false）
                              pub tuning: EndpointTuning }   // §8-16
impl AttachedEndpoint {
    pub fn is_local(&self) -> bool;          // 与 client_for 绕开代理同一依据（reach::is_local）
    pub fn first_auth(&self) -> Result<AuthSpec, AxError>; // 原登记或显式列表首账号
    pub fn has_credential(&self) -> bool;    // 关于凭证，金库外只能回答这一问
    pub fn chat_url(&self) -> String;        // base_url ＋ 该兼容格式自己的路径
    pub fn models_url(&self) -> String;
}
pub struct EndpointBook { /* 私有：endpoints（各带自己的 Transport）、chosen */ }
pub struct Chosen<'b> { pub endpoint: &'b AttachedEndpoint, pub entry: &'b ModelEntry,
                         pub(crate) transport: &'b Transport }   // 这个 endpoint 共用的客户端（§8-3）
impl EndpointBook {
    pub fn new() -> EndpointBook;   pub fn is_empty(&self) -> bool;
    pub fn apply(&mut self, record: &EventRecord) -> Result<(), AxError>;
    pub fn session_account(&self, addr: &Address, provider: &str) -> Option<&ServerLabel>;
    pub fn absorb(&mut self, kind: EventKind, run: RunId, addr: Option<&Address>, data: &Payload) -> Result<(), AxError>;
    pub fn apply_payload(&mut self, kind: EventKind, data: &Payload) -> Result<(), AxError>;
    pub fn select(&self, tag: ModelTag, policy: &BuildingPolicy) -> Result<Chosen<'_>, AxError>;
    pub fn accounts_for_attachment(&self, name: &str, incoming: Option<Vec<ProviderAccount>>)
        -> Option<Vec<ProviderAccount>>; // 登记与重放共用的缺席保留规则
    pub fn endpoints(&self) -> impl Iterator<Item = &AttachedEndpoint>;
    pub fn choices(&self) -> impl Iterator<Item = (ModelTag, &str, &ModelEntry)>;
}
pub fn attached_payload(&AttachedEndpoint) -> Result<Payload, AxError>;      // endpoint_attached 唯一成形处
pub fn selected_payload(ModelTag, &str, &ModelEntry, Option<CeilingSource>) -> Result<Payload, AxError>;  // model_selected 同上
```

- **一次取模型只答一问**：用哪个模型、在哪个端点上。「不答时怎么办」不是本 crate 的问题（§8-11）。
- **三种不答，三个码**：这一类标签没人选过＝`E_MODEL_UNCHOSEN`（出路：去设置页接供应方、选模型）；选过的端点已不在＝`E_CONFIG_INVALID`；confidential 楼的选择会让字节离开运行中的机器＝`E_GATE_DENIED`。「没选」单独成码，因为它是一座新城的第一个状态，客户端要按码给「去设置」，而 `E_CONFIG_INVALID` 在别处还答「会话中途换了模型」，那里的出路是开新对话。

- **为何不是 duty pool**：多 Agent 功能未成形之前，职责池没有消费者，而没人读的权威只会漂。降为 `ModelTag` 两值枚举（`Main`／`Digest`）：**标签因为有人按它取模型而存在**，新增一个标签的前提是先有调用方。
- **两个入口一个读者**：`apply`（重建路径，手里是 record）与 `apply_payload`（写入路径，手里是刚要写的 payload）共用同一套载荷读取，于是「写者以为的」与「重建得到的」不可能分岔。
- **confidential 在选型点再守一次**：非回环 endpoint 对 confidential 楼恒拒（`E_GATE_DENIED`）。`gateway::endpoint` 的兜底拒同期改为**按本地性判定**（而非一律拒）：规则是「字节不出运行中的机器」，不是「不准用这个类型」；否则一个回环的 Anthropic 服务器会被误拒。
- **路径归兼容格式**：人输入 base URL（provider 文档就是那么印的），`messages`／`chat/completions`／`models` 由兼容格式拼。这与 `EndpointConfig.base_url`「完整端点 URL、不拼路径」并不矛盾：适配器保持字面，拼路径的是上层登记面。
- **`probed` 是这份 models 的来源，不是端点的健康度**：`true` ＝ `GET .../models` 答了，登记的 id 是对端自己说的；`false` ＝ 探测失败而人自己报了型号，城照登。载荷里缺 `probed` 键读作 `true`，于是没有这个键的 `endpoint_attached` 重放不变。
- **`AuthSpec::for_dialect` 是凭证头的唯一产地**：`AuthSpec::for_dialect(dialect: DialectKind, reference: SecretRef, header: Option<String>) -> AuthSpec`，纯函数，住 `endpoint/auth.rs`。人显式填的头名恒胜（`Header`）；否则 Anthropic → `Header{name:"x-api-key"}`，OpenAI 及其余 → `Bearer`。**它不住 `endpoint/config.rs`**：「凭证头归兼容格式」是一个可以自己站着的概念；`AuthSpec` 类型本体留在 config.rs，因为搬它会让同一个名字在 crate 内多出一条 `pub(crate) use` 路径。登记面（`accounting::worker::credentials::endpoints::endpoint_of`）不自己在 Bearer 与具名头之间选，否则「Anthropic 用哪个头」在城里有两个权威，而漂开的总是没人看的那个。
-/

/-!
### 8-11 不设备用端点

一个标签只选一个端点与一个模型；`selected_payload` 直接收 `ceiling_from: Option<CeilingSource>`。线上没有设备用端点的字段，而一个在生产里只有一臂的「两臂的值」，另一臂就是给一个还不存在的设置面预留的权威。**带 `fallback_endpoint`／`fallback_model` 键的记录照样读得回**：`read_choice` 不取这两个键，与它对待任何本书不拥有的键同一口径，测试钉住「带着这两个键的 `model_selected` 仍读成它所述的那次选择」。

**重开的参数**：设置面上有人能为一个标签指定备用端点与备用模型，且 `runtime` 侧有一处在失败时读它。那时备用端点与 §8-6 的准入一起设计，因为何时改投备用端点只能由退避节奏回答。
-/

/-!
## 有序 Provider accounts

`kernel::event::record::ProviderAccount` 为账号声明，只含 ServerLabel id、可缺席的
SecretRef reference 与可缺席的 header；无 reference 为显式匿名账号。
EndpointTuning.accounts 缺席表示保留已有显式列表，尚未迁移的登记仍读原 auth。
显式列表必须非空、id/reference 不重复，匿名账号不得带 header。
endpoint_attached.tuning.accounts 为同一列表的 Ledger 形状，旧记录缺席仍可读。
列表内账号的增加、替换、移除和重排用完整非空列表走原 AttachEndpoint，
不创建第二个 Provider 数据库；缺席列表不移除已迁移声明。
保留规则只由 EndpointBook::accounts_for_attachment 决定，登记面与重放均调用它。
校验住 kernel::event::record::validate_provider_accounts；probe 与 adapter
从 AttachedEndpoint::first_auth 取得凭据，snapshot 保留同一账号列表，
显式列表不回退原 auth。成功回答将非秘密账号 ID 绑定到房间的 Session，
重排与重启不改变仍在列表中的绑定；新 Session 清除绑定，被移除的绑定回到首账号。
旧单账号登记继续使用原 AuthSpec，不构造虚拟账号或重新选择认证头。
失败处理仍由 runtime 既有的 Provider 策略决定，账号故障转移不在本接口内。
Accounts 模型以通过账号校验的提交为输入；
空表、重复引用和非法 header 由 Rust 账户校验回归判断。
-/

/-! D28 保持原登记事件与 Vault 格式，以完整有序列表为一次原子替换，避免逐账号命令
产生半个登记；拒绝空列表以免误作匿名或回退。此登记接口只规定账号声明与首次凭据解析，不承诺 Session 亲和或失败恢复。
-/
namespace Gateway.Router.Accounts
/-- 列表缺席才使用旧登记；显式空表不是旧登记。 -/
def effective {Account : Type} (legacy : Account) (stated : Option (List Account)) : List Account :=
  match stated with
  | none => [legacy]
  | some accounts => accounts
/-- 整次重排替换列表，缺席的更新保留已迁移列表。 -/
def replace {Account : Type} (held incoming : Option (List Account)) : Option (List Account) :=
  incoming.or held
theorem updates_preserve_explicit {Account : Type} (held : List Account) (updates : List (Option (List Account))) :
    (updates.foldl replace (some held)).isSome = true := by
  induction updates generalizing held with
  | nil => rfl
  | cons update rest ih =>
    cases update with
    | none => exact ih held
    | some accounts => exact ih accounts
end Gateway.Router.Accounts

/-! D29 显式账号列表生效时拒绝旧凭据字段

`accounting::worker::credentials::endpoints::endpoint_of` 先保留缺席的 accounts，
再调用 `gateway::AttachedEndpoint::validate_legacy_fields` 判定旧 credential；
该纯准入方法是登记命令拒绝旧字段的唯一权威。
有效 accounts 存在时，`secret` 或 `auth_header` 在场均返回
`E_CONFIG_INVALID`，action 为 `configure provider accounts`，subject 含 Provider 名及固定原因文字，不含 secret、header 或 reference，
recovery 引导修改具名账号的 reference/header 并省略旧字段。ProbeEndpoint 与
AttachEndpoint 共用这个判定，拒绝发生在 probe、诊断成功和 Ledger 写入之前。
旧字段与显式新列表同时出现也拒绝，避免一次提交宣称两种凭据权威。
校验只读字段的存在性，空字符串也算在场，拒绝先于 SecretRef 解析；
旧 Ledger 中已经归档的 auth 不是本次提交字段，不在重放时重新拒绝。

缺席旧字段允许保留或原子替换账号列表；没有显式列表的旧登记仍允许更新 auth。
不根据 reference 是否碰巧等于某账号推断目标，也不清除账号表，因为这两种做法
会使旧写者依赖列表内容，或破坏已有 Session 的健康亲和。Vault 同引用换密仍由
PutSecret 独立完成；此判定只决定登记命令是否有明确目标，不回滚先前的 Vault 操作。

下面 submit 对应 endpoint_of 的准入；trace 以原状态继续处理被拒的提交。
派生回归在 `crates/accounting/src/worker/credentials/endpoints.rs`：从 RunWorker.handle
登记 legacy、迁移 A/B、提交 C，检查错误、无外发、账本不变、实际 probe header，
并检查重启后的完整账号列表；提交轨迹的投影检查在 router::book 旁。
-/
namespace Gateway.Router.Accounts
structure Update (Account : Type) where
  accounts : Option (List Account)
  legacyCredential : Bool

def submit {Account : Type} (held : Option (List Account)) (update : Update Account) :
    Except Unit (Option (List Account)) :=
  let settled := replace held update.accounts
  if settled.isSome && update.legacyCredential then .error () else .ok settled

def apply {Account : Type} (held : Option (List Account)) (update : Update Account) :
    Option (List Account) :=
  match submit held update with
  | .error () => held
  | .ok settled => settled

theorem submissions_preserve_explicit {Account : Type} (held : List Account)
    (updates : List (Update Account)) :
    (updates.foldl apply (some held)).isSome = true := by
  induction updates generalizing held with
  | nil => rfl
  | cons update rest ih =>
    cases update with
    | mk accounts legacy =>
      cases accounts with
      | none =>
        cases legacy <;> simpa [apply, submit, replace] using ih held
      | some accounts =>
        cases legacy
        · simpa [apply, submit, replace] using ih accounts
        · simpa [apply, submit, replace] using ih held
/-- 任意多次旧字段提交都不改变已迁移账号，包括同时声明替换列表的提交。 -/
theorem legacy_submissions_preserve_accounts {Account : Type} (held : List Account)
    (updates : List (Option (List Account))) :
    (updates.foldl (fun state accounts => apply state ⟨accounts, true⟩) (some held)) =
      some held := by
  induction updates with
  | nil => rfl
  | cons accounts rest ih =>
    cases accounts <;> simpa [apply, submit, replace] using ih
end Gateway.Router.Accounts

/-! EndpointBook::absorb(kind,run,addr,data) 是实时与重放共用入口；model_called
保存每个 Run 的最后一次非秘密账号尝试，model_returned 将它提交到该房间的亲和。
RunFrozen 删除未成功尝试，SessionOpened 清该房间的亲和；snapshot 保存这些投影，
不含 Key。派活在 room_for 后将成功亲和传给 adapter，不把未定房间当 Session。
重排不挪健康账号；被移除的绑定按新表的首账号选择。
模型假设同一房间只有一个活动 Run，生产 open_session 在房间忙时返回 E_BUSY，
因此新 Session 不会在旧 Run 的回答尚未返回时覆盖它的绑定作用域。 -/
namespace Gateway.Router.Affinity
inductive Step (Account : Type) where
  | attempt (account : Account)
  | answered
  | frozen
  | opened
structure State (Account : Type) where
  pending : Option Account := none
  bound : Option Account := none
def apply {Account : Type} (state : State Account) : Step Account → State Account
  | .attempt account => { state with pending := some account }
  | .answered => { pending := none, bound := state.pending.or state.bound }
  | .frozen => { state with pending := none }
  | .opened => {}
theorem attempts_preserve_binding {Account : Type} (state : State Account) (attempts : List Account) :
    (attempts.foldl (fun held account => apply held (.attempt account)) state).bound = state.bound := by
  induction attempts generalizing state with
  | nil => rfl
  | cons account rest ih => exact ih (apply state (.attempt account))
end Gateway.Router.Affinity


/-! D30 成功的 model_returned 是唯一绑定提交点；使用原 Ledger 投影而不保存第二份
账号文件，保证实时、全重放和 snapshot+tail 读同一规则。账号撤销后按首账号选择，
不会在无错误请求之间轮换。Rust 的生产回归驱动 HTTP、重排和进程重建，投影检查
覆盖任意失败尝试序列；这些检查是实现符合性证据，不是 Rust 精化证明。
未覆盖的恢复分类、有限换账号预算、搜索设置与账号登记 UI 仍由各自接口后续规定。 -/
