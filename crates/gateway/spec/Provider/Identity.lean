-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# gateway::provider::identity

规定 `provider::identity`（`crates/gateway/src/provider/identity.rs`）：一个模型跨供应方的身份。本文件是 `crates/gateway/Spec.lean` 的一个分部；下面每一节保留它在 gateway 规格里的标签 §8-n，别处引作 `crates/gateway/Spec.lean §8-n`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的规则只在 `provider::identity::canonical` 一处实现。
-/

/-!
### 8-38 同一个模型来自几家供应方时，怎样认出它是同一个（形状 1 判定）

```rust
pub fn canonical(facts: &ModelFacts) -> String;
```

- **规则只有一条，只在这里**：上游说出它自己的规范 id 时用它（OpenRouter 的 `hugging_face_id`；它的 `canonical_slug` 带着厂商自己的 id 没有的发布日期，读它会把回退规则认作同一个的两行分开，所以不读），否则用模型 id；无论哪一种，都去掉最后一个 `/` 之前的组织前缀，再转成小写。`anthropic/claude-sonnet-5` 与 `claude-sonnet-5` 因此是同一个模型，`deepseek-ai/DeepSeek-V4` 与 `deepseek-v4` 也是。两行是同一个模型，当且仅当两个字符串相等；不做模糊匹配，因为一次错认会把两个价格、窗口都不同的模型合成选择器里的一行。
- **答复里每一行都带它**：`wire::ModelFactsSummary.canonical`（`crates/wire/Spec.lean` §8-92），页面按它合并选择器的模型列表，并在旁边列出每个供应方自己的 id；页面不自己算。
- **读出它的地方**：`endpoint::models` 把它读进 `ModelFacts.canonical`，本函数先看它。
- **被否**：①按名字相似度合并（编辑距离、去掉版本后缀）：`gpt-5` 与 `gpt-5-mini` 会被认作同一个；②让页面合并：同一条规则在 TypeScript 与 Rust 各写一份，CLI 的 `/model` 补全也要第三份。
-/
