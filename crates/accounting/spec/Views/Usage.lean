-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# accounting::views::usage：skill 与 MCP 的使用表

规定 `crates/accounting/src/views/usage.rs` 与它的三个子模块 `answers`、`export`、`shelves`。本文件是 `crates/accounting/Spec.lean` 的一个分部；下面每一节保留它在 accounting 规格里的标签 §8-n，别处引作 `crates/accounting/Spec.lean §8-n`，决定引作 `accounting D<n>`。

前半是接口与决定，后半是折叠的模型：一次读在没有钉住那件 skill 的 run 里不算使用，折叠只往使用列表的末尾加，每一次使用都有一个钉住它的 run。Rust 侧由 `accounting::views::usage::model_tests` 的 proptest 从同一个输入空间抽轨迹，判同三条性质。
-/

/-!
### 8-34 skill 与 MCP 的使用表从账本折出（`accounting::views::usage`，形状 7 投影；`crates/wire/Spec.lean` D28、D33）

```rust
// accounting::views::usage
pub(crate) struct Usage { /* 折叠状态，私有 */ }
impl Usage {
    pub(crate) fn fold<'a>(records: impl IntoIterator<Item = &'a EventRecord>) -> Usage;
    pub(crate) fn skills(&self, shelves: &[Shelved], only: Option<&str>) -> wire::SkillUsageAnswer;
    pub(crate) fn mcp(&self, configured: &BTreeSet<String>, only: Option<&str>) -> wire::McpUsageAnswer;
    pub(crate) fn shells(&self) -> wire::ShellsAnswer;
}
// accounting::views::usage::export
pub(super) fn skill_rows(answer: &wire::SkillUsageAnswer) -> Vec<Row>;
pub(super) fn mcp_rows(answer: &wire::McpUsageAnswer) -> Vec<Row>;
pub(super) fn write(rows: &[Row], format: wire::ExportFormat) -> String;
```

- **一遍读完整本账。** `Query::SkillUsage`、`McpUsage`、`UsageExport` 与 `Shells` 在快照放开之后由 `LedgerAsk` 从第一行读到最后一行，交给 `Usage::fold`；读不下去的一行结束这一遍，已经读到的照答（与 `history` 同一条规则）；账本索引本身打不开时答 `Unavailable`，`reason` 是那个错误（wire D47）。
- **shell 读数**（wire D48）：同一遍里每一条 `tool_result` 的 `result` 对象交给 `runtime::ShellTally::absorb`，`shells` 照它的 `interpreters` 逐行写出；哪些行算、怎么分类只在 `crates/runtime/spec/Tools/Exec.lean` D30。
- **skill 的一次使用**照 wire D33：这一行是 `tool_called`，它的 run 的 `run_started` 钉住了这件 skill 的名字；`describe` 的 `name` 是那个名字或 `skill <名字>` 时部分记作 `guide`，`read` 的 `path` 是那个名字时记作 `SKILL.md`，是 `<名字>/<相对路径>` 时记作那个相对路径。使用的版本是钉住时的哈希。
- **一个内容版本**是某件 skill 在账上第一次以某个摘要出现的那一行：一行 `skill_shelved`（kernel D23）或一个钉住它的 `run_started`；摘要、那一行的 `seq`、时刻与 run，以及 wire D33 的 `author`：带同一摘要的 `skill_shelved` 给 `Shelved`（它的 `seq` 与 `source`，晚于版本到达的也补上），否则看钉住那一刻之前这个名字有没有过 `skill_shelved`，有即 `OutsideShelf`，无即 `Unrecorded`。
- **书架上的每一件都有一行**，没被用过的计数为零；书架上已经没有、但账上用过的名字也有一行，`held` 为空。同名的件在几格书架上时，`held` 每格一项，各带此刻的摘要与审核状态。
- **审核状态**照 `city::library::audit_state` 判（`crates/city/spec/Library/Audit.lean`），输入是账上这个名字的每一行 `skill_audited`。此刻的摘要与任何一次审核都不同、而审核存在时，状态是 `Stale`：页面据此请 User 重新审核。
- **MCP 的一次使用**是一行 `tool_called`，它记下的 `effect` 是 `Connector { label }`：服务器就是那个 `label`，工具名是行上的名字去掉 `<label>_` 前缀。`call` 一行指名的工具按第一个 `_` 拆（`kernel::ServerLabel` 不收 `_`），拆出的头是已知服务器名（账上见过的与此刻配置的）才归属；否则归在 `server: None` 下，工具写完整的名字。
- **结果**：同一个 run 里 `tool_use_id` 与它配对的 `tool_result` 答了是 `Ok`，拒了是 `Failed`，没有配对行是 `Unknown`。
- **按天数**：信封时刻的 UTC 日历日，`YYYY-MM-DD`，由 `runtime::clock::iso` 的前十个字符读出，不另写一份历法。
- **从没用过的 MCP 服务器**：每栋楼的配置（`city::load_config`）里列出的服务器各有一项，没有用过的工具表为空；此刻提供哪些工具要一次握手，那是 `McpHealth` 的问题。
- **导出**的列与格式照 wire D33；JSONL 与 CSV 由同一张行表写出，行表由 `skills` 与 `mcp` 两个答复展开。
- **三个平台**：只读账本与书架，Windows、macOS 与 Linux 相同；CSV 的 `\r\n` 是 RFC 4180 的。
- 验收：`views::usage::tests` 的 `a_fixture_ledger_folds_a_usage_table_per_skill_and_server`、`a_skill_whose_content_changed_is_asked_to_re_audit`、`an_export_writes_one_row_per_use_in_both_formats`；`views::usage::model_tests` 的两条 proptest 判模型的三条性质。
-/

/-! D49 使用表在答问时从账本折，不进 `Views` 的常驻折叠

**决定**：`Usage` 是一个纯函数的结果，三帧在快照放开之后各读一遍账本；`Views` 不多一张表，快照编码与 `VIEWS_FOLD_RULES` 不动。MCP 的服务器从行上记下的 `effect` 读，不从此刻的配置猜。

**理由**：使用表只有 skill 页与 MCP 页打开时才问，常驻折叠要为每一行 `tool_called` 付代价、为快照多编一张随账本线性增长的表；答问时读一遍是 wire D33 写下的形状，它的重开参数就是本决定的重开参数。服务器读 `effect`，是因为工具注册时 `agent_protocols::mcp::tools` 写下的 `Connector { label }` 随行进了账本：它说的是调用那一刻的服务器，配置改过、服务器删掉之后仍然对；而「此刻的登记」要握一次手才知道工具名，答一次使用表要等几秒。

**被否**：①常驻折叠：快照多一张表，`VIEWS_FOLD_RULES` 进位，而这两页不常开；②按此刻的配置拆名字：服务器删掉后它的使用全变成「已移除」，且要握手；③凡 `<x>_<y>` 都归给服务器 `x`：城自己的工具名里也有 `_`（wire D33 ③）。

**重开参数**：一座城的使用表答复超过 100 ms 时（wire D33），改成常驻折叠。
-/

namespace Accounting.Views.Usage

/-- 模型里账本的一行，只留折叠读的那几件：钉住、一次读、别的行。 -/
inductive Line where
  | pinned (run skill : Nat)
  | read (run skill day : Nat)
  | other

/-- 折叠的状态：见过的 `(run, skill)` 钉住对，与每次使用 `(skill, day)`，按账本次序。 -/
structure Table where
  pins : List (Nat × Nat)
  uses : List (Nat × Nat)

/-- 一行怎样移动表。 -/
def step (t : Table) : Line → Table
  | .pinned r s => { t with pins := (r, s) :: t.pins }
  | .read r s d => if (r, s) ∈ t.pins then { t with uses := t.uses ++ [(s, d)] } else t
  | .other => t

/-- 从某张表起折一串行。 -/
def foldFrom (t : Table) (lines : List Line) : Table := lines.foldl step t

/-- **一次读在没有钉住这件 skill 的 run 里不算使用**：与路径的写法无关。 -/
theorem a_read_the_run_did_not_pin_is_not_a_use (t : Table) (r s d : Nat)
    (h : (r, s) ∉ t.pins) : step t (.read r s d) = t := by
  simp [step, h]

/-- 一行只往使用列表的末尾加。 -/
theorem step_only_appends (t : Table) (l : Line) :
    ∃ more, (step t l).uses = t.uses ++ more := by
  cases l with
  | pinned r s => exact ⟨[], by simp [step]⟩
  | read r s d =>
    by_cases h : (r, s) ∈ t.pins
    · exact ⟨[(s, d)], by simp [step, h]⟩
    · exact ⟨[], by simp [step, h]⟩
  | other => exact ⟨[], by simp [step]⟩

/-- **折叠只往末尾加**：账本只追加，一次使用一旦算进表就留在原处，后来的行改不了它。 -/
theorem the_fold_only_appends (lines : List Line) (t : Table) :
    ∃ more, (foldFrom t lines).uses = t.uses ++ more := by
  induction lines generalizing t with
  | nil => exact ⟨[], by simp [foldFrom]⟩
  | cons l rest ih =>
    obtain ⟨a, ha⟩ := step_only_appends t l
    obtain ⟨b, hb⟩ := ih (step t l)
    refine ⟨a ++ b, ?_⟩
    simp only [foldFrom, List.foldl_cons] at hb ⊢
    rw [hb, ha, List.append_assoc]

/-- 每次使用都有一个钉住它的 run。 -/
def Pinned (t : Table) : Prop := ∀ u ∈ t.uses, ∃ r, (r, u.1) ∈ t.pins

theorem step_keeps_pinned (t : Table) (l : Line) (h : Pinned t) : Pinned (step t l) := by
  cases l with
  | pinned r s =>
    intro u hu
    obtain ⟨r', hr'⟩ := h u hu
    exact ⟨r', List.mem_cons_of_mem _ hr'⟩
  | read r s d =>
    by_cases hp : (r, s) ∈ t.pins
    · have pins : (step t (.read r s d)).pins = t.pins := by simp [step, hp]
      intro u hu
      rw [pins]
      simp only [step, hp, if_true, List.mem_append, List.mem_singleton] at hu
      rcases hu with hu | hu
      · exact h u hu
      · subst hu; exact ⟨r, hp⟩
    · simpa [step, hp] using h
  | other => simpa [step] using h

/-- **每一次使用都有一个钉住它的 run**：从空表起折任何一串行都如此。 -/
theorem every_use_was_pinned (lines : List Line) :
    Pinned (foldFrom ⟨[], []⟩ lines) := by
  suffices ∀ t, Pinned t → Pinned (foldFrom t lines) from this _ (by intro u hu; cases hu)
  induction lines with
  | nil => intro t h; simpa [foldFrom] using h
  | cons l rest ih =>
    intro t h
    simpa [foldFrom] using ih (step t l) (step_keeps_pinned t l h)

end Accounting.Views.Usage
