-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# kernel::policy_limit

规定 `kernel::policy_limit`（`crates/kernel/src/policy_limit.rs`）：上限类政策值的合法域与拒因句式。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。
-/

/-!
### 8-73 `kernel::policy_limit`：上限类政策值的合法域与拒因句式，从类型给出（形状 2 值）

**为什么拒因归类型**：上限若是裸整数，四段拒因（动作／主体／码／recovery）就由每个调用点手拼，各自 `format!` 同一个数；数会改，句子会漏改，调用方也拼得出第二种拒因。类型给出拒因，调用点只递观测值。

**形状**：三个形状 2 值类型，各带私有字段、一个 crate 内构造器和唯一方法 `admit`。`admit` 是判定与文案的同一入口：域内回 `Ok(())`，越界回 `E_INVALID_ARGS`，四段全部由类型给出。

| 类型 | 唯一实例（数的家仍是 `consts_policy`，§8-8） | `admit` | 越界时的四段 |
|---|---|---|---|
| `ImagesPerTurn` | `IMAGES_PER_TURN` | `admit(found: usize) -> Result<(), AxError>` | 动作 `put pictures on a provider request`；主体 `this turn carries {found} pictures`；recovery `one turn carries at most {max} pictures; send the rest in a later turn` |
| `ImageMaxBytes` | `IMAGE_MAX_BYTES` | `admit(at: &Locator, size: usize) -> Result<(), AxError>` | 动作 `put a picture on a provider request`；主体 `{at} is {size} bytes`；recovery `one picture is at most {max} bytes; shrink it before attaching it` |
| `ImageQuality` | `IMAGE_QUALITY` | `admit(asked: u32) -> Result<(), AxError>` | 动作 `encode a picture`；主体 `quality {asked} is outside 0..={max}`；recovery `pass a quality between 0 and {max}; an encoder has no others` |
| `ClockZonesMax` | `CLOCK_ZONES_MAX` | `admit(configured: usize) -> Result<(), AxError>` | 动作 `format clock stamp`；主体 `{configured} zones exceed CLOCK_ZONES_MAX={max}`；recovery ``keep at most {max} rows in `[clock] zones`; the UTC row is added on top of them and is never one of them`` |

**合法域**：`0..=max`（含端点）——恰好落在上限上的观测被收下，越界一步即拒（不钳位）。句式与域随类型走，改上限只改 `consts_policy` 一处。观测值是 `usize`，上限是 `u64`，比较在 `u64` 里做：`usize` 至多 64 位，窄化在任何 Rust 目标上都不会失败，那条失败支仍答「越界」，这条判定因此不依赖目标的指针宽度。

**拼不出第二种拒因**：字段私有、唯一构造器 `new` 是 `pub(crate)`（只给 `consts_policy` 铸实例）、无 serde、无 getter、无 `Display`。外部调用方既铸不出一个更松的上限（那就等于第二种拒因），也读不出裸数去 `format!` 自己的句子；四段文案的唯一生产点是 `admit`。编译失败反例 `tests/ui/forge_policy_limit.rs` 钉住「调用方铸不出政策值」。两处生产调用点（`gateway::endpoint::call::pictures_for`、`runtime::clock::stamp`）的测试以整值相等断言「调用方交出的就是本类型的拒因」，某一个调用点再拼一句自有文案即红。

**三个类型不合并且不并入 `Ceiling`／`Window`**（8-40 同一条理）：上限互换后仍然编译得过，一个类型就是一个可交换面。

**队列（其余政策值逐个 newtype 化时从这里取）**：

- `WORKTREE_MAX_BYTES`：有拒因，但今天拒因是 `StorageError::WorktreeBusy` 的 detail（两个数都在里面），迁它的前提是那句 detail 也从类型派生。
- `OUTPUT_CEILING_DEFAULT`：合法域（非零）今天是两个家——`consts_policy` 的测试与 `gateway::provider::ceiling` 的 `Ceiling::new(..)?`；铸成 `Ceiling` 实例即一个家。
- `SANDBOX_FUEL_DEFAULT`：`SandboxLimits.fuel` 是裸 `u64`，域未设。
- `STARTUP_BUDGET_TOKENS`／`BYTES_PER_TOKEN`／`LOOP_REPEAT_THRESHOLD`／`OFFLOAD_MIN_BYTES`／`DRAFT_HELD_ESCALATE`／`EDIT_WAR_FREEZE`／`DISCARD_FILES_MAX`／`DISCARD_RETENTION_DAYS`：算术输入或判定输入，无拒因句式，保持裸数即是正确形状。
- `PREFIX_SLOTS`：域已在类型上（`NonZeroU64`），不迁。
-/

/-! D3 定规：上限类政策值的拒因句式从类型给出，调用方拼不出第二种拒因

这一条是人定的。

**决定**：有拒因语义的政策值 newtype 化（`kernel::policy_limit`，§8-73）：域内域外的判定与动作／主体／码／recovery 四段文案由类型一处给出，调用方只递观测值。外部铸不出一个更松的政策值（无公开构造器、字段私有、无 serde），也读不出裸数去拼自己的句子（无 getter、无 `Display`），第二种拒因无从拼起——编译失败反例 `forge_policy_limit` 钉住这两扇门。

**理由**：三段拒因是这座城对模型的教学面，同一个上限的句子散在几个调用点时，改数漏改句就会让一个上限说出两种话；四段文案收进类型，句子与域永远同一个家，改一处即改全部。

**被否**：①维持调用点手拼拒因（现状）——上限改动要靠人记得三处句子，漏一处即两种拒因；②一个泛型 `PolicyLimit<R>` 加标签参数——句式仍要按标签分支，只是把三个 `admit` 压成一个 match，可交换面反而变宽；③给类型留 getter 或 `Display` 让调用点自拼句子——那正是第二种拒因的入口。

**重开参数**：出现可由人移动的上限（像第二道阈值那样进 `CONFIG.toml`）时，构造点改为解析式（域外在解析点拒、拒因带合法域），本定规「拒因从类型给出」不动。
-/
