-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# city::rules_tool

规定 `rules_tool`（`crates/city/src/` 下同名的文件）。居民怎么读、怎么提议改自己楼的规则，以及两件治理工具怎样说出自己治理的 scope。本文件是 `crates/city/Spec.lean` 的一个分部；下面每一节保留它在 city 规格里的标签 §8-n，别处引作 `crates/city/Spec.lean §8-n`，决定引作 `city D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `city::rules_tool` 旁的测试守住。
-/

/-!
### 8-2b city::rules_tool（形状 4 适配器）

```rust
pub struct RulesTool { /* city_root、building、meta —— 私有；op ∈ {read, propose} */ }
impl RulesTool { pub fn new(city_root: &Path, building: Address) -> Result<RulesTool, AxError>; }
// meta.effect = Effect::Govern；effect_of：read → Read，propose → Govern（kernel Gate.lean D26）
```

- **`propose` 在效果层被拒，`read` 放行**：`effect_of` 对 `read` 答 `Effect::Read`、对 `propose` 答 `Effect::Govern`（`crates/kernel/spec/Gate.lean` D26），读规则不改写什么，模型因此看得到审判它的规则。改写的那一臂仍在效果层被拒，这是定规而不是漏接：一个 run 不改写审判它自己的规则（D1 定规；`crates/kernel/Spec.lean` §8-27 的 `Governance` 行）。`Effect::Govern` 无门、无审批、无 `ApprovalItem`：`runtime::bench::admit` 在 `invoke` 之前就把调用拒掉，拒绝以 tool result 回到模型而回合不终止（`runtime::turn::wave`「A tool Err is not a turn Err」那条）。规则要变只有人改文件这一条路——`RULES.toml` 的唯一写者是人，下一个 run 按改后的字节受审。
- **拒词说出被治理的 scope**：本工具的 `subject` 答 `GateSubject::Scope`（§8-36），效果层因此给出 `E_GATE_DENIED` 的「一个 run 不得改写审判它自己的规则」，恢复语指向人改的 `CONFIG.toml` 与 `RULES.toml`。参数读不懂的调用在同一处被拒，拒词与 `invoke` 读到同样参数时给出的相同；两种拒都出自效果层，run 都到不了 `invoke`。
- **为什么不是 `edit`**：`RULES.toml` 住在楼的保留子树，没有任何写域到得了那里——这不是一个要绕过的障碍，它就是规则本身。本工具 `propose` 一臂的写面是 `policy::write_rules`，形状是整份提案、先求值后落盘（§8-2 末条）；这一臂在效果层恒被拒，run 到不了它。人改楼规走另一扇门 `policy::write_rules_against`（带基线，§8-2 的那一节），两扇共用同一个求值器与同一个落盘函数。
- **楼是携入的而不是参数**：工具持调用方自己那栋楼的地址，于是一个 Run 无法靠填另一个名字去改别人的规则。
-/

/-!
### 8-36 两件治理工具说出自己治理的 scope（`rules_tool`、`city_tool` 的 `Tool::subject`，形状 4 适配器）

```rust
impl Tool for RulesTool {
    fn subject(&self, call: &ToolCall) -> Result<GateSubject, AxError>;
    // Ok(GateSubject::Scope("building:<本楼地址>"))，op 读不懂即 Err
}
impl Tool for CityTool {
    fn subject(&self, call: &ToolCall) -> Result<GateSubject, AxError>;
    // Ok(GateSubject::Scope("city"))，action、name、template 读不懂即 Err
}
```

- **scope 的拼法取 `kernel::event::Scope` 的 `Display`**：`rules` 答本楼的 `Scope::Building`，`city` 答 `Scope::City`，`GateSubject::Scope` 里的字符串就是账本上 `rules_changed` 写 scope 的那一种文字（D13）。
- **`city` 三个动作答同一个 scope**：`list` 读、`raise` 与 `adopt` 改的都是城的形状，被点名的那栋楼在 `raise` 时还不存在，治理它的不是它自己的规则。
- **参数只有一处文法**：`subject` 与 `invoke` 经同一个读参函数（`rules` 的 `Op::read`、`city` 的 `Request::read`），所以 `subject` 的拒词就是 `invoke` 读同样参数时的拒词（`crates/kernel/Spec.lean` §8-23 讲 `Tool::subject` 的那一段：文法读不出的调用返回 `Err`，bench 原样拒收）。
- **验收**：两件工具对一条合法调用答出上述 `Scope`，对一条读不懂的调用答出与 `invoke` 相同的码（`city` crate 内两件工具旁的测试）；经工具台的拒词由 `runtime::bench::admit` 的 Govern 臂给出，它的测试已钉住恢复语。
-/

/-! D1 定规：一个 run 不改写审判它自己的规则

`Verdict: user-approved`（kernel D1 那条定规的城侧应用；六类升级的固定答案表见 `crates/kernel/Spec.lean` §8-27）

**决定**：`rules`／`city` 两件工具保留 `Effect::Govern`，而这个效果在效果层恒拒：run 提不出提案、等不到审批、写不了规则。规则要变只有人改文件这一条路——`RULES.toml` 的唯一写者是人；建楼走线上命令 `CreateBuilding`，收编走 CLI 的 `sprawling adopt`。

**理由**：规则是审判一个 run 的尺子，而提案-审批形把改尺子的手留给被审判者、把「批准」放进一个 agent 循环——默认答案会被点过去的门等于没有门（kernel D1 同一理由，此处第二次适用而不是第二个权威）。规则的正确位置是文件本身：diff、历史与回退都在版本库里，而一份获批的提案正文只在一次对话里活过一回。

**被否**：govern 存在形——提案正文由 kernel 里的 govern 门 截一段写进 `action_desc` 供人过目，门问人、批后落盘。它与 `GateOutcome::Escalate` 在 kernel 侧同集删净（kernel D1 的同集删净名单列着 `gate::govern`）；`rules_tool` 的 `op=propose` 只保留「整份文档、先求值后落盘」这个形状（§8-2b），通向它的判定是拒而不是问。

**重开参数**：`attach` 是唯一会问人的门，理由是人的动作本身就是答案、没有可以点过去的默认（`crates/kernel/Spec.lean` §8-27）。治理审批只有取得同样的性质——人在 run 之外对整份 diff 作答，且不存在「全批」的默认——才需要重新论证这一条；参数不动，定规不动。
-/

/-! D13 治理工具的 scope 用账本上的 scope 文字

**决定**：`rules` 与 `city` 的 `subject` 答 `GateSubject::Scope`，字符串是 `kernel::event::Scope` 的 `Display`：`building:<地址>` 或 `city`（§8-36）。

**理由**：scope 在这座城里已经有一种文字，`rules_changed` 与 `city_halted` 都按它写进账本，`Scope::parse` 读回它。拒词里的 scope 用同一种文字，模型与人读到的是账本上会出现的那个词；`city` 与一栋叫 `city` 的楼在这种写法里也分得开。补上 `subject` 之前，两件工具答 trait 默认的 `GateSubject::None`，效果层按「主体与效果不一致」拒，恢复语让模型去报告工具缺陷，而工具并没有缺陷。

**被否**：①只写楼的地址（`lab`）：`city` 要另造一个词，而这个词可能就是某栋楼的名字；②`city` 的 `raise`／`adopt` 答被点名的楼：那栋楼还不存在，改的是城的形状，治理它的不是它自己的规则；③`subject` 不读参数、只答 scope：一条读不懂的调用得到的是「不得改写规则」而不是「这个动作不存在」，模型下一步改不对。

**重开参数**：`GateSubject::Scope` 改成携带类型化的 `kernel::event::Scope`（kernel 一侧的改形），这时两件工具交值而不交文字。
-/
