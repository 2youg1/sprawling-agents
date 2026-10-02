-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# accounting::held_vault 与 accounting::toolkit_broker

规定 `crates/accounting/src/held_vault.rs` 与 `crates/accounting/src/toolkit_broker.rs`。本文件是 `crates/accounting/Spec.lean` 的一个分部；下面每一节保留它在 accounting 规格里的标签 §8-n，别处引作 `crates/accounting/Spec.lean §8-n`，决定引作 `accounting D<n>`。
-/

/-!
### 8-9 accounting::held_vault 与 accounting::toolkit_broker：读面与写者共用的两组事实（形状 2 值类型）

```rust
// accounting::held_vault
pub fn resolving(vault: Arc<Mutex<gateway::Custodian>>) -> gateway::SecretResolver;
pub fn poisoned_vault() -> AxError;   // E_STORAGE_FATAL：vault 的锁中毒
// accounting::toolkit_broker
pub fn broker_for(/* toolkit 地址、城根、vault */) -> Result<Option<(agent_protocols::Broker, String)>, AxError>;
```

- **一个锁着的 vault 的解析器与锁中毒时的拒绝各只有一处**：装配点、读面与 serving 都要一次性的解析器，拒绝的措辞只写一次。
- **broker 的钥匙登记在哪、这座城对 broker 是谁，页面与命令读同一组事实**：连接动作 `connect_toolkit` 仍是 worker 的。
-/
