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
