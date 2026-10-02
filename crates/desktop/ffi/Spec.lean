-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-! # desktop_ffi 的规格

`sprawling-desktop-ffi`（库名 `desktop_ffi`，目录 `crates/desktop/ffi`）是桌面 server 唯一的 FFI 缝：没有准入安全接口的四组 Win32 调用——枚举顶层窗口、按窗口捕获、剪贴板文本、DPI 感知——由一片 Zig 叶子（`crates/desktop/ffi/zig/leaf.zig`）整段做完，Rust 一侧只借出缓冲、读回一个 step 与一个错误码。本文件是这个包的规格入口；能写成定理的性质在下面证明，其余要求、理由与决定写在十七节的注释里，决定写作 `D<n>`，别处引作 `desktop_ffi D<n>`。

桌面 server 那一侧怎么用这条缝（哪些调用经过它、`unsafe` 恒只在哪里）写在 `crates/desktop/Spec.lean` §8-12 与 D12；那里是 server 的规格，这里是缝本身的规格。
-/

namespace DesktopFfi

/-! ## 1 需求分解

三个可独立验收的单元：

- **边界规则**（`zig/boundary.zig`，Rust 面 `desktop_ffi::boundary`）：叶子往 Rust 的内存里写什么。四条：一串句柄按缓冲长度保留、按全长计数（`kept`）；一段别的程序写的剪贴板文本复制到第一个零单元为止、恒不越过块的大小（`textCopy`）；一段新文本恰好带一个终止符写进恰好那么长的块（`textFill`）；一张位图的字节数不溢出（`bitmapBytes`）。
- **资源配对**（`zig/leaf.zig`）：每个操作在它自己的 export 之内取得、也释放它取得的一切。剪贴板写的那块内存要么交给剪贴板、要么由叶子释放（`write`）；捕获取得的三个 GDI 对象各释放一次，位图解除选择之后才读回（`captured`）。
- **Rust 面**（`desktop_ffi` 的 `top_level`、`capture`、`clipboard`、`dpi`、`ended`）：每个 export 一个安全函数，函数里恰好一个 `unsafe` 块，块上一行 `SAFETY:` 写使它成立、并且可能为假的前提。来源：AGENTS.md「Rust」一节的平台调用次序；路线图 X3。
-/

/-! ## 2 验收标准

下面的定理是模型对性质的证明：保留的恰是前缀、计数是全长；复制恒不越过块、写下的恒放得进缓冲、恒不含零、缓冲够长时恒成功；填写恰好一个终止符且与复制互逆；块恒不留在进程手里；GDI 对象恒成对释放。每一组各有一条「拿掉守卫即反例」的定理。

生产实现与模型的一致性由三处检查，均不需要桌面：`desktop_ffi::boundary` 的测试 `the_leaf_and_the_rust_reference_agree_on_every_drawn_input` 用种子化的输入比对 Zig 叶子与 Rust 参考（`src/reference.rs`），每条规则两万个；同一比对的 `the_leaf_and_the_rust_reference_agree_for_as_long_as_asked` 按 `just fuzz-desktop` 给的轮数与种子跑（Rust 一侧的 fuzz）；`zig test zig/leaf.zig` 跑 Zig 侧的单测、种子化性质测试与 `std.testing.fuzz` 测试（Zig 一侧的 fuzz；覆盖引导的 `--fuzz` 今天不在 Windows 上实现，这里跑的是种子化那一半与 fuzz 测试的单次输入）。资源配对在真窗口上由桌面 server 的契约测试判（`crates/desktop/Spec.lean` §8-11）。
-/

/-! ## 3 假设与歧义

- **操作系统的行为是假设，不是定理。** `EnumWindows` 同步地在调用线程上回调、`GlobalSize` 报出块的真实大小、`GetDIBits` 写的字节不超过头里描述的大小、`SetClipboardData` 成功即接管块：这些写在 `leaf.zig` 旁，模型把它们当作参数（`Answers` 的各位）而不证明。Lean 证明的是叶子对这些回答的处理，不是 Win32 本身。
- **Zig 的运行期检查不证明外来地址有效。** 叶子以 `ReleaseSafe` 编译，越界即 trap；但一个 Rust 借出的指针是否真有那么长，由 Rust 面的 `SAFETY:` 前提承担，叶子无从检查。
- **未定**：无。
-/

/-! ## 4 现状分析

四组调用此前在 `platform::windows` 的 `enumerate`、`capture`、`clipboard`、`dpi` 四个模块里经 `windows` 绑定手写 `unsafe`（`crates/desktop/Spec.lean` §8-11 的表）；本包落地后，那四个模块只调这里的安全函数。叶子从 `crates/desktop/ffi/build.rs` 以 `zig build-lib` 编译成静态库，只在 Windows 目标上编；别的目标上本包只剩 `step`。
-/

/-! ## 5 权威信源

- Step 的名字与编号：`crates/desktop/ffi/src/step.rs`；`zig/step.zig` 是它的 Zig 拼写，`build.rs` 每次构建比对两者（D2）。
- Zig 的版本：`crates/desktop/ffi/zig-version`（D3）。
- 各 Win32 调用的前提：learn.microsoft.com 上 `EnumWindows`、`GetDC`、`ReleaseDC`、`PrintWindow`、`GetDIBits`、`OpenClipboard`、`GetClipboardData`、`GlobalSize`、`SetClipboardData`、`SetProcessDpiAwareness` 各自的函数页。
- 常量（`PW_RENDERFULLCONTENT`、`CF_UNICODETEXT`、`GMEM_MOVEABLE`、`HWND_MESSAGE`）只写在 `leaf.zig` 一处；DPI 感知值由 Rust 从 `windows` 绑定的定义传入。
-/

/-! ## 6 命名统一

沿用 `crates/desktop/Spec.lean` 的词：three-part refusal、scope、generation。**step**：叶子的一次调用停在哪里（`Finished` 或停下的那一步）；**leaf**：Zig 的那一片；**boundary rule**：叶子往借来的缓冲里写什么的规则。Lean 里的名字与 Zig／Rust 的对应：`kept` ↔ `Kept.keep`／`boundary::keep`，`textEnd` ↔ `textEnd`，`textCopy` ↔ `textCopy`／`boundary::text_copy`，`textFill` ↔ `textFill`／`boundary::text_fill`，`bitmapBytes` ↔ `bitmapBytes`／`boundary::bitmap_bytes`。
-/

/-! ## 7 模块边界

叶子不拥有任何措辞、scope、generation 或工具名；它只做调用、只写借来的缓冲、只答 step 与码。把 step 与码变成三段式拒词归桌面 server 的 `platform::windows::fault`。Rust 面每个调用组一个文件，`leaf.rs` 是 export 的唯一声明处，`ended.rs` 是把跨边界的数读成 `Step` 的唯一处。`fixture`（只在 `fixture` feature 下编译）是测试自己开一扇窗口、建一个编出来的句柄的那几处 `unsafe`：桌面 server 继承工作区的 `forbid`，连测试也写不出 `unsafe`，本包是唯一能放开它的一层，所以 server 的契约测试经这里开窗口（`crates/desktop/Spec.lean` D14）。
-/

/-! ## 8 接口先行

```rust
pub enum Step { Finished, Absent, NoRoom, Measuring, Listing, NoWindow, Context, Bitmap, Selecting,
    Drawing, Reading, ShortRows, Owner, Opening, Fetching, Locking, EmptyBlock, Allocating, Emptying, Handing }
pub enum Failure { At { step: Step, code: winsafe::co::ERROR }, Unspelled(u32) }
pub fn top_level::windows() -> Result<Vec<winsafe::HWND>, Failure>;
pub fn capture::pixels(window: &winsafe::HWND, width: i32, height: i32) -> Result<Vec<u8>, Failure>; // 自上而下 BGRA
pub fn clipboard::text() -> Result<Option<Vec<u16>>, Failure>;   // 不含终止符
pub fn clipboard::put_text(units: &[u16]) -> Result<(), Failure>;
pub fn dpi::declare(awareness: u32) -> Result<(), winsafe::co::HRESULT>;
pub fn dpi::awareness() -> Result<u32, winsafe::co::HRESULT>;
pub fn boundary::{keep, text_copy, text_fill, bitmap_bytes};   // 边界规则本身，供对拍与 fuzz
```

C ABI 的 export 一律 `sprawling_desktop_<名>`，声明在 `src/leaf.rs`。
-/

/-! ### 边界规则一：句柄的保留 -/

/-- 保留到目前为止的状态：写进缓冲的那些项（按次序）与看到的总数。 -/
structure Kept where
  held : List Nat
  count : Nat
deriving DecidableEq, Repr

/-- 一项进来：缓冲还有位置就写下，总数恒加一。与 `Kept.keep` 同一条规则。 -/
def Kept.keep (capacity : Nat) (k : Kept) (item : Nat) : Kept :=
  if k.count < capacity then { held := k.held ++ [item], count := k.count + 1 }
  else { held := k.held, count := k.count + 1 }

/-- 一整串流过保留规则，从空状态开始。 -/
def kept (capacity : Nat) (stream : List Nat) : Kept :=
  stream.foldl (Kept.keep capacity) ⟨[], 0⟩

/-! ### 边界规则二：剪贴板文本的复制 -/

/-- 一段文本在块里的部分：第一个零单元之前的那些单元，没有零就是整块。 -/
def textPrefix : List Nat → List Nat
  | [] => []
  | x :: rest => if x = 0 then [] else x :: textPrefix rest

/-- 一段文本在块里的终点：第一个零单元之前，或块的末尾。与 `textEnd` 同一条规则。 -/
def textEnd (block : List Nat) : Nat :=
  (textPrefix block).length

/-- 复制的结局：写下了 `written`，或缓冲不够、需要 `needed` 个单元。 -/
inductive Copied where
  | finished (written : List Nat)
  | noRoom (needed : Nat)
deriving DecidableEq, Repr

def textCopy (block : List Nat) (capacity : Nat) : Copied :=
  if textEnd block ≤ capacity then .finished (block.take (textEnd block))
  else .noRoom (textEnd block)

/-! ### 边界规则三：新文本的填写 -/

/-- 块长恰好是文本加一个终止符时写下它们，否则什么都不写。 -/
def textFill (units : List Nat) (blockLen : Nat) : Option (List Nat) :=
  if blockLen = units.length + 1 then some (units ++ [0]) else none

/-! ### 边界规则四：位图的字节数 -/

/-- 两边为正、乘积放得进 `most` 时的字节数；`most` 是 `usize` 的上界，作为参数留开。 -/
def bitmapBytes (most : Nat) (width height : Int) : Option Nat :=
  if 0 < width ∧ 0 < height ∧ width.toNat * height.toNat * 4 ≤ most then
    some (width.toNat * height.toNat * 4)
  else none

/-! ### 资源一：剪贴板写的那块内存 -/

/-- 一块内存此刻归谁。 -/
inductive Holder where
  | none
  | process
  | clipboard
  | freed
deriving DecidableEq, Repr

/-- 写的各步停在哪里，与 `Step` 里同名的那几个对应。 -/
inductive WriteStep where
  | finished | allocating | owner | opening | emptying | handing
deriving DecidableEq, Repr

/-- 操作系统对写的每一步的回答：分配、锁住填写、建 owner、打开、清空、交出。 -/
structure Answers where
  allocated : Bool
  filled : Bool
  owned : Bool
  opened : Bool
  emptied : Bool
  handed : Bool

/-- 写的结局：停在哪一步、块归谁、剪贴板有没有被清空。 -/
structure Written where
  step : WriteStep
  block : Holder
  cleared : Bool
deriving DecidableEq, Repr

/-- `clipboardWrite` 的控制流：先分配并填好，再建 owner、打开、清空、交出；交出之前任何一步失败，块由叶子释放。 -/
def write (a : Answers) : Written :=
  if !a.allocated then ⟨.allocating, .none, false⟩
  else if !a.filled then ⟨.allocating, .freed, false⟩
  else if !a.owned then ⟨.owner, .freed, false⟩
  else if !a.opened then ⟨.opening, .freed, false⟩
  else if !a.emptied then ⟨.emptying, .freed, false⟩
  else if !a.handed then ⟨.handing, .freed, true⟩
  else ⟨.finished, .clipboard, true⟩

/-! ### 资源二：捕获的 GDI 对象 -/

/-- 捕获取得的三样东西。 -/
inductive Gdi where
  | context | memory | bitmap
deriving DecidableEq, Repr

/-- 一次捕获在 GDI 上做的事，按发生的次序。 -/
inductive Event where
  | take (object : Gdi)
  | give (object : Gdi)
  | select
  | unselect
  | draw
  | read
deriving DecidableEq, Repr

/-- 操作系统对捕获每一步的回答。 -/
structure Drawn where
  context : Bool
  memory : Bool
  bitmap : Bool
  selected : Bool
  printed : Bool

/-- 选进、作画、选回：选进失败时什么都不选回；作画失败时仍选回。 -/
def drawing (d : Drawn) : List Event × Bool :=
  if !d.selected then ([], false)
  else if !d.printed then ([.select, .draw, .unselect], false)
  else ([.select, .draw, .unselect], true)

/-- `capture` 的控制流：每取得一样就以 `defer` 记下它的归还，归还按取得的逆序发生。 -/
def captured (d : Drawn) : List Event :=
  if !d.context then []
  else if !d.memory then [.take .context, .give .context]
  else if !d.bitmap then [.take .context, .take .memory, .give .memory, .give .context]
  else
    let (inner, ok) := drawing d
    [.take .context, .take .memory, .take .bitmap] ++ inner ++ (if ok then [.read] else []) ++
      [.give .bitmap, .give .memory, .give .context]

/-! ## 9 工作流程

每个操作是一次 export 调用：Rust 借出缓冲 → 叶子做完整段 Win32 调用、只写借来的缓冲 → 叶子答 step 与码 → Rust 把数读成 `Step`（`ended::step`），`NoRoom` 时按回报的长度加大缓冲重试（有界次数），其余 step 交给桌面 server 写成拒词。叶子不留任何跨调用的状态。
-/

/-! ## 10 实现逻辑

下面的证明按规则分组。每组先证正面性质，再给一个反例说明那条守卫不是摆设。
-/

/-- 保留规则的循环不变式：处理完前缀 `p` 后，缓冲里是 `p` 的前 `capacity` 项，总数是 `p` 的长度。 -/
theorem keep_step (capacity : Nat) (p : List Nat) (x : Nat) :
    Kept.keep capacity ⟨p.take capacity, p.length⟩ x = ⟨(p ++ [x]).take capacity, (p ++ [x]).length⟩ := by
  rw [List.take_append]
  by_cases h : p.length < capacity
  · have hle : p.length ≤ capacity := Nat.le_of_lt h
    obtain ⟨k, hk⟩ : ∃ k, capacity - p.length = k + 1 := ⟨capacity - p.length - 1, by omega⟩
    simp [Kept.keep, h, hk, List.take_of_length_le hle]
  · have hz : capacity - p.length = 0 := by omega
    simp [Kept.keep, h, hz]

theorem kept_from (capacity : Nat) :
    ∀ (s p : List Nat),
      s.foldl (Kept.keep capacity) ⟨p.take capacity, p.length⟩ =
        ⟨(p ++ s).take capacity, (p ++ s).length⟩
  | [], p => by simp
  | x :: s, p => by
    simp only [List.foldl_cons]
    rw [keep_step, kept_from capacity s (p ++ [x])]
    simp

/-- 保留的恰是流的前 `capacity` 项，计数恰是流的全长。 -/
theorem kept_is_prefix_and_full_count (capacity : Nat) (stream : List Nat) :
    kept capacity stream = ⟨stream.take capacity, stream.length⟩ := by
  have h := kept_from capacity stream []
  simpa [kept] using h

/-- 缓冲恒不被写过它的长度。 -/
theorem kept_never_past_capacity (capacity : Nat) (stream : List Nat) :
    (kept capacity stream).held.length ≤ capacity := by
  rw [kept_is_prefix_and_full_count, List.length_take]
  exact Nat.min_le_left _ _

/-- 缓冲不短于流时，流整个留下：`NoRoom` 之后按回报的计数重试必定成功。 -/
theorem kept_whole_when_room_suffices (capacity : Nat) (stream : List Nat)
    (room : stream.length ≤ capacity) : (kept capacity stream).held = stream := by
  rw [kept_is_prefix_and_full_count]
  exact List.take_of_length_le room

/-- 反例：若计数也只数写下的项，短缓冲的调用方就读不出该把缓冲加到多大。 -/
theorem a_count_of_written_items_hides_the_shortfall :
    (kept 1 [7, 8]).held.length ≠ (kept 1 [7, 8]).count := by decide

theorem textPrefix_length_le : ∀ (l : List Nat), (textPrefix l).length ≤ l.length
  | [] => by simp [textPrefix]
  | x :: l => by
    by_cases h : x = 0
    · simp [textPrefix, h]
    · have := textPrefix_length_le l
      simp only [textPrefix, if_neg h, List.length_cons]
      omega

/-- 文本的终点恒在块内：叶子恒不读过 `GlobalSize` 量出的界。 -/
theorem text_end_within_block (block : List Nat) : textEnd block ≤ block.length :=
  textPrefix_length_le block

theorem textPrefix_no_zero : ∀ (l : List Nat), 0 ∉ textPrefix l
  | [] => by simp [textPrefix]
  | x :: l => by
    by_cases h : x = 0
    · simp [textPrefix, h]
    · simp only [textPrefix, if_neg h, List.mem_cons, not_or]
      exact ⟨fun e => h e.symm, textPrefix_no_zero l⟩

theorem take_text_end : ∀ (block : List Nat), block.take (textEnd block) = textPrefix block
  | [] => by simp [textEnd, textPrefix]
  | x :: l => by
    by_cases h : x = 0
    · simp [textEnd, textPrefix, h]
    · have ih := take_text_end l
      simp only [textEnd] at ih ⊢
      rw [textPrefix, if_neg h, List.length_cons, List.take_succ_cons, ih]

/-- 写下的恰放得进缓冲。 -/
theorem copied_fits (block : List Nat) (capacity : Nat) (written : List Nat)
    (h : textCopy block capacity = .finished written) : written.length ≤ capacity := by
  unfold textCopy at h
  split at h
  · rename_i hle
    injection h with h
    subst h
    rw [List.length_take]
    omega
  · cases h

/-- 写下的恒不含零：一个读者在第一个零处停下，读到的就是全部。 -/
theorem copied_has_no_zero (block : List Nat) (capacity : Nat) (written : List Nat)
    (h : textCopy block capacity = .finished written) : 0 ∉ written := by
  unfold textCopy at h
  split at h
  · injection h with h
    subst h
    rw [take_text_end]
    exact textPrefix_no_zero block
  · cases h

/-- 写下的是块的前缀：叶子只复制，恒不改写别的程序写的字。 -/
theorem copied_is_the_blocks_prefix (block : List Nat) (capacity : Nat) (written : List Nat)
    (h : textCopy block capacity = .finished written) : written = block.take written.length := by
  unfold textCopy at h
  split at h
  · injection h with h
    subst h
    rw [List.length_take, Nat.min_eq_left (text_end_within_block block)]
  · cases h

/-- 缓冲不够时报出的长度大于缓冲，按它加大缓冲重试必定成功：重试有进展。 -/
theorem no_room_names_enough_room (block : List Nat) (capacity needed : Nat)
    (h : textCopy block capacity = .noRoom needed) :
    capacity < needed ∧ ∃ written, textCopy block needed = .finished written := by
  unfold textCopy at h
  split at h
  · cases h
  · rename_i hgt
    injection h with h
    subst h
    refine ⟨by omega, block.take (textEnd block), ?_⟩
    simp [textCopy]

/-- 反例：若复制以「块会以零结尾」为前提、一直读到零，一个没有终止符的块会让它读出块外——模型里表现为终点越过块长。 -/
theorem a_terminator_is_a_claim_not_a_bound :
    textEnd [1, 2, 3] = [1, 2, 3].length ∧ ¬ (0 ∈ [1, 2, 3]) := by decide

/-- 填写成功时块长就是要求的长度，最后一个单元是终止符。 -/
theorem filled_ends_in_one_terminator (units : List Nat) (blockLen : Nat) (block : List Nat)
    (h : textFill units blockLen = some block) :
    block.length = blockLen ∧ block.getLast? = some 0 := by
  unfold textFill at h
  split at h
  · rename_i hlen
    injection h with h
    subst h
    simp [hlen]
  · cases h

theorem textPrefix_append_zero : ∀ (units : List Nat), 0 ∉ units → textPrefix (units ++ [0]) = units
  | [], _ => by simp [textPrefix]
  | x :: l, h => by
    have hx : x ≠ 0 := fun e => h (by simp [e])
    have hl : 0 ∉ l := fun m => h (List.mem_cons_of_mem x m)
    rw [List.cons_append, textPrefix, if_neg hx, textPrefix_append_zero l hl]

/-- 填写与复制互逆：一段不含零的文本写进块、再读出来，恰是它自己。剪贴板的往返靠的就是这一条。 -/
theorem fill_then_copy_is_the_text (units : List Nat) (h : 0 ∉ units) (capacity : Nat)
    (room : units.length ≤ capacity) :
    ∃ block, textFill units (units.length + 1) = some block ∧
      textCopy block capacity = .finished units := by
  refine ⟨units ++ [0], by simp [textFill], ?_⟩
  have hend : textEnd (units ++ [0]) = units.length := by
    unfold textEnd
    rw [textPrefix_append_zero units h]
  unfold textCopy
  rw [hend, if_pos room, List.take_left' rfl]

/-- 反例：块长与文本不符时不写；若照写，块尾就没有终止符。 -/
theorem a_block_of_the_wrong_length_is_refused : textFill [1, 2] 2 = none := by decide

/-- 位图的字节数恰是行数乘每行的字节数，且恒不超过上界：Rust 借出的缓冲正是 `GetDIBits` 写的那么长。 -/
theorem bitmap_bytes_are_rows_of_four_byte_pixels (most : Nat) (width height : Int) (bytes : Nat)
    (h : bitmapBytes most width height = some bytes) :
    bytes = height.toNat * (width.toNat * 4) ∧ bytes ≤ most ∧ 0 < bytes := by
  unfold bitmapBytes at h
  split at h
  · rename_i hc
    injection h with h
    subst h
    obtain ⟨hw, hh, hm⟩ := hc
    have hw' : 0 < width.toNat := by omega
    have hh' : 0 < height.toNat := by omega
    refine ⟨by rw [Nat.mul_comm height.toNat, Nat.mul_assoc, Nat.mul_assoc, Nat.mul_comm height.toNat 4],
      hm, ?_⟩
    exact Nat.mul_pos (Nat.mul_pos hw' hh') (by decide)
  · cases h

/-- 反例：不判两边为正，零宽的窗口会借出一个空缓冲而 GDI 仍被要一行。 -/
theorem a_zero_side_is_refused (most : Nat) : bitmapBytes most 0 5 = none := by
  simp [bitmapBytes]

/-- 块恒不留在进程手里：要么从未分配，要么交给了剪贴板，要么被叶子释放。 -/
theorem no_block_is_left_with_the_process (a : Answers) : (write a).block ≠ .process := by
  unfold write
  cases a.allocated <;> cases a.filled <;> cases a.owned <;> cases a.opened <;>
    cases a.emptied <;> cases a.handed <;> simp

/-- 块归剪贴板，当且仅当写完成了：不会有两个主人，也不会成功而块被释放。 -/
theorem the_clipboard_holds_the_block_exactly_when_written (a : Answers) :
    (write a).block = .clipboard ↔ (write a).step = .finished := by
  unfold write
  cases a.allocated <;> cases a.filled <;> cases a.owned <;> cases a.opened <;>
    cases a.emptied <;> cases a.handed <;> simp

/-- 剪贴板被清空而写没完成，只有一种结局：`handing`，拒词据此说剪贴板现在是空的。 -/
theorem cleared_and_unwritten_is_handing (a : Answers)
    (h : (write a).cleared = true) (n : (write a).step ≠ .finished) : (write a).step = .handing := by
  revert h n
  unfold write
  cases a.allocated <;> cases a.filled <;> cases a.owned <;> cases a.opened <;>
    cases a.emptied <;> cases a.handed <;> simp

/-- 先备好再清空：剪贴板只在块已分配并填好之后才被清空（`crates/desktop/Spec.lean` D8）。 -/
theorem cleared_only_after_the_block_is_ready (a : Answers) (h : (write a).cleared = true) :
    a.allocated = true ∧ a.filled = true := by
  revert h
  unfold write
  cases ha : a.allocated <;> cases hf : a.filled <;> cases a.owned <;> cases a.opened <;>
    cases a.emptied <;> cases a.handed <;> simp

/-- 反例：先清空再分配（`winsafe` 0.0.29 的次序）时，分配失败也留下一个空剪贴板而拒词说的是内存不够。 -/
def writeEmptyingFirst (a : Answers) : Written :=
  if !a.emptied then ⟨.emptying, .none, false⟩
  else if !a.allocated then ⟨.allocating, .none, true⟩
  else if !a.handed then ⟨.handing, .process, true⟩
  else ⟨.finished, .clipboard, true⟩

theorem emptying_first_clears_without_a_block :
    (writeEmptyingFirst ⟨false, true, true, true, true, true⟩).cleared = true ∧
    (writeEmptyingFirst ⟨false, true, true, true, true, true⟩).step = .allocating := by decide

/-- 一样东西被取得的次数与被归还的次数。 -/
def taken (o : Gdi) (es : List Event) : Nat := es.count (.take o)
def given (o : Gdi) (es : List Event) : Nat := es.count (.give o)

/-- 每一样 GDI 对象，取得几次就归还几次，且至多一次：失败的路径也不漏，也不重复释放。 -/
theorem every_gdi_object_is_given_back_once (d : Drawn) (o : Gdi) :
    taken o (captured d) = given o (captured d) ∧ given o (captured d) ≤ 1 := by
  cases o <;> unfold captured drawing <;>
    cases d.context <;> cases d.memory <;> cases d.bitmap <;> cases d.selected <;>
    cases d.printed <;> decide

/-- `a` 出现在任何一个 `b` 之前；`b` 从不出现时也成立。 -/
def before (a b : Event) : List Event → Bool
  | [] => true
  | e :: es => if e = b then false else if e = a then true else before a b es

/-- 读回恒在解除选择之后：`GetDIBits` 恒不读一张仍选在某个 DC 里的位图。 -/
theorem read_back_follows_unselect (d : Drawn) : before .unselect .read (captured d) = true := by
  unfold captured drawing
  cases d.context <;> cases d.memory <;> cases d.bitmap <;> cases d.selected <;>
    cases d.printed <;> decide

/-- 反例：在选回之前读（守卫的作用域包住了读回）时，这条性质为假。 -/
theorem reading_inside_the_selection_breaks_it :
    before .unselect .read [.take .context, .select, .draw, .read, .unselect] = false := by decide

/-! ## 11 边界枚举

缓冲长度为零（保留、复制都只报计数，什么都不写）；流或文本为空；块没有终止符（终点是块尾）；块报出大小为零（`EmptyBlock`，不当作没有文本）；文本恰好填满缓冲；位图一边为零、为负、乘积溢出；剪贴板写的六步各自失败；捕获的五步各自失败。前四条与位图三条由 §10 的定理覆盖，写与捕获的每一种失败组合由 `cases` 穷举覆盖。
-/

/-! ## 12 错误处理

叶子恒以 step 答：`Finished`、`Absent`（剪贴板没有文本，是成功）、`NoRoom`（附长度，Rust 重试），其余是停下的那一步，附调用线程当时的 `GetLastError`。Rust 面把 step 读成 `Failure::At { step, code }`；一个不是任何 step 的数读成 `Failure::Unspelled`，而不是假定它有效。DPI 两个调用直接答 HRESULT。失败之后叶子不留任何资源（§10 的两组资源定理），剪贴板写在 `Handing` 之后是空的，这一件由拒词说出。

**D1 一个 export 做完整段操作，句柄恒不跨边界。** 被否：把每个 Win32 调用各包一个 export、在 Rust 里持 HDC 与 HGLOBAL 并靠 `Drop` 释放——那样释放的前提又回到 Rust 的 `unsafe` 里，缝也变成二十个而不是六个。重开参数：一个操作需要跨两次工具调用持有系统资源。
-/

/-! ## 13 依赖选型

`winsafe`（版本在根 `Cargo.toml` 的 `[workspace.dependencies]`）：边界上的 `HWND`、`co::ERROR`、`co::HRESULT` 都是它的 `#[repr(transparent)]` 类型，叶子直接写进这些类型的槽里，所以 Rust 面不需要 `from_ptr` 或 `from_raw` 就得到它们。Zig：只用标准库与自己声明的 `extern`；构建脚本只用 Rust 标准库起 `zig`，不加 crates.io 依赖。被否：`cc` 或 `zig` 的构建 crate（为一条命令多一个黑箱）；`build.zig`（`zig build` 先编译构建脚本本身，每次冷构建多几秒，而这里只有一条 `build-lib`）。
-/

/-! ## 14 硬编码声明

**D2 step 的定义在 Rust，Zig 是检查过的拼写。** `src/step.rs` 是名字与编号的唯一定义，`build.rs` 以 `include!` 读它，要求 `zig/step.zig` 含有逐字的渲染结果，不同即拒绝构建并印出应有的文本。被否：构建时把 `step.zig` 生成进 `OUT_DIR`——那样 `zig test zig/leaf.zig` 不经 cargo 就跑不起来。

**D3 Zig 的版本只写在 `crates/desktop/ffi/zig-version`。** `build.rs` 要求 `zig version` 与它相等；doctor 的 `zig` 一行以 `include_str!` 读它；CI 的安装步骤读它。被否：`build.zig.zon` 的 `minimum_zig_version`（这里没有 `build.zig`，而且它说的是下限不是钉子）。

缓冲的起始大小与重试次数（`top_level` 1024 个句柄、加 64、四次；`clipboard` 4096 个单元、四次）是我们的选择：一台桌面的顶层窗口数是几百，绝大多数剪贴板文本在四千单元以内；改它们只改一次调用的往返次数，不改结果。
-/

/-! ## 15 影响面

唯一的调用方是 `sprawling-desktop` 的 Windows 臂（`platform::windows` 的 `enumerate`、`capture`、`clipboard`、`dpi` 四个模块）。改一个 export 的签名要同时改 `leaf.zig`、`src/leaf.rs` 与它的安全函数；改 `Step` 要同时改 `step.zig`（`build.rs` 会拒绝漏改的那一侧）与桌面 server 的 `fault`。`sprawling` 经 `sprawling-desktop` 链接本包，所以 Windows 上构建 `sprawling` 需要 Zig。
-/

/-! ## 16 测试与约束

已证：§10 的定理。测试：`desktop_ffi` 的 `ended` 与 `boundary` 单测（对拍）；`zig test crates/desktop/ffi/zig/leaf.zig`（Zig 侧单测、种子化性质、fuzz 测试的单次输入）；`just fuzz-desktop`（按给定轮数与种子的长时对拍）。libFuzzer 在这里没有平台：叶子只在 Windows 上编，而 cargo-fuzz 在 windows-msvc 上链接不出 sancov 的节区符号，nightly 也不带 msvc 的 ASan 运行时；所以 Rust 一侧以抽样而不是以覆盖来 fuzz。重开参数：cargo-fuzz 在 windows-msvc 上链接得出。环境假设：§3。约束：`unsafe` 恒只在本包、恒是一次 export 调用、恒带一行 `SAFETY:`；lint 表是 `crates/desktop/Cargo.toml` 的 `[workspace.lints]`，与根工作区逐键比对（`xtask guard`）。
-/

/-! ## 17 文档关系

- `crates/desktop/Spec.lean` §8-11、§8-12、D12、D14：server 一侧的调用组表、缝的位置，以及本包那一张只差 `unsafe_code` 一行的 lint 表；那里的表变了，§1 的四组随之变。
- `tools/xtask/Spec.lean` §8-46：guard 怎么读这堵墙。
- `crates/sprawling/Spec.lean` §8-146：doctor 的 `zig` 一行。
- `AGENTS.md`「Rust」一节：平台调用的次序与 `SAFETY:` 行的写法；那里的规则变了，D1 重议。
-/

end DesktopFfi
