-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::conversation

规定 `conversation`（`crates/runtime/src/` 下同名的文件）。会话历史：已发出的消息不再被改写。本文件是 `crates/runtime/Spec.lean` 的一个分部；下面每一节保留它在 runtime 规格里的标签 §8-n，别处引作 `crates/runtime/Spec.lean §8-n`。
-/

/-!
### 8-47 runtime::conversation：已发出的消息不再被改写，空回复之后的工具结果除外（形状 2 值类型）


```rust
impl Conversation {
    pub fn mark_sent(&mut self);   // 本次组装发出了 messages() 的全部；之后到达的 user 文本不再并入其中任何一条
}
```

- **规则**：`Conversation` 记下上次组装发出了几条消息（`sent`）。user 文本（steer、提醒）只并入**尚未发出**的最后一条 User 消息；最后一条 User 消息已经发出时，文本进一个待投槽（`held`），由下一次 `push_tool_results` 接在这一波结果之后，即词汇表里 Steer 的落点「下一份工具结果的末尾」。待投槽不在 `messages()` 里，所以它永远不会出现在一条它到达之前就已组好的请求中。
- **调用点**：活的 run 在 `Turn::assemble` 答出 `Advanced` 之后调一次（`run::lifecycle`）；`fork` 在读到本 run 的 `prompt_shape_compared` 时调一次：`prompt_assembled` 每个 run 只写一次（8-39），而 `prompt_shape_compared` 是每一回合组装之后紧接着写的那一行。两边的标记来自同一个事实（这一回合的请求组好了），所以分支按同一规则重放出同样的字节。
- **理由**：`BeforeCall`／`BeforeWave`／`BeforeToolCall`／`BeforeSpawn` 四个安全点都在组装之后，那时窗口最后一条仍是刚随请求发出的 User 消息。把 steer 并进去，下一次请求里 steer 排在一条没读过它的助手回复之前：模型看到的时间顺序是假的，而且已发出消息的字节变了，provider 的前缀缓存从这条消息起全部失效。
- **字节**：`tools/fixtures/golden-p0` 的剧本在第 0 回合收到 steer，它第二次请求的 run 区域在工具结果之后带着这条 steer。
- **例外：空回复之后的工具结果**：一条没有内容的回复不推助手消息，所以随后的 `push_tool_results` 碰到的最后一条仍是已发出的 User 消息。结果和待投文字并进这条消息，它的字节因此变了，provider 的前缀缓存从这条消息起失效。这里接受改写，因为另一条路是在它之后另开一条 User 消息，即被否的①：两条相邻的 User 消息是入口不变量要排除的形状。时间顺序仍然是真的：这条消息之后没有模型读过的回复。改写之后这条消息重新算作未发出（`sent` 退到它之前），所以在下一次组装之前到达的 steer 并进它，排在这批结果之后，不再多等一波。
- **一条回复没有任何调用时**：run 就此结束（8-37），待投文字不再有下一次组装；它已由 `steer_received` 入账，账本仍是它的来历。
- **fork 的切点落在一波之内时**：这一波整波丢弃（半个交换没有 provider 接受），分支只继承 `messages()`，待投槽里的文字不随分支走。待投文字只在「组装之后、这一波结果之前」存在，所以它针对的正是被丢弃的那一波；分支从没看到那一波，把它接到分支的第一条消息里，模型会读到一句指向不存在的上下文的话。这段文字已由 `steer_received` 入账，母 run 的账本仍是它的来历。被否：让 `Inherited` 带上待投文字——分支的首条 User 消息会以一句针对别人那一波的 steer 开头。
- **被否**：①在已发出的 User 消息之后另开一条 User 消息——两条相邻的 User 消息正是本模块入口不变量要排除的形状；②由执行器在工具结果之后再调一次 `push_steer`——待投状态会住在 `Conversation` 之外，`fork` 要复刻第二份同样的记忆，两个家会漂移。
-/
