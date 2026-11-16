-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# kernel::discard

规定 `kernel::discard`（`crates/kernel/src/discard.rs` 与 `crates/kernel/src/discard/` 下的 `request`、`verdict`、`forecast`）：删除的还原方案、Discard 门的判定表与 exec 的预判。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。
-/

/-!
### 8-26 kernel::discard

```rust
pub enum Restoration { Tracked(Locator), Interred(Locator), Rebuildable { reason: String } }
pub struct Discard { /* paths, plan, taint, total_bytes —— 私有 */ }
impl Discard {
    /// Sole constructor (C14): restoration mandatory and scheme-checked —
    /// Tracked wants file:, Interred wants cas:, Rebuildable wants a
    /// non-empty reason; violations are E_DISCARD_IRREVERSIBLE.
    pub fn new(paths: Vec<Address>, plan: Restoration, taint: TaintSet, total_bytes: ByteLen)
        -> Result<Discard, AxError>;
    pub fn paths(&self) -> &[Address];  pub fn plan(&self) -> &Restoration;
    pub fn taint(&self) -> &TaintSet;   pub fn total_bytes(&self) -> ByteLen;
}

pub enum DiscardRequest { Planned(Discard),
                                            Unplanned { paths: Vec<Address>, taint: TaintSet, total_bytes: ByteLen } }
pub enum DenyReason { NoRestoration, Tainted }
pub enum DiscardVerdict { Allow, Deny { reason: DenyReason } }
/// The discard door's decision table, sole authority —
/// gate::discard delegates wholly and only shapes the refusal.
pub fn decide(req: &DiscardRequest) -> DiscardVerdict;

pub enum DiscardForecast { Clear, Suspected { pattern: String } }
pub fn forecast(arm: &ExecArm) -> DiscardForecast;
```

- **decide 表**：Unplanned → Deny{NoRestoration}（无还原不可构造，使 Planned 恒有 plan）；Planned 且 taint 非空 → Deny{Tainted}（恒，无视规模）；余 Allow。判序固定，确定可重放。
- **规模与归属不改答（D1「默认 YOLO」）**：一次 Planned 删除恒带 `Restoration`，因此恒可回滚；文件数、字节数、别人登记过的 asset 都不让删除停下来问人。删除的上界另有家：write domain 决定一个 resident 够得到哪些文件，registry 保存把 asset 放回去所需的凭据。
- **forecast 三臂预判力递减**：Program 读 `(path, args)` 整体——取 path 的末段并去掉 `.exe` 后缀，basename ∈ {rm, rmdir, del}，或 git 的参数里有 `reset --hard` 或 `clean`，或 find 的参数里有 `-delete`；Python 与 Shell 只能查子串，可被混淆绕过，恒保守：两臂共用文本表（`rm `、`rmdir`、`-delete`、`git reset --hard`、`git clean`），Python 另查 `os.remove`、`shutil.rmtree`、`os.unlink` 与以写模式打开文件（`open(` 与 `'w'` 或 `"w"` 同在），Shell 另查截断重定向（含 `>` 而全文不含 `>>`）。git 检查点兜底（`storage::checkpoint`）。两张子串表是 pub(crate) 数据面。
- 同文件 `#[test]` 守 Discard 门 fail-closed：Unplanned 恒 Deny；Tainted 恒 Deny（kani 不接手，理由见 `crates/kernel/Spec.lean` §2）。
-/

namespace Kernel.Discard

/-- 删除为什么被拒，与 `kernel::DenyReason` 逐变体同名。 -/
inductive DenyReason where
  | NoRestoration
  | Tainted
  deriving DecidableEq, Repr

/-- Discard 门的答案，与 `kernel::DiscardVerdict` 逐变体同名。 -/
inductive DiscardVerdict where
  | Allow
  | Deny (reason : DenyReason)
  deriving DecidableEq, Repr

/-- 到门口的请求，与 `kernel::DiscardRequest` 逐变体同名。`Planned` 带着一份还原方案（`Discard::new` 是唯一构造点，没有还原方案拼不出来），`Unplanned` 是 exec 预判路上的请求，文本预判铸不出计划。两者都带 taint 与总字节数；路径不进判定，模型里省去。 -/
inductive DiscardRequest where
  | Planned (tainted : Bool) (total_bytes : Nat)
  | Unplanned (tainted : Bool) (total_bytes : Nat)
  deriving DecidableEq, Repr

/-- `discard::decide`，Discard 门的判定表，唯一权威；`gate::discard` 整个委派给它，只塑形拒词。判序固定：没有计划先拒，再看 taint。 -/
def decide : DiscardRequest → DiscardVerdict
  | .Unplanned _ _ => .Deny .NoRestoration
  | .Planned true _ => .Deny .Tainted
  | .Planned false _ => .Allow

/-- 没有计划的删除恒被拒。 -/
theorem an_unplanned_discard_is_always_denied (tainted : Bool) (total_bytes : Nat) :
    decide (.Unplanned tainted total_bytes) = .Deny .NoRestoration :=
  rfl

/-- 外来内容要求的删除恒被拒，不论大小。 -/
theorem a_tainted_discard_is_always_denied (total_bytes : Nat) :
    decide (.Planned true total_bytes) ≠ .Allow := by
  simp [decide]

/-- **规模不改答（D1「默认 YOLO」）。** 一次有计划的删除恒可回滚，所以文件数与字节数都不让它停下来问人。 -/
theorem size_never_changes_the_answer (request : DiscardRequest) (bytes : Nat) :
    decide (match request with
      | .Planned tainted _ => .Planned tainted bytes
      | .Unplanned tainted _ => .Unplanned tainted bytes) = decide request := by
  cases request with
  | Planned tainted _ => cases tainted <;> rfl
  | Unplanned _ _ => rfl

/-- **放行的删除都带着还原方案且不带 taint。** -/
theorem an_allowed_discard_is_planned_and_clean (request : DiscardRequest)
    (allowed : decide request = .Allow) : ∃ bytes, request = .Planned false bytes := by
  cases request with
  | Planned tainted bytes =>
    cases tainted
    · exact ⟨bytes, rfl⟩
    · simp [decide] at allowed
  | Unplanned _ _ => simp [decide] at allowed

end Kernel.Discard
