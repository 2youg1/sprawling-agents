-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::answer::model_figures

规定 `answer::model_figures`（`crates/wire/src/` 下同名的文件）与 `Query::ModelFigures`。人把两个数字格留空时，城会为一个端点的每个模型登记什么窗口与上限，各由哪一级说出。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的帧形状由 Rust 的类型与 `crates/wire/tests/wire_contract.rs` 钉住的 wire schema（`wire::schema_hash`）守住，梯子的性质由 `crates/gateway/spec/Provider/Ceiling.lean` 证明。
-/

/-!
### 8-97 模型表的占位：`Query::ModelFigures`

```rust
Query::ModelFigures {
    endpoint: ProviderName,          // 端点的名字：读「此前登记」那一级
    base_url: String,                // 框里的文字，城自己归一化
    face: DialectKind,               // 表单上选的兼容格式
    models: Vec<FiguresAsked>,       // 表上的每一行
}
pub struct FiguresAsked {
    pub id: String,
    pub context_tokens: Option<Window>,      // provider 的列表为这一行写的窗口；人点名的行缺席
    pub max_output_tokens: Option<Ceiling>,  // 同上，上限
    pub tag: Option<ModelTag>,               // 这一行要担的角色
}
Answer::ModelFigures(ModelFiguresAnswer)
pub struct ModelFiguresAnswer { pub models: Vec<FiguresLine> }      // 与问的次序相同
pub struct FiguresLine {
    pub id: String,
    pub window: Option<WindowLine>,          // 缺席：没有一级说出窗口，窗口未知，上下文提醒不响
    pub ceiling: Option<CeilingLine>,        // 缺席只在策略缺省本身为零时，那一态由策略模块自己的测试禁止
}
pub struct WindowLine { pub tokens: Window, pub from: FigureFrom }
pub enum CeilingLine {                       // 线上 snake_case
    Sent { tokens: Ceiling, from: FigureFrom },
    ProviderDefault,                         // chat 与 responses 两种格式上没人说：请求不带上限，供应方按模型取它的缺省
}
pub enum FigureFrom { Person, Upstream, Registered, Preset, Policy }   // 线上 snake_case，与 `gateway::FigureSource::as_str` 同拼
```

- **答的是选型点会登记的那两个数**：每一行由 `gateway::Figures::resolve` 求出（gateway D37），与 `accounting::worker::credentials::endpoints::choosing` 在 `SelectModel` 不带数字时调的是同一个函数、同样的四级陈述：人填的两格（这一问恒为空，因为占位只在格子空着时显示），provider 的列表行（`FiguresAsked` 的两个数），此前登记（端点账本里 `tag` 指着同一端点、同一模型的那一次选择，`EndpointBook::registered`），目录行与预置表。客户端不抄梯子，只画答案。
- **base URL 与兼容格式由城归一化**：`gateway::normalise_entered` 加 `Normalised::face`，与 `Entered::resolved` 在 probe 与 attach 上走的是同一条路，所以表上问的主机与登记时调用的主机是同一个。归一化拒绝的文字整问答 `Unavailable`，带它的原因（D47）。
- **目录一问建一次**：`MarketSnapshot::builtin()` 在一问开头建一次，每一行用 `lookup` 取自己的目录行；没有一级碰网络或磁盘。
- **列表行由页面带来**：表单在登记之前就要画占位，而这时 provider 的列表只在 `endpoint_probed` 的记录里，视图不折这条记录。页面把它读到的两个数送回来，问「这样的一行会登记成什么」。
- **不写进 `endpoint_probed`**：目录与预置表随二进制钉住，写进账本的一份副本会在下一版改表之后过时；这一问每次都按此刻的二进制作答。
- 名字表多一项，schema 哈希因此而变，`WIRE_V` 不为此进位（D1）。
-/

/-! D58 模型表的占位由城的梯子按行作答，来源是一个闭集的枚举

**决定**：(a) 一条 `Query::ModelFigures`，按端点名、框里的 base URL、兼容格式与表上的行作答，每行答窗口及其来源、上限（`Sent` 带来源，或 `ProviderDefault`）；(b) 来源在线上是闭集枚举 `FigureFrom`，拼写与账本里 `model_selected.window_from`／`ceiling_from` 的词相同，由 `accounting::views::providers` 的穷尽映射从 `gateway::FigureSource` 写出；(c) provider 列表的两个数由页面带来。

**理由**：(a) 占位要说的是「空着时城会登记什么」，那是选型点的梯子的答案，只有调同一个函数才不会与登记分岔；按行带 `tag`，此前登记那一级才答得出来。(b) 页面按来源选一句话，闭集让它的 switch 是穷尽的，城多一级时页面编译不过，而不是落到一个默认分支；思考档的 `OfferSource` 已是这个写法。(c) 列表行只在 `endpoint_probed` 里，让视图为一个占位折这条记录，要多持一份每个端点最后一次探测的状态。

**被否**：①把数字与来源加进 `endpoint_probed`——见 §8-97，账本会留下一份随版本过时的副本；②来源在线上是账本的词（字符串）——页面的 switch 要一个默认分支，一个新来源会被画成别的句子；③客户端照抄梯子（人、provider、目录）——梯子有两个权威，下一次改梯子时页面与登记会说不同的话；④只按 `(base URL, face, id)` 问、不带列表行与角色——provider 列表与此前登记两级答不出来，占位会与登记不同。

**重开参数**：视图开始折 `endpoint_probed`（那时列表行可以由城自己取）；或价格进梯子（gateway §3 的开放问题），那时这一问多答价格。
-/
