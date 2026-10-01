-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import crates.kernel.spec.Address

/-!
# kernel::write_domain

规定 `kernel::write_domain`（`crates/kernel/src/write_domain.rs`）：写域、只写文档的写域、写入限制与 edit war 的判定。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。
-/

/-!
### 8-11 kernel::write_domain

```rust
// WriteDomain 是两臂枚举 Everything／Documents，各带一个过了 C17 的 DomainPrefixes（§8-46）；
// 一次 run 在它之上再叠一道写入限制 WriteLimit（§8-78）。
impl WriteDomain {
    /// C17 at the construction point: any reserved-prefix member is refused.
    pub fn new(prefixes: Vec<Address>) -> Result<Self, AxError>;   // E_INVALID_ARGS
    pub fn admits(&self, target: &Address) -> DomainVerdict;
    pub fn prefixes(&self) -> impl Iterator<Item = &Address>;
}

pub struct EditSample { pub addr: Address, pub run: RunId }            // 切片序＝时序
pub enum EditWarVerdict { Calm, Freeze { addr: Address } }
pub fn observe_edit_war(samples: &[EditSample]) -> EditWarVerdict;
```

- `admits`：目标 `is_reserved()` 恒 Outside（构造点已拒，判定点再拒＝fail-closed 双层）；否则 ∃prefix 使 `target.is_within(prefix)` → Within。空前缀集合法（只读角色），恒 Outside。
- **edit war 依据**：同 addr 的样本按序去重相邻同 Run 后得 run 序列 r₁…rₙ；「夺回」＝rᵢ==rᵢ₋₂ 且 rᵢ≠rᵢ₋₁；夺回数 ≥ `EDIT_WAR_FREEZE`(2) → Freeze（A→B→A→B 即两次夺回）。逐 addr 独立计，首个达阈的 addr 入 verdict（BTreeMap 序）。
- `#[test]` `reserved_target_is_outside_even_for_an_empty_domain` 守 reserved 目标恒不 Within（kani 不接手，理由见 `crates/kernel/Spec.lean` §2）。
-/

/-!
### 8-46 kernel::write_domain 增 `WriteDomain::Documents`（形状 1 判定 + 形状 2 value）

**需求**：City Hall 的两位居民（Mayor、clerk）只写 Markdown。给他们一个「整栋楼可写」的写域，再靠提示词请他们别碰代码，是把不变量交给措辞；写域本身要能表达「只写文档」。

**接口**：

```rust
pub struct DomainPrefixes { /* prefixes: Vec<Address> —— 私有 */ }
impl DomainPrefixes {
    /// C17 的唯一构造点：任一 reserved 成员拒整个前缀集。
    pub fn new(prefixes: Vec<Address>) -> Result<Self, AxError>;      // E_INVALID_ARGS
    pub fn iter(&self) -> impl Iterator<Item = &Address>;
}

pub enum WriteDomain {
    /// 前缀内的一切文件。
    Everything(DomainPrefixes),
    /// 前缀内的 Markdown 文档，且永不含任何 `Roadmap.md`。
    Documents(DomainPrefixes),
}
impl WriteDomain {
    pub fn new(prefixes: Vec<Address>) -> Result<Self, AxError>;      // ＝ Everything
    pub fn documents(prefixes: Vec<Address>) -> Result<Self, AxError>;
    pub fn admits(&self, target: &Address) -> DomainVerdict;
    pub fn prefixes(&self) -> impl Iterator<Item = &Address>;
}

pub enum DomainVerdict {
    Within,
    Outside { prefixes: Vec<String> },
    /// 落在前缀内，但不是这个写域写的那种文件。
    NotWritable { reason: DocumentReason },
}

pub enum DocumentReason { NotMarkdown, ThePlan }

pub const ROADMAP_FILE: &str = "Roadmap.md";   // 随 ROADMAP_COLUMNS 住 spine::row
```

- **变体带私有值而非公开字段**：`WriteDomain::Documents` 可以被外部写出来，但只能拿一个已经过 C17 检查的 `DomainPrefixes` 去写。一个构造点，不因为枚举化而多出第二个。
- `admits` 判定序（fail-closed，先拒后放）：target `is_reserved()` → `Outside`；不在任一前缀内 → `Outside`；`Everything` → `Within`；`Documents` → 文件名不以 `.md` 结尾 → `NotWritable { NotMarkdown }`；文件名等于 `ROADMAP_FILE`（任意深度、任意楼）→ `NotWritable { ThePlan }`；否则 `Within`。
- **为什么计划文件在写域这一层就拒**：`Roadmap.md` 的唯一编辑入口是 `plan` 工具（`kernel::spine::rewrite` 的重写），它保证六列表格重写后仍成立。放任 `edit` 直接改，等于给同一条规则第二个权威，而漂掉的那个总是没人读的那个。
- `gate::domain` 对 `NotWritable` 出 `E_OUTSIDE_WRITE_DOMAIN` 三段式：规则「这个写域只写 Markdown 文档」，违规指出是哪一种，替代给出「改 `.md`」或「用 `plan` 工具」。
- 被否：给 `WriteDomain` 加一个 `documents: bool` 字段——布尔旗标不是穷尽枚举，且「只写文档」与「什么都写」是两条策略而不是一条策略的一个开关。

**一个区域不是一个文件。** `Effect::Write { domain }` 是工具在建造时**声明**的区域（`hall/mayor`），不是某次调用要写的文件；把这块区域交给 `gate::domain` 判，`Documents` 域就会对着 `hall/mayor` 答「不是 Markdown 文档」，Mayor 便写不了任何一份文档，而写文档正是它唯一被交代的事。所以两个问题各有一个权威：

```rust
impl WriteDomain { pub fn reaches(&self, area: &Address) -> bool; }   // admits 的前缀半段：非保留区、且在某个前缀内
pub fn reach(domain: &WriteDomain, area: &Address, taint: &TaintSet) -> GateOutcome;   // 区域：只问够不够得到
```

- `gate::reach` 判声明的区域：`reaches` 为假出与 `domain` 同一段 `Outside` 三段式（同一处产出，`outside` 提为二者共用），为真 `Allow`；它永不问文件名，因为区域没有文件名。
- `gate::domain` 判文件，一字不改；它的调用方从 bench 挪到 **`runtime::tools::edit`** 拿到路径的那一刻（`crates/runtime/Spec.lean` §8-36）——门口只判区域，所以工具这一层必须判全（`Outside` 与 `NotWritable` 都拒），它判全的方式是调同一个门。
- 被否：让 bench 读 `call.args["path"]`——那把 bench 和一个工具的参数名绑在一起，而 `exec` 同样声明 `Write` 却没有路径。
-/

/-!
### 8-78 写入限制：在写域之上叠一道「只新建」（`kernel::write_domain`、`kernel::gate::domain`，形状 1 判定＋形状 2 值）

```rust
pub enum WriteLimit { Full, Create }                                // "full" | "create"
impl WriteLimit { pub const ALL: [WriteLimit; 2]; pub const fn as_str(self) -> &'static str; }
pub fn replacing(limit: WriteLimit, target: &Address) -> GateOutcome;   // kernel::gate
```

- **限制叠在写域上，不并进写域。** 写域（§8-11、§8-46）回答一个地址能不能写、写哪种文件，它来自楼的 `RULES.toml`；写入限制回答一次 run 能不能改动已经存在的文件，它来自这次派活。`Full` 表示不额外收窄；`Create` 表示只准原子地新建一个不存在的普通文件，已经存在的文件——包括这次 run 刚建成的——不能覆盖、删除或改名。两道判定都要通过，所以 `Full` 永远放不宽楼的写域，`Documents` 楼里的 `Create` 仍只能新建 Markdown 文档、仍够不到计划文件。
- **`gate::replacing` 是「这次写会动到已有文件」时的唯一判定**：`Full` 答 `Allow`；`Create` 答 `Deny`，`E_OUTSIDE_WRITE_DOMAIN` 三段式，规则「this run creates files and changes none」，违规点出目标，替代给出「写到一个新路径」，恢复语说限制由派活时选定、换一次派活才能改。复用既有的码而不新开一个：对调用者而言这与写域外的拒绝是同一类事——这次 run 不准写那里——恢复的路也同类。
- **「目标是否已经存在」不由本模块判。** 那是文件系统在写那一刻的事实；只有在写的那一刻原子地判，竞争的两次新建才只成一次（`crates/runtime/Spec.lean` §8-55，storage-SPEC §8-32）。kernel 只持规则与拒词，判定点在每一条写路径上调它。
- 验收：`gate::domain` 测试里 `Create` 拒、`Full` 放各一条；真实写路径上的验收在 `crates/runtime/Spec.lean` §8-55。
-/

namespace Kernel.WriteDomain

open Kernel.Address

/-- 一个写域写的那种文件之外的目标，为什么不写。与 `kernel::DocumentReason` 逐变体同名。 -/
inductive DocumentReason where
  | NotMarkdown
  | ThePlan
  deriving DecidableEq, Repr

/-- `WriteDomain::admits` 的答案，与 `kernel::DomainVerdict` 逐变体同名。 -/
inductive DomainVerdict where
  | Within
  | Outside (prefixes : List Address)
  | NotWritable (reason : DocumentReason)
  deriving DecidableEq, Repr

/-- 两种写域，各带一个过了 C17 的前缀集，与 `kernel::WriteDomain` 逐变体同名。前缀集只能由 `DomainPrefixes::new` 造出，见下面的 `prefixes_new`。 -/
inductive WriteDomain where
  | Everything (prefixes : List Address)
  | Documents (prefixes : List Address)
  deriving DecidableEq, Repr

/-- 一次 run 在写域之上叠的限制，与 `kernel::WriteLimit` 逐变体同名（8-78）。 -/
inductive WriteLimit where
  | Full
  | Create
  deriving DecidableEq, Repr

/-- `admits` 对一个目标除了地址还要知道的事：哪些段名受保护（`PROTECTED_METADATA`，ASCII 折叠），文件名是不是 Markdown，是不是计划文件（`ROADMAP_FILE`）。三者的拼写只住 Rust。 -/
structure Files where
  protected_name : String → Bool
  markdown : Address → Bool
  plan : Address → Bool

/-- `DomainPrefixes::new`，C17 的唯一构造点：任一成员落在保留区里，整个前缀集被拒（`E_INVALID_ARGS`）。 -/
def prefixes_new (protected_name : String → Bool) (prefixes : List Address) : Option (List Address) :=
  if prefixes.any (is_reserved protected_name) then none else some prefixes

theorem prefixes_new_refuses_every_reserved_member (protected_name : String → Bool)
    (prefixes kept : List Address) (built : prefixes_new protected_name prefixes = some kept) :
    ∀ pre ∈ kept, is_reserved protected_name pre = false := by
  simp only [prefixes_new] at built
  split at built
  · cases built
  · rename_i none_reserved
    cases built
    intro pre member
    simp only [List.any_eq_true, not_exists, not_and] at none_reserved
    simpa using none_reserved pre member

def WriteDomain.prefixes : WriteDomain → List Address
  | .Everything prefixes => prefixes
  | .Documents prefixes => prefixes

/-- `WriteDomain::reaches`，`admits` 的前缀半段：目标不在保留区里，且在某个前缀之内。`gate::reach` 判一块声明的区域时只问这一半，因为区域没有文件名。 -/
def WriteDomain.reaches (protected_name : String → Bool) (domain : WriteDomain) (target : Address) :
    Bool :=
  !is_reserved protected_name target && domain.prefixes.any (is_within target)

/-- `WriteDomain::admits` 的判定序（先拒后放）：保留区与前缀之外 → `Outside`；`Everything` → `Within`；`Documents` 里不是 Markdown → `NotWritable NotMarkdown`；是计划文件 → `NotWritable ThePlan`；否则 `Within`。 -/
def WriteDomain.admits (files : Files) (domain : WriteDomain) (target : Address) : DomainVerdict :=
  if !domain.reaches files.protected_name target then .Outside domain.prefixes
  else
    match domain with
    | .Everything _ => .Within
    | .Documents _ =>
      if !files.markdown target then .NotWritable .NotMarkdown
      else if files.plan target then .NotWritable .ThePlan
      else .Within

/-- **写域收下的目标恒不在保留区里。** 判定点再拒一次，与构造点的拒绝合成 fail-closed 的双层；这条对空前缀集一样成立（Rust 的 `reserved_target_is_outside_even_for_an_empty_domain`）。 -/
theorem an_admitted_target_is_never_reserved (files : Files) (domain : WriteDomain) (target : Address)
    (admitted : domain.admits files target = .Within) :
    is_reserved files.protected_name target = false := by
  simp only [WriteDomain.admits] at admitted
  split at admitted
  · cases admitted
  · rename_i reached
    simp only [Bool.not_eq_true', Bool.not_eq_false] at reached
    simp only [WriteDomain.reaches, Bool.and_eq_true, Bool.not_eq_true'] at reached
    exact reached.1

/-- 收下即够得到：`admits` 答 `Within` 的目标，`reaches` 也答真。 -/
theorem admits_implies_reaches (files : Files) (domain : WriteDomain) (target : Address)
    (admitted : domain.admits files target = .Within) :
    domain.reaches files.protected_name target = true := by
  simp only [WriteDomain.admits] at admitted
  split at admitted
  · cases admitted
  · rename_i reached
    simpa using reached

/-- 空前缀集（只读的角色）什么都不收。 -/
theorem an_empty_domain_admits_nothing (files : Files) (target : Address) :
    (WriteDomain.Everything []).admits files target ≠ .Within ∧
      (WriteDomain.Documents []).admits files target ≠ .Within := by
  constructor <;> simp [WriteDomain.admits, WriteDomain.reaches, WriteDomain.prefixes]

/-- **只写文档的写域永不收计划文件。** `Roadmap.md` 的唯一编辑入口是 `plan` 工具，它保证六列表格重写后仍成立。 -/
theorem a_documents_domain_never_admits_the_plan (files : Files) (prefixes : List Address)
    (target : Address) (plan : files.plan target = true) :
    (WriteDomain.Documents prefixes).admits files target ≠ .Within := by
  simp only [WriteDomain.admits]
  split
  · simp
  · split
    · simp
    · simp_all

/-- 一次写能不能落下：写域收下目标，且目标已经存在时限制是 `Full`。「目标是否已经存在」是写的那一刻文件系统的事实，由写路径原子地判（`crates/runtime/Spec.lean` §8-55），这里是参数。 -/
def permits (files : Files) (domain : WriteDomain) (limit : WriteLimit) (target : Address)
    (exists_already : Bool) : Bool :=
  domain.admits files target == .Within && (!exists_already || limit == .Full)

/-- **限制只收窄、不放宽。** 落得下的写，写域一定收它：`Full` 放不宽楼的写域。 -/
theorem the_limit_never_widens_the_domain (files : Files) (domain : WriteDomain) (limit : WriteLimit)
    (target : Address) (exists_already : Bool)
    (allowed : permits files domain limit target exists_already = true) :
    domain.admits files target = .Within := by
  simp only [permits, Bool.and_eq_true, beq_iff_eq] at allowed
  exact allowed.1

/-- **`Create` 不改动任何已有的文件。** -/
theorem create_changes_no_existing_file (files : Files) (domain : WriteDomain) (target : Address) :
    permits files domain .Create target true = false := by
  simp [permits]

/-- 正常路径可实现：前缀为楼 `lab` 的 `Everything` 收这栋楼里的一个源文件。 -/
example :
    (WriteDomain.Everything [⟨["lab"]⟩]).admits
        ⟨fun segment => segment == ".sprawling", fun _ => true, fun _ => false⟩ ⟨["lab", "a.rs"]⟩ =
      .Within := by
  decide

end Kernel.WriteDomain
