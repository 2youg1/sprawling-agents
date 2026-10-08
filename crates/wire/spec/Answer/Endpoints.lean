-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::answer::endpoints

规定 `answer::endpoints`（`crates/wire/src/` 下同名的文件）。设置页读回的已挂端点，与一次探测的读数。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的帧形状由 Rust 的类型与 `crates/wire/tests/wire_contract.rs` 钉住的 wire schema（`wire::schema_hash`）守住。
-/

/-!
### 8-28 `endpoint_probed` 答的是一次读数，不是一次成败

```jsonc
{ "name": …, "base_url": …,
  "reach": { "host": …, "named": …, "connected": …, "answered": …, "through": …, "elapsed_ms": … },
  "models": ["id", …],
  "facts":  [{ "id", "context_tokens"?, "max_output_tokens"?, "input_modalities", "input_price"?, "output_price"? }, …],
  "failed": { "code": …, "subject": … }   // 仅当模型表读不出来
}
```

- **读不出模型表的 probe 照样作答**。名字解析不了、端口没人应、证书不受信、供应方答 401，对填表的人是四个不同的下一步；作为一次拒绝返回，它们在界面上塌成传输库的一句话。记录带上停在哪一段（`kernel::Reach`，由 `gateway::reach` 量出），再把拒绝自己的 code 与 subject 放在旁边。
- **`models` 与 `facts` 同时在**。只要 id 的客户端不必读 facts；要显示上下文窗口与输出上限的表格不必第二次发问。读不出来时两者都是空数组而 `failed` 在场，于是「表是空的」与「表读不出来」在形状上可分。
- **没有被应答说出来的数字，记录里也没有**。`/models` 的一行里有什么由供应方决定；城不替它补零、补默认、补猜测——补出来的数字会盖过真正计费的那个。
-/

/-!
### 8-85 端点一行读回它是怎样挂上的，与每个账号的 Key 在不在

```rust
pub struct EndpointSummary {
    // …既有字段…
    pub tuning: EndpointTuning,                // 挂上它时的那份规矩，按 `AttachEndpoint` 携带的形状读回（§8-29）
    pub account_status: Vec<AccountStatus>,    // 依 `tuning.accounts` 的次序；没有账号表的旧登记为空
}
pub struct AccountStatus { pub id: kernel::ServerLabel, pub key: KeyState }
pub enum KeyState {                            // 线上拼作 snake_case
    Anonymous,             // 这个账号不指引用：请求不带 Key（无认证的本地端点）
    Stored,                // 金库在这条引用下有一个非空的值
    Missing,               // 金库里没有，也没有环境变量：页面可以存一个
    Environment,           // 一个环境变量给出了它：页面改不了，人要改的是那个变量
    EnvironmentUnusable,   // 环境变量设了而读不成文本：它遮住金库，Key 照样用不上
    Unread,                // 这座城没有开着的金库，没有去看
}
```

- **`tuning` 是挂上时的原样，不是生效值**：没人定过的数字仍是缺席，页面照旧用 `Query::Config` 的 `tuning` 画占位符（§8-47e）。读回的写法是 accounting 把 `AttachEndpoint` 的 tuning 变成 gateway 那份的同一个模块的逆向（`accounting::tuning`），两个方向放在一起，往返一次得到同一份 gateway tuning；`proxying` 读回为 `Some`，因为 gateway 一侧已经把缺席定成城的规则。请求头的值读回为 `spelled()`，即金库引用或一个过了密钥扫描的字面量，与账本里那一行相同，所以这里没有账本之外的东西。
- **`account_status` 按账号 id 对上，不按位置**：每一行带 `id`，页面不必假定两张表一样长、一样排。它只说 Key 在不在、页面能不能改，不带值，也不带环境变量的名字。四态由 `gateway::Custodian::describe` 的 `configured` 与 `writable` 读出：`writable` 为假就是环境变量给的（`describe` 只在那里答假），再按 `configured` 分出 `Environment` 与 `EnvironmentUnusable`；`writable` 为真时按 `configured` 分出 `Stored` 与 `Missing`。
- **看金库在快照放开之后**：`describe` 要问平台的凭据服务，这一问不能占着每个读者共用的那条折叠线程，所以 `EndpointView` 与 `Config` 一样走 `Prepared`（`crates/accounting/spec/Views.lean` §8-36）。金库的锁中毒时整份答复是 `Unavailable` 并带原因（D47），而不是把每个账号都说成 `Missing`。
- `WIRE_V` 随本组改形进一位（D1、D49）。
-/

/-! D49 设置页靠读回的 tuning 整份重发一次挂接，Key 状态按账号 id 答

**决定**：(a) `EndpointSummary` 带回挂上时的 `EndpointTuning`，账号编辑器改完顺序或增删账号后，把这份 tuning 换上新的 `accounts` 再发一次 `AttachEndpoint`；(b) 每个账号的 Key 状态是 `account_status` 里按 id 的一行，六态的 `KeyState` 由城算好；(c) 搜索供应方账号的 Key 状态用同一个 `AccountStatus`（`spec/Answer/Config.lean` §8-86）。

**理由**：(a) `AttachEndpoint` 整份替换 tuning（只有 `accounts` 缺席时保留旧表），页面若只知道账号，重发一次就会把人设过的期限、请求头和覆盖清掉；读回原样，账号的改动才是只改账号。(b) 布尔对（`configured`、`writable`）要页面自己判断「不可写又已配置」是什么意思，那是 `describe` 语义的第二份解释；城答一个词，页面只管画。按 id 而不按位置，是因为两张平行的表一旦一张变了，就会把 A 的状态画在 B 旁边。(c) 两处画的是同一种账号行，C2 的同一个外观组件两处共用。

**被否**：①另开一条只改账号的命令（`SetAccounts`）：要多一个动词、一条 reach 表行与一条账本行，而 gateway D28 已经把「一次挂接是完整账号表的原子替换」定为唯一写法；②像 `has_credential` 那样每个账号一个布尔：说不出「环境变量给的，页面改不了」；③答复里带环境变量名：多给的是主机配置的细节，页面的下一步并不需要它。

**重开参数**：tuning 里出现读回时必须隐去的字段（例如一个不是引用的凭据）时，(a) 要改成按字段读回。
-/

/-!
### 8-92 端点上每个模型带出它的思考档 offer 与跨供应方的身份

```rust
pub struct ModelFactsSummary {
    // …既有字段：id、context_tokens、max_output_tokens、input_modalities、input_price、output_price…
    pub thinking: ThinkingOffer,       // 这个（Endpoint，模型）提供的思考档，带来源
    pub canonical: String,             // 这个模型跨供应方的身份
}
pub struct ThinkingOffer {
    pub levels: Vec<Effort>,           // 城的固定升序；只含提供的档，恒不含 `none`
    pub on: Switch,                    // 只有开关、没有档位的模型能不能「开启思考」
    pub default: Option<Effort>,       // 上游说的默认档
    pub default_on: Option<bool>,      // 上游说的：什么都不发时是否在想
    pub words: Vec<EffortWord>,        // 只在上游的词与城的拼写不同时出现
    pub from: OfferSource,
    pub source: Option<String>,        // Preset 一级的文档地址
}
pub enum Switch { Allowed, Refused, Unknown }               // 线上 snake_case
pub enum OfferSource { Person, Upstream, Preset, Unknown }   // 线上 snake_case
pub struct EffortWord { pub effort: Effort, pub word: String }
```

- **两类字段，两种来历**：`id` 到 `output_price` 是上游的原话，缺席就是没说，恒不从预置表或梯子补上；`thinking` 是 gateway 的思考档梯子（`gateway::provider` 下的 `thinking` 模块） 沿 Person → Upstream → Preset → Unknown 的梯子解出的答案，它自己用 `from` 说出是哪一级给的，`source` 给出那一级的出处，所以页面读到的每一档都说得出它从哪里来，不会被当成上游的原话。
- **单位是（Endpoint，模型）**：同一个模型在不同供应方、不同 face 上提供的档位不同，所以 offer 挂在端点的模型行上；换供应方，页面画的思考档带跟着换，人存下的档位不变（人定下的规矩：模型与档位分开存）。
- **`levels` 为空就没有思考控制**：页面不画思考档带，也不画令牌上的档位一段；只有开关的模型 `levels` 为空而 `on` 是 `Allowed`，默认档 `High` 编码成「开启思考」。`none` 恒不在 `levels` 里，关闭思考不是一档。
- **`canonical` 只由 `gateway::provider::identity` 算**：上游说出它自己的规范 id 时用它，否则用模型 id 去掉最后一个 `/` 之前的组织前缀后转成小写；两行是同一个模型，当且仅当两个字符串相等，不做模糊匹配。页面按它把选择器里的模型合并成一行，旁边列出每个供应方自己的 id。
- **现状**：`canonical` 今天只走第二条（上游的规范 id 在 `ModelFacts` 读出它之后接上）；`thinking` 今天是 `from: Unknown` 的空 offer，直到梯子落地。
- `WIRE_V` 随本版的改形进一位（D53）。
-/
