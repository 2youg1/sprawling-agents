-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# gateway::ocr

规定 `ocr`（`crates/gateway/src/ocr.rs`）：把一张图变成一行字的可选设施；没有名字的录音从开头的字节认容器。本文件是 `crates/gateway/Spec.lean` 的一个分部；下面每一节保留它在 gateway 规格里的标签 §8-n，别处引作 `crates/gateway/Spec.lean §8-n`。
-/

/-!
### 8-34 一张图变成一行字：`gateway::ocr`（`recogniser` 形状 4 适配器，`picture` 形状 2 值，`chosen` 形状 1 装配）；没有名字的录音（`transcribe::recording` 的 `of_signature` 与 `read_unlabelled`）

computer use 读不到字的窗口（画在画布上的界面、远程桌面）只剩截图，模型要文字只能经人接入的一个能读图的模型（二进制里不内置任何模型，定规）。人为 `ModelTag::Ocr` 选的那个模型就是它；本节把那一次选择变成一项设施，与 §8-12 的转写同形。

```rust
// gateway::ocr（索引，无逻辑）
pub use chosen::recogniser_for;
pub use picture::Picture;
pub use recogniser::Recogniser;

// ocr/chosen.rs（形状 1 装配，`adapter_for` 与 `transcriber_for` 的孪生）
pub fn recogniser_for(chosen: &Chosen<'_>, secrets: SecretResolver,
                      dialect_headers: Vec<(String, String)>) -> Result<Recogniser, AxError>;

// ocr/picture.rs（形状 2 值）
pub struct Picture { /* seen: ImageRef、bytes —— 私有 */ }
impl Picture {
    // seen.locator 必须是 bytes 的 `cas:` 哈希（不带范围），否则 E_INVALID_ARGS。
    pub fn new(seen: ImageRef, bytes: Vec<u8>) -> Result<Picture, AxError>;
}

// ocr/recogniser.rs（形状 4 适配器）
pub struct Recogniser { /* attached: Option<…> —— 私有 */ }
impl Recogniser {
    pub fn absent() -> Recogniser;                        // 这座城没有这项设施
    pub fn is_attached(&self) -> bool;
    pub fn recognise(&mut self, picture: Picture, policy: &BuildingPolicy) -> Result<String, AxError>;
}
```

- **发给模型的是所选 face 的图片内容，不另造 OCR 协议。** `recogniser_for` 经 `adapter_for` 造出与 run 同一种适配器：chat 面写 `image_url`，Responses 面写 `input_image`，Messages 面写 `image` 块，各由 dialect 一处写（§10 的设计 A）。请求是一条 system（读出图里的全部文字，只交文字）加一条 user 消息（这张图与一句要求），没有工具；上限是这个选择登记时的 `max_output_tokens`，即 `select_model` 那条事实梯的答案（§8-17），不另定一个数。专用 OCR 服务（一条不是对话的线）不在这一版：要接它时，它是 `Recogniser` 的第二种 attached，重议参数是人要接一个不说三种 face 之一的 OCR 端点。
- **图的字节经兑付那一格进请求，不进账本，也不进 CAS。** 适配器的 `ImageResolver` 读的是 `Recogniser` 自己的一格：`recognise` 把这张 `Picture` 放进去，调用返回后取走；兑付只认 locator 与格里那张图相同的请求。`Picture::new` 守住 locator 就是字节的哈希，所以兑付交出的字节与 locator 说的是同一份。不经 CAS：楼里的一个 PNG 文件读进来就够发，为发一次再存一份，读就成了写。`recognise` 取 `&mut self`，一次只有一张图在格里。
- **能不能把图交给这个模型，由端点判。** 登记的 `input` 是 §8-37 那架梯子的答案。选中的模型登记为只读字（`InputKinds::Text`）时，适配器在发出前拒绝（`E_INVALID_ARGS`，恢复语让人把这个标签指向一个 `text_image` 的模型；`endpoint::model` 发出前的那道检查）；本模块不再判一遍。`policy` 照 run 的调用交给适配器，机密楼的那道兜底拒绝因此与主模型同一处。
- **答复只读文字。** 答复里的文字块按序以换行连起来，推理块与别的块不读；一个文字块都没有就是空串。一张没有字的图答空串是合法的：那是模型读到的东西，而一个说不清的答复在 dialect 解读时已经是 `E_WIRE_MISMATCH`。
- **没配就是一句具名的拒绝。** `Recogniser::absent()` 上的 `recognise` 恒返回 `E_TOOL_UNAVAILABLE`：action ＝ `read the text in a picture`，subject ＝ `this city has no OCR model chosen`，恢复语让人为 `ocr` 选一个能读图的模型，或改用窗口的 accessibility tree。码的理由同 §8-12：这次部署里没有这项设施。
- **没有名字的录音，容器从开头的字节认。** 连接器把声音块存进 CAS 时不记它的 media type（runtime D15），块只有字节。`AudioType::of_signature` 看开头：`RIFF`…`WAVE` 是 `Wav`，`OggS` 是 `Ogg`，EBML 头 `1A 45 DF A3` 是 `Webm`，第 4 到 8 字节是 `ftyp` 的是 `Mp4`，`ID3` 或 MPEG 帧同步（`FF`，次字节高三位全 1）是 `Mpeg`。每一种的认法写在 `AudioType` 的一个穷尽 `match` 里，第六种容器进来时编译器要它的认法；认不出＝`E_INVALID_ARGS`，拒词由 `ALL` 列出五种。`Recording::read_unlabelled` 读到同一个上限多一字节为止，再从读到的字节认容器。被否：让模型在参数里写 media type（模型抄错一个字，provider 就以 400 拒一段好录音）；连接器把 media type 存在块旁边（CAS 只存字节，来源记录只记楼与 run，storage-SPEC §8-3，多一个字段是 storage 的接口变化，而字节本身已经说出了容器）。文件仍按扩展名读（§8-33）：扩展名是写下它的人对容器的声明，两条路各认各的输入，表仍是 `AudioType` 一张。
- **验收**：`ocr::recogniser` 的测试经回环替身（`endpoint::fakes::fake_provider`）对三种 face 各发一次，比请求的路径、请求里的模型名与这张图的 base64，以及读回的字；没有设施时按名拒绝；`Picture::new` 拒 locator 与字节不符的一对。`transcribe::recording` 的测试：五种容器各从自己的开头认出，认不出的列出五种。
-/

/-! D13 OCR 是一项设施，形状照转写

决定：`gateway::ocr` 与 `gateway::transcribe` 同形：端点账本里 `ModelTag::Ocr` 的那一次 `select` 给出 `Chosen`，`recogniser_for` 把它变成设施，`absent()` 答 `E_TOOL_UNAVAILABLE`；城的工具 `ocr` 只在那个选择成立时上 run 的工具表（sprawling-SPEC 8-142）。适配器就是 `adapter_for` 造的那一个，图片内容由所选 face 的 dialect 写（§8-34）。理由：「哪个端点答这一类活」已有账本这一个机制，「一张图在三种 face 上怎么写」已有 dialect 这一处；OCR 再写一套请求，就是这两件事的第二个权威。被否的备选：①专为 OCR 写一条请求（三种 face 各要一份图片拼法）；②让主模型自己看截图（主模型可能只读字，而 run 要的是一行可以读、可以搜的字，不是窗口里的一张图）；③把图先存进 CAS 再经城的 `ImageResolver` 兑付（读一个文件就成了一次写）。
-/
