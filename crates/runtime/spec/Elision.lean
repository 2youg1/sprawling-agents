-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::elision

规定 `elision`（`crates/runtime/src/` 下同名的文件）。「这里被裁掉了」的唯一标记，以及切口可以落在哪里。本文件是 `crates/runtime/Spec.lean` 的一个分部；下面每一节保留它在 runtime 规格里的标签 §8-n，别处引作 `crates/runtime/Spec.lean §8-n`。
-/

/-!
### 8-42 runtime::elision（形状 2 值类型＋形状 6 数据面；**「这里被裁掉了」的唯一权威**）


```rust
pub enum Elided { Nothing, Head, Middle, Tail }          // 被拿走的那一段在哪一头
pub struct Cut { pub text: String, pub dropped: ByteLen, pub place: Elided }
impl Cut { pub fn whole(text: &str) -> Cut; }            // 什么都没拿走
pub fn marker(dropped: ByteLen) -> String;               // `[truncated: N bytes]`
pub fn gap_marker(dropped: ByteLen) -> String;           // 同一句，独占一行
pub fn marker_room(text_len: usize) -> usize;            // 计数未知时按最宽预留
pub fn gap_marker_room(text_len: usize) -> usize;
pub(crate) struct Boundary<'a>;                          // 一个切口：切点两侧的字节，各是完整字符
impl<'a> Boundary<'a> {
    pub(crate) fn before(text: &'a str, at: usize) -> Boundary<'a>;
    pub(crate) fn after(text: &'a str, at: usize) -> Boundary<'a>;
    pub(crate) fn head(self) -> &'a str;
    pub(crate) fn tail(self) -> &'a str;
    pub(crate) fn offset(self) -> usize;
}
pub fn boundary_before(text: &str, at: usize) -> usize;  // 不大于 at 的最大字符边界（Boundary 的偏移投影）
pub fn boundary_after(text: &str, at: usize) -> usize;
pub fn splice(text: &str, front: usize, back: usize, place: Elided) -> Cut;
```

**四条口径：**

1. **一句话，一个产地。** prefix、pipeline、compaction、sieve 四处都会拿走字节，给读者的那句话只有 `marker` 与它的分行形式 `gap_marker`。`replay::rebuild_prefix` 逐字重写同一句才重算得出段哈希：拼写一旦有第二个家，离线重放就在哈希对拍处失败，而失败发生在与改动无关的另一个模块里。
2. **`dropped` 数的是源字节，标记自己不算。** 去掉标记后的长度加上 `dropped` 恒等于输入长度。计数由 `splice` 从两个边界值算出，与插标记是同一个动作；调用方拿两个长度相减求丢弃量，减出来的数会把插进去的标记当成幸存文本，因此这条路在接口上不再存在。
3. **切口只由一个值给，而它给的是两侧的字节。** `Boundary::before` 与 `Boundary::after` 是全 crate 仅有的两处字符边界判定；判定的答复就是这个值本身——它向文本要那一次切分，拿回切点两侧的字节，所以**不存在一个能让调用方自己算成非边界的下标**。`boundary_before`／`boundary_after` 是这个值的偏移投影，供只需要位置的调用点用；`splice` 与 `keep_sections` 都向它要两侧的字节，因而 `dropped` 与幸存的字节出自同一个值，合起来恒是输入。同一个循环写成几份时行为一开始都相同，分叉是无声的：任一处改成向上取整或加最小保留量，另几处不会跟。裸下标与它的文本是两条可以互相矛盾的事实，`str::get` 在那个矛盾上答 `None`，而调用方读成「这里什么都没留下」——丢掉的正是截断本该保留的字节。
4. **预留按最宽算。** 标记的宽度随计数的位数变，而计数要等切完才知道；`marker_room(text.len())` 给出该文本能产生的最宽标记，因为丢弃量不可能超过文本自身长度。于是「结果不大于预算」由预留保证，而不是由一次事后检查补救。

**关门测试**（`elision::tests` 与 `compaction::tests`）：`splice` 的 `dropped` 加上去掉标记后的长度恒等于输入长度——**对任意一对下标成立**，包括落在字符内部的与顺序颠倒的一对（proptest 量化，不是举例）；任何切口落在字符边界；`compact` 对一段日志报出的丢弃量与幸存字节数加起来是原文长度。
-/
