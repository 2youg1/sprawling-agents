-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# kernel::delegation

规定 `kernel::delegation`（`crates/kernel/src/delegation.rs`）：委派一层深的两层守卫。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。
-/

/-!
### 8-17 kernel::delegation

```rust
pub enum DelegateKind { Resident, Ephemeral }
impl DelegateKind { pub fn as_str(self) -> &'static str; }   // 一个词一个权威（工具解析与 status 打印同源）
/// Depth-zero position; the only type with a delegate method.
pub struct Delegator(/* 私有单元 */);
impl Delegator { pub fn root() -> Delegator;                     // 铸造点：装配/citysim
                 pub fn delegate(&self, kind: DelegateKind) -> Delegate; }
pub struct Delegate { /* kind —— 私有 */ }                        // 无 delegate 方法：trybuild 反例
impl Delegate { pub fn kind(&self) -> &DelegateKind; }

pub enum Depth { Root, Delegated }
pub enum DelegationVerdict { Allow, Deny }
/// Dynamic half of the two-layer guard (static half = the missing method).
pub fn admit(parent: Depth, kind: &DelegateKind) -> DelegationVerdict;   // Delegated 恒 Deny
```

- `root()` 公开是诚实的承认：类型封的是「从 Delegate 值铸子代」这条路，「谁持有 Delegator」由装配纪律看守；动态 `admit` 是第二层（「深度两层拦」）。
- `E_DELEGATION_DEPTH` 不消解；塑形在 gate::spawn（gate 码唯一生产者）。
-/

namespace Kernel.Delegation

/-- 一次委派派出的是什么，与 `kernel::DelegateKind` 逐变体同名。 -/
inductive DelegateKind where
  | Resident
  | Ephemeral
  deriving DecidableEq, Repr

/-- 发起者站在哪一层，与 `kernel::Depth` 逐变体同名。 -/
inductive Depth where
  | Root
  | Delegated
  deriving DecidableEq, Repr

/-- 与 `kernel::DelegationVerdict` 逐变体同名。 -/
inductive DelegationVerdict where
  | Allow
  | Deny
  deriving DecidableEq, Repr

/-- 两层守卫的动态一半（`delegation::admit`）；静态一半是 `Delegate` 没有 `delegate` 方法。 -/
def admit : Depth → DelegateKind → DelegationVerdict
  | .Root, _ => .Allow
  | .Delegated, _ => .Deny

/-- 一层深：被派生的位置再派生恒被拒（`E_DELEGATION_DEPTH`，由 `gate::spawn` 塑形），派的是什么都一样。 -/
theorem a_delegate_never_delegates (kind : DelegateKind) : admit .Delegated kind = .Deny :=
  rfl

/-- 正常路径可实现：深度零位派得出两种委派。 -/
theorem the_root_delegates_either_kind (kind : DelegateKind) : admit .Root kind = .Allow :=
  rfl

end Kernel.Delegation
