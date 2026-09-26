# accounting-SPEC.md

> crate：`accounting`。本 SPEC 先于代码存在；实现不多不少地遵守本文。
> 骨架：apostle-sdd 十七节；按模块分章、每章自足（ARCHITECTURE.md §5）。

## 1 需求分解

`accounting` 是 accounting thread 的家：城的唯一写者，以及它向外伸手时经过的端口。它存在的理由是 ARCHITECTURE.md §11 记下的 V6 缺口——`RunWorker` 自己造模型适配器而不是接收一个，所以 citysim 的脚本只能复现一次 run，复现不了一次 dispatch。把写者移进一个只经端口向外伸手的 crate，端口的第二实现就能从外面把它驱动起来。

| 模块 | 这个模块回答的问题 | §8 |
|---|---|---|
| `models` | 一次 run 或一次命名调用，拿什么模型适配器去说话 | 8-1 |

目标形状里还有三个端口与一批搬迁，现在都还不在本 crate：

| 端口 | 它回答的问题 | 现在的住处 |
|---|---|---|
| `Clock` | 现在几点 | `bin::assembly::now_ms`，唯一采样点 |
| `Machine` | city 所在的主机上有哪些工具，以及装上一个 | `bin::doctor::probe::Machine`，`pub(crate)` |
| `Connectors` | 一栋楼连哪些 MCP server | `bin::assembly::mcp` |

`RunWorker` 的六个对象（凭据、协作、计划、治理、入口、飞行中的 run）、全部用例与 `views` 仍在 `crates/sprawling/src/assembly` 与 `crates/sprawling/src/views`。

## 2 验收标准

一个接收了脚本 `ModelFactory` 的 `RunWorker`，其 dispatch 走到脚本模型，而不是走到端点簿里登记的那个端点：`a_dispatch_reaches_the_model_the_worker_was_handed`（`crates/sprawling/tests/model_factory.rs`）。端点指向一个拒绝连接的 loopback 端口，所以只要 worker 还自己造适配器，这次 run 就碰不到脚本模型，历史里也就没有脚本写下的那句回答。

## 3 假设与歧义

- `Clock` 端口要不要连带 `last_tick` 与调度器一起搬：`last_tick` 是 `RunWorker` 字段，它的读法随六个对象一起定。能定下它的证据是：计划组搬进本 crate 之后，调度器还剩几个调用 `now_ms` 的地方。
- `views` 在读侧与写侧各有一份 `Governance`，搬进本 crate 时哪一侧拥有这个类型，取决于 `bin::views` 与 `bin::assembly` 之间的环断在哪里（sprawling-SPEC.md 8-92 的表）。

## 4 现状分析

本 crate 现有一个端口 `ModelFactory`。生产适配器在装配根（`bin::assembly::models`），第二实现在 `crates/sprawling/tests/model_factory.rs`。

## 5 权威信源

ARCHITECTURE.md §3（依赖律与 `depmap`）、§4（端口表）、§11（V6 缺口）；`gateway::adapter_for`（一个 `Chosen` 怎样变成一个适配器）。

## 6 命名统一

ModelFactory｜accounting thread｜Chosen｜Redemption。「适配器」专指 `kernel::Model` 的一个实现；「端口」专指本 crate 声明、外层实现的 trait。

## 7 模块边界

- 怎样按 endpoint、dialect、凭据造出一个适配器，归 `gateway::adapter_for`：本 crate 只声明「造一个」这个动作。
- 挑哪个模型（`EndpointBook::select`）与何时续期凭据，归 `RunWorker`：端口拿到的是已经选好的 `Chosen` 与已经兑换好的 `Redemption`。
- 把生产适配器接到 worker 上，归装配根 `bin::assembly`。

## 8 接口先行

### 8-1 accounting::models（形状 3 端口）

```rust
pub trait ModelFactory {
    /// # Errors
    /// Whatever building the adapter refuses: a malformed endpoint, a
    /// confidential building's bytes asked to leave the machine.
    fn build(
        &self,
        chosen: &gateway::Chosen<'_>,
        redemption: gateway::Redemption,
    ) -> Result<Box<dyn kernel::Model + Send>, AxError>;
}
```

```rust
// bin::assembly::models（形状 4 适配器）
pub struct GatewayModels;          // 生产：dialect 头 + gateway::adapter_for
impl RunWorker {
    pub fn with_models(self, models: Box<dyn accounting::ModelFactory + Send>) -> RunWorker;
}
```

- **失败**：原样传 `gateway::adapter_for` 的 `AxError`（它自带 action、subject、code 与 recovery）；端口不另造错误码。
- **两处调用点，一个端口**：dispatch 的同意阶段（`agreeing`）与命名调用（`session::naming_call`）都经 `RunWorker.models` 造适配器。dialect 头在生产适配器里算一次，两个调用点不再各拼一遍。
- **固定值**：`RunWorker::new` 与 `over` 装上 `GatewayModels`；`with_models` 是唯一换掉它的门。
- **一致性套件**：端口的断言就是 §2 那条测试——拿到的适配器必须就是被调用的那一个。

## 12 决策

1. **生产适配器住装配根，不住本 crate。** 理由：它把 `gateway` 的具体构造接到端口上，这正是 ARCHITECTURE.md §3 说的装配边；本 crate 只依赖 `kernel` 与 `gateway` 的接口类型。被否决的做法：在 `gateway` 里实现本 trait——那要让 `gateway` 依赖 `accounting`，依赖就朝外指了。
2. **`with_models` 是一个消费 `self` 的方法，而不是 `new` 的第四个参数。** 理由：生产只有一种工厂，`new` 的每个调用方（serve、doctor、测试）都会写同一个 `GatewayModels`；换工厂的只有 citysim 与测试。被否决的做法：`new` 加参数——四个调用点重复同一个值，而这个值只有一个权威。
3. **端口参数是 `Chosen` 与 `Redemption`，不含 dialect 头。** 理由：dialect 头由 `Chosen` 的 dialect 决定，把它交给调用方算，两个调用点就各有一份拼法。被否决的做法：照抄 `gateway::adapter_for` 的三参数签名。
