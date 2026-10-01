-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# kernel::registry

规定 `kernel::registry`（`crates/kernel/src/registry.rs`）：产物、资产与居民的登记。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。
-/

/-!
### 8-18 kernel::registry

```rust
pub struct ResidentId(String);               // 非空；`role@building.n` 文法 P1 随 city::resident 收紧
pub struct Claim { pub locator: Locator, pub by: String }        // 证词：未验证产出
pub struct Artifact { /* locator, verified_by —— 私有 */ }
impl Artifact {
    /// Sole constructor: player–referee in the type. Verification evidence
    /// must cite a kind in `completion::CITABLE`, else E_EVIDENCE_MISSING.
    pub fn verify(claim: Claim, evidence: EventRef) -> Result<Artifact, AxError>;
    pub fn locator(&self) -> &Locator;  pub fn verified_by(&self) -> &EventRef;
}

pub struct Registry { /* artifacts: BTreeMap<String, Artifact>, assets: BTreeSet<String>,
                         residents: BTreeSet<ResidentId> —— 私有 */ }
pub enum RegisterVerdict { Registered, AlreadyRegistered }
impl Registry {
    pub fn new() -> Registry;
    pub fn register_artifact(&mut self, artifact: Artifact) -> RegisterVerdict;   // 键＝locator 规范拼写
    pub fn promote_asset(&mut self, locator: &Locator) -> Result<RegisterVerdict, AxError>; // 未登记 → E_PATH_NOT_FOUND
    pub fn register_resident(&mut self, id: ResidentId) -> RegisterVerdict;
    pub fn artifact(&self, locator: &Locator) -> Option<&Artifact>;
    pub fn is_asset(&self, locator: &Locator) -> bool;           // Discard 门的查询面
}
```

- Registry 是值不是存储：状态住调用方；kernel 只定登记规则与查询面。
- 评分归 eval（P3）；promotion 只登记不评分。
-/
