-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# gateway::transcribe

规定 `transcribe`（`crates/gateway/src/transcribe.rs`）：把一段录音变成一行字的可选设施。本文件是 `crates/gateway/Spec.lean` 的一个分部；下面每一节保留它在 gateway 规格里的标签 §8-n，别处引作 `crates/gateway/Spec.lean §8-n`。
-/

/-!
### 8-12 gateway::transcribe（`transcriber` 形状 4 适配器，`recording` 形状 2 值，`wire` 形状 1 判定）

把一段录音变成一行字的那个 provider 端点。**它是可选设施**：没配的城照常跑完每一件事，只是在有人开口说话时明说自己听不见，而不是在兑付那一格炸开或回一个空串。

```rust
// gateway::transcribe（索引，无逻辑）
pub use chosen::transcriber_for;
pub use recording::{AudioType, Recording};
pub use transcriber::Transcriber;               // TranscriberConfig 在 crate 内用，不出 crate

// transcribe/chosen.rs（形状 1 装配，`adapter_for` 的孪生）
pub fn transcriber_for(chosen: &Chosen<'_>, secrets: SecretResolver)
    -> Result<Transcriber, AxError>;    // TRANSCRIBE_TIMEOUT_MS = 120_000

// transcribe/recording.rs（形状 2 值）
pub enum AudioType { Webm, Ogg, Mpeg, Mp4, Wav }
impl AudioType {
    pub const ALL: [AudioType; 5];                // 这座城发得出去的全部容器，拒词由它列出
    pub fn media_type(self) -> &'static str;   pub fn file_name(self) -> &'static str;
    pub fn of_media_type(raw: &str) -> Result<AudioType, AxError>;  // 认不得的容器＝E_INVALID_ARGS
    pub fn of_file_name(name: &str) -> Result<AudioType, AxError>;  // §8-33；认不得的扩展名＝E_INVALID_ARGS
    pub fn of_signature(head: &[u8]) -> Result<AudioType, AxError>; // §8-34；开头的字节认不出容器＝E_INVALID_ARGS
}
pub struct Recording { /* bytes、kind —— 私有 */ }
impl Recording {
    pub fn new(bytes: Vec<u8>, kind: AudioType) -> Result<Recording, AxError>;  // 空／越顶＝E_INVALID_ARGS
    pub fn read_from(reader: impl std::io::Read, kind: AudioType)
        -> Result<Recording, AxError>;   // §8-33；读到上限多一字节为止，读失败＝E_STORAGE_FATAL
    pub fn read_unlabelled(reader: impl std::io::Read)
        -> Result<Recording, AxError>;   // §8-34；同一个上限，容器由 of_signature 从读到的字节认
    pub fn kind(&self) -> AudioType;   pub fn len(&self) -> usize;
}

// transcribe/transcriber.rs（形状 4 适配器）
pub struct TranscriberConfig { pub base_url: String, pub model: String,
                               pub auth: AuthSpec, pub timeout_ms: u64 }
pub struct Transcriber { /* attached: Option<Endpoint> —— 私有 */ }
impl Transcriber {
    pub fn absent() -> Transcriber;                       // 这座城没有这项设施
    pub fn attach(config: TranscriberConfig, secrets: SecretResolver) -> Result<Transcriber, AxError>;
    pub fn is_attached(&self) -> bool;
    pub fn transcribe(&self, recording: &Recording) -> Result<String, AxError>;
}
```

**没配就是一句具名的拒绝，不是一个空串。** `Transcriber::absent()` 上的 `transcribe` 恒返回三段式 `E_TOOL_UNAVAILABLE`：action ＝ `transcribe a recording`，subject ＝ `this city has no transcription endpoint attached`，recovery 指出两条人能立刻做的路（登记一个服务 `audio/transcriptions` 的 endpoint，或者改用打字）。**为何复用 `E_TOOL_UNAVAILABLE` 而不新增一码**：基表里这一码的语义正是「这次部署里没有这项设施」，而 `E_CONFIG_INVALID` 会说成人填错了什么——什么都没填错，这项设施本就是可选的。这项设施有两条路用它：界面的 `/transcribe`（composer 的麦克风），与城给 run 的工具 `transcribe`（sprawling-SPEC 8-131）；两条路缺的是同一项设施，故同一个码，`E_BROWSER_UNAVAILABLE` 那样只属于一件工具的专码在这里会给同一个事实第二个名字。码表是 kernel 全城权威且按「能否定义掉」逐码守着，为一件已有码能如实表达的事把它撑大，就是给同一个事实立第二个名字。

**凭据只有一条路，请求也只有一条。** `Transcriber` 内部持一个真的 `Endpoint`（`DialectKind::OpenAi`、`Redemption::without_images`、`pricing: None`），整次 POST 由 `Endpoint::post_bytes` 发（§8-2），认证头由 `Endpoint::authorize` 写——与聊天调用、与 `list_models` 探测是同一格兑付。头名由 `AuthSpec::for_dialect` 定（§8-9），登记面不自己在 Bearer 与具名头之间选。**恒不为转写开第二个持凭据的地方**：两处持凭据就是两处会漏。

**哪个端点答这类活，由账本说了算，装配只有一处。** `transcriber_for` 与 `adapter_for` 是同一句话的两半：`EndpointBook::select` 给出 `Chosen`，这两个自由函数各自把那个选择变成一件可调用的设施。调用方自己拼一个 `TranscriberConfig`，就是给「base URL 与凭据在哪里合流」立第二个地点。**上传的录音，容器从 content-type 读而不从文件名猜**：浏览器录进哪一种容器只有它知道，`AudioType::of_media_type` 是那一步的唯一入口，认不得的容器当场拒。放在文件里的录音没有 content-type，容器从文件名读（§8-33）。

**路径归兼容格式。** 人填 base URL（provider 文档就那么印），`audio/transcriptions` 由 `transcribe::wire` 拼，拼法复用 `router::attached::join`（`pub(crate)`）——「base URL 加上兼容格式自己的路径」在城里只有一个算法。

**多部分请求体自写，不引 reqwest 的 `multipart` feature。** 本 crate 自写线格式是既定选型（§10），而 `multipart/form-data` 只是两个字段加一条分界线；引 feature 要动根清单与 `Cargo.lock` 两个共享权威，换来的代码比自写的还多。分界线是**确定性的**：常量种子起头，只要它作为子串出现在录音里就加一个 `-` 再试，录音有限故必然终止；同一份 `(model, Recording)` 因此永远拼出同一串字节，重放能重推当时实发的请求。

```rust
// transcribe/wire.rs（形状 1 判定，纯函数）
pub(crate) struct FormBody { pub(crate) content_type: String, pub(crate) bytes: Vec<u8> }
pub(crate) fn form_body(model: &str, recording: &Recording) -> FormBody;
pub(crate) fn transcription_path() -> &'static str;                       // "audio/transcriptions"
pub(crate) fn transcription_of(wire: &serde_json::Value) -> Result<String, AxError>;
```

- 请求体两个字段：`model`（纯文本）与 `file`（`filename` 取自 `AudioType`，`Content-Type` 取自同一处——**扩展名与 media type 是同一个事实的两面，故住同一个枚举**，provider 两边都看）。`response_format` 不写：默认就是 `{"text": …}`，而写一个与默认相同的字段是给同一件事立第二个权威。
- 读答案只认一个键：`text` 缺席或非字符串＝`E_WIRE_MISMATCH`（走 `mismatch::require`／`as_str`，与两家 dialect 同一批读取器）；非 2xx＝`E_PROVIDER`，subject 只写 URL 与状态码，恒不回显对侧正文。
- **空转写是合法答案**：一段静音本来就该转出空串。空串只在「没有端点」那条路上被禁止，而那条路根本不返回 `Ok`。

**为何 `Recording` 是值而不是一对参数**：字节与它的格式永远同行，且两条不变量（非空、不超 `RECORDING_MAX_BYTES`）只在 `new` 一处守；无 setter。**为何 `wire` 与 `transcriber` 分家**：「这段多部分请求体长什么样」是纯数据的判定、可逐字节断言，「怎么把它发出去并兑付凭据」要一个 socket——两件事变化的理由不同。

**线上的入口是 `wire` 的 `POST /transcribe`**（`crates/wire/src/reception/admission.rs`），经 `TranscribeSink` 交到这里；它不是 `Command` 的变体。
-/

/-! D12 转写是一项设施，两条路用它

决定：`transcribe` 既是 composer 麦克风背后的设施，也是城给 run 的一件工具（sprawling-SPEC 8-131）；两条路读端点账本里 `ModelTag::Transcribe` 的同一个选择，经同一个 `transcriber_for` 造设施；工具只在那个选择成立时上 run 的工具表。理由：computer use 要能把声音变成字，而二进制里不内置任何模型（定规），于是模型要转写只能经人接入的这个端点；`Transcriber::absent()` 的码仍是 `E_TOOL_UNAVAILABLE`，语义不变，仍是「这次部署里没有这项设施」。被否的备选：给工具另立一个专码（同一个事实两个名字）；让转写只做界面设施（模型拿不到任何转写能力）。
-/
