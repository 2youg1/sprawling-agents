-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# gateway::endpoint::stream

规定 `endpoint::stream`（`crates/gateway/src/endpoint/stream.rs`）：逐字读一次调用，结算答案只有一个解析器。本文件是 `crates/gateway/Spec.lean` 的一个分部；下面每一节保留它在 gateway 规格里的标签 §8-n，别处引作 `crates/gateway/Spec.lean §8-n`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `gateway::endpoint::stream` 旁的测试守住。
-/

/-!
### 8-13 逐字读一次调用（形状 3 适配器）

`Endpoint::call_streaming`（crate 内固有方法，`permit::Gated` 的 `Model::call_streaming` 过门后调它）：请求带 `stream: true`，逐行读 `data:`，把每一帧交给 `dialect::increment_of`，最后 `dialect::settled_from_stream` 把收集到的帧重装成**这个 dialect 非流式的那个形状**，再交给同一个 `response_from_wire`。

**「逐行」指的是响应体到达的节奏，而不是一个已经读完的字符串的行。** `Response` 当 `std::io::Read` 包进 `BufReader` 逐行读，一帧到达即交一帧；先把整个 body 读完再逐行转发，会让全部增量在模型停下之后的同一毫秒里一起发出。否决「把转发搬到 socket 任务那一层去查锁」：`to_watchers` 是非阻塞广播，帧压根没有到达那里，往下游找只会找到一个不存在的原因。**验收形式**：假供应方先写开头几帧并 flush，**然后等调用方回报「第一条增量已转出」才写剩下的**；读完再回放的实现永远回报不了，服务器自己就是断言，测试侧不读时钟。

**结算答案只有一个解析器。** 直接把流读成 `ChatResponse` 会立刻长出第二个权威：同一个回复，流式路径与阻塞路径可能得出两个结论。重装成非流式形状是这条口径的全部实现。

**不理会 `stream: true` 的供应方照样作答。** 流式请求的响应头里，`content-type` 的媒体类型是 `application/json` 时，这份 body 就是一个已定答案：`stream` 把它交给阻塞路径同一个 `response_from_wire`，不当帧读，返回的 `ModelReturn` 与 `call` 对同一份 body 的返回相等。不少 OpenAI 兼容的服务端（本地的居多）不理会这个字段，整段作答；当帧读，它一帧也没有，调用便以「流在结算帧前结束」失败，一次已完整到达的回答被当成截断，watchdog 随之退避重试。媒体类型缺席或是 `text/event-stream` 时照帧读，因为请求要的是流。落选的是「嗅探首行是 `{` 还是 `data:`」：HTTP 已经用 content-type 说了 body 是什么，另立一套判定只会与它分歧。

**`increment_of` 认散文与推理两条流，别的一律不报。** 一帧答 `Increment::Said`（散文）或 `Increment::Thought`（推理），两条流由 `kernel::Increment` 分开，读者不必猜一段字来自哪条。各 dialect 各读各的：Anthropic 读 `delta.type` 为 `text_delta` 的 `delta.text` 与 `thinking_delta` 的 `delta.thinking`；OpenAI chat 读 `choices[0].delta.content`，没有正文时读推理（`reasoning` 或 `reasoning_content`，两种网关两种拼法）；Responses 读 `response.output_text.delta` 与推理那一种事件的 `delta`。**工具参数、signature 与别的帧一律不报**：半个工具参数不是短一点的工具参数，签名是替 provider 转交核验用的、不是拿来读的。它不返回 `Result`——一个读不出来的增量就是不显示的增量，一个显示细节不得有能力弄失败一次本来正常的调用。

**思考块的 signature 与文本走两条 delta，两条都要收。** Anthropic 把一个 thinking 块拆成 `thinking_delta`（正文）与 `signature_delta`（签名）两串增量，而 `content_block_start` 给出的那份 signature 恒为空串。`settled` 因此按 index 累积 signature 并在重装时写回，与 `partial_json` 同形。**空 signature 在 `block_from` 升为 `E_WIRE_MISMATCH`**：provider 拿签名去核验它自己发出的那段推理，空的那份带进下一回合就是一个 400，而这座城此刻还说不出为什么——拒在产生它的那一回合，报的才是「流把签名丢了」。两条往返（settled→`response_from_wire`→`request_wire`）逐字节相等由 `anthropic/stream.rs` 的测试钉住。

**半个工具参数恒拒，两家同码同形。** 流中断时 `partial_json`／`arguments` 停在一个值的中间；把它读成 `{}` 就是把半次调用变成一次真的「无参数调用」，而 `exec {}`／`write {}` 会落账、过门、真执行。判定住 `mismatch::settled_tool_arguments(tool, at, raw)`——两家 dialect 都从碎片拼参数，故拼完即校验的那一句只有一处，拒词点名工具、index、已收字符数与解析停在哪里，码取 `E_PROVIDER`（`stream_cut`，`Retry::Unknown`：截断的流与截断的 body 是同一种失败）。落选的是「在 OpenAI 侧沿用 `response_from` 的 `E_WIRE_MISMATCH`」：形状没有漂，漂的是这次传输，而 `E_WIRE_MISMATCH` 的恢复语会让人去改 dialect 与 base url 两个没有错的设置。

**一个工具调用在它的 `content_block_stop` 到达时就是完整的，解码器在那一刻交出它。** `anthropic::stream::completed_call(frames, at) -> Result<Option<ToolCall>, AxError>`：`frames` 是迄今收到的帧，`at` 是刚收到 `content_block_stop` 的那个 index。块是 `tool_use` 则答 `Some(ToolCall)`，别的块答 `None`；参数停在一个值的中间照旧是 `E_PROVIDER`（同 `settled_tool_arguments`）。**重装只有一处**：`settled` 与 `completed_call` 都经 `rebuilt` 把帧按 index 拼回块，再由 `block_from` 读成 `ContentBlock`——提前交出的调用与结算后 `ModelReturn` 里那一条逐字段相等，这由测试钉住。**交出的不是 `Increment`**：`Increment` 在线协议上、只供人看、不许任何下游据以决策（kernel `Increments` 的约定），而一次提前交出的调用恰恰是要据以执行的；把它塞进 `Increment` 既要动 `WIRE_V`，又会让「看的东西」变成「决定的东西」。`Endpoint::stream` 每收到一帧就问 `dialect::call_completed_by(kind, frames)`：Anthropic 兼容格式在最后一帧是 `content_block_stop` 时经 `completed_call` 作答，别的兼容格式恒答 `None`；答出的调用交给 `call_speculating` 的 `early`。

**提前交出的调用不是历史**：账本仍只从结算后的 `ModelReturn` 记 `tool_called`；回答被截断或取消，提前交出的调用与据它得出的结果一并丢弃。这两句与「只提前启动第一个写调用之前的只读调用」由 `crates/runtime/spec/Turn/Speculation.lean` 证明：`speculation_is_not_an_event`、`speculation_keeps_serial_order`、`a_cut_answer_discards_its_cache`；`speculating_past_a_write_changes_the_ledger` 给出落选设计（写调用之后的读也提前启动）的反例——提前启动的读看到的是写之前的世界。

**认不出的帧跳过，缺失的结算帧不跳过。** provider 会加新的事件类型，一个人不该因为其中一个是新的就丢掉整次调用；但流在说明「为什么停」的那一帧之前结束，是 `Provider` 失败并且可重试——它和一个被截断的 body 是同一种失败，刻意不允许「保留已收到的增量」来补救：把不完整的回复当成完整的呈现出去，是这里唯一不能有的结局。

**机密楼宇的拒绝写一次。** 两扇门（`call` 与 `call_streaming`）都说同一句话，出自同一个 `confidential_refusal`——一条安全拒绝有两份拷贝，就是两个各自变软的机会。
-/
