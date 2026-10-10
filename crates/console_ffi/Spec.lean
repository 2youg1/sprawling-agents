-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-! # console_ffi 的规格

`sprawling-console-ffi`（库名 `console_ffi`，目录 `crates/console_ffi`）是控制台的渲染器：一片 Zig 叶子（`crates/console_ffi/zig/leaf.zig`）把 Rust 写好的一份 scene 读成终端一帧的字节，Rust 一侧只借出 scene 与输出缓冲，读回帧长与光标所在的行。控制台本身（CLI 与安静宿主的生命周期、按键、会话、读记录）在 `crates/sprawling/spec/Console.lean` §8-11；那里决定屏上有什么，这里决定它长什么样。

渲染是一个纯函数——同一份 scene、同一个宽度得到同样的字节——没有状态机。这里证明的是它排版的一条性质：写进一行的字恒不越过这一行剩下的列（§2），终端的最后一列因此从不被写（D6）。其余由 `console_ffi::tests` 守住：每一帧写进一个终端模拟器（`vt100`），比对整屏文字、光标位置与下一帧擦除的行数；Zig 一侧的单测由 `zig test crates/console_ffi/zig/leaf.zig` 跑。
-/

/-! ## 1 需求分解

- **scene 的词表**（`console_ffi::part`，Zig 拼写 `zig/part.zig`）：帧的种类、转写行的种类、活动区的行、工具调用的结局、run 的结局、请求的答复、叶子的返回状态。
- **scene 的写法**（`console_ffi::scene`）：控制台说屏上有什么——一次工具调用和它用了多久、正在打的字和光标在第几个字符——不排版。
- **叶子**（`zig/`）：读 scene（`scene.zig`），量字宽、把控制字符换成 U+FFFD（`text.zig`），按列写带样式的字（`paint.zig`），排正文（`flow.zig`），格式化时刻、耗时与计数（`format.zig`），定列（`grid.zig`），画转写行（`transcript.zig`）、活动区（`live.zig`）与安静宿主（`quiet.zig`），把一帧包成一次同步更新（`frame.zig`）。
- **Rust 面**（`console_ffi::leaf`）：一个安全函数 `draw`，函数里恰好一个 `unsafe` 块。
-/

/-! ## 2 一行恒不越过它剩下的列

模型里的一段文字是每个字符的列宽（组合符 0，宽字符 2，其余 1）；`cut` 是 `paint.zig` 的 `Painter.cut`，`fit` 是 `Painter.fit`：放得下就整段写，放不下就写到留出一列为止，再写一个占一列的省略号。性质对每一段文字、每一个剩余列数成立：写下的列数不超过剩下的列（`a_fitted_text_never_runs_past_its_room`），放得下的文字一字不少（`a_text_that_fits_is_written_whole`）。派生的检查是 `console_ffi::tests::a_narrow_window_drops_the_times_and_keeps_every_line_inside_it`：四十列的窗口里每一行不超过三十九列，宽字符在内。
-/

namespace ConsoleFfi

/-- 一段文字：每个字符占几列。 -/
abbrev Text := List Nat

def width : Text → Nat
  | [] => 0
  | c :: cs => c + width cs

/-- 从头取字符，直到下一个放不下为止（`Painter.cut`）。 -/
def cut : Text → Nat → Text
  | [], _ => []
  | c :: cs, room => if c ≤ room then c :: cut cs (room - c) else []

/-- 放得下就整段写；放不下就留一列给省略号（`Painter.fit`）。 -/
def fit (t : Text) (room : Nat) : Text :=
  if width t ≤ room then cut t room
  else if room = 0 then []
  else cut t (room - 1) ++ [1]

theorem width_append (a b : Text) : width (a ++ b) = width a + width b := by
  induction a with
  | nil => simp [width]
  | cons c cs ih => simp [width, ih, Nat.add_assoc]

theorem cut_fits (t : Text) (room : Nat) : width (cut t room) ≤ room := by
  induction t generalizing room with
  | nil => simp [cut, width]
  | cons c cs ih =>
    unfold cut
    split
    · have := ih (room - c)
      simp only [width]
      omega
    · simp [width]

theorem cut_keeps_what_fits (t : Text) (room : Nat) (h : width t ≤ room) : cut t room = t := by
  induction t generalizing room with
  | nil => rfl
  | cons c cs ih =>
    simp only [width] at h
    have fits : c ≤ room := by omega
    unfold cut
    simp [fits, ih (room - c) (by omega)]

theorem a_fitted_text_never_runs_past_its_room (t : Text) (room : Nat) :
    width (fit t room) ≤ room := by
  unfold fit
  split
  · exact cut_fits t room
  · split
    · simp [width]
    · rw [width_append]
      have := cut_fits t (room - 1)
      simp only [width]
      omega

theorem a_text_that_fits_is_written_whole (t : Text) (room : Nat) (h : width t ≤ room) :
    fit t room = t := by
  unfold fit
  simp [h, cut_keeps_what_fits t room h]

end ConsoleFfi

/-! ## 8 接口

```rust
// console_ffi
pub fn draw(scene: &Scene, into: &mut Vec<u8>) -> Result<Frame, Refused>;
pub struct Frame { pub cursor_row: u16 }               // 光标在活动区第一行之下几行
pub enum Refused { Malformed, TooLong }

// console_ffi::scene
pub struct TimeOfDay(u32);                             // 本地午夜以来的秒
pub enum Entry { Banner{..}, You{..}, Head{..}, Reasoning{..}, Tool{..}, Reply{..}, Note{..}, Ended{..}, Resolved{..} }
pub struct Live<'a> { pub waiting, pub working, pub calling, pub asking, pub composer, pub menu }
pub struct Inline<'a> { pub columns: u16, pub erase: u16, pub previous: Option<Part>, pub entries: &'a [Entry], pub live: Option<Live<'a>> }
pub struct Quiet<'a> { pub columns: u16, pub rows: u16, pub url, pub code, pub key, pub transient }
impl Scene { pub fn inline(&Inline) -> Scene; pub fn quiet(&Quiet) -> Scene; }
```

- **主屏一帧**：从上一帧光标所在的行向上 `erase` 行回到活动区第一行，清到屏底，写下新的转写行，再画活动区，把光标放回 composer。整帧包在 `ESC [ ? 2026 h` 与 `ESC [ ? 2026 l` 之间；不认 2026 的终端忽略它，画出同样的字节。
- **转写行只写一次**，进终端自己的回滚，搜索、选中、复制都归终端。人说的一行前写 OSC 133 `A`，记得这个标记的终端能在人说过的话之间跳。
- **安静宿主**：备用屏中央两行——地址、配对码（为超出这台电脑的 serve 现铸的 key 跟在配对码后面）——至多再一行临时行，在下方隔一行。
-/

/-! D1 渲染器是一片 Zig 叶子，在它自己的包里，`unsafe` 只在那一次调用

决定：控制台的排版与转义序列由 Zig 写成，静态链接进二进制；叶子只读借来的 scene、只写借来的缓冲，不读时钟、不开文件、不留状态；包的 lint 表与工作区相同，只有 `unsafe_code` 是 `deny`，`console_ffi::leaf` 里那一次调用在最窄处以 `#[expect(unsafe_code, reason = …)]` 放开，旁边一行 `SAFETY:` 写使它成立、可能为假的前提。`build.rs` 用 `zig-version` 钉住的 Zig 按目标三元组 `zig build-lib`，非 Windows 目标加 `-fPIC`；仓库里构建时这份钉与 `crates/desktop/ffi/zig-version` 必须相同。`ReleaseSafe` 让叶子的每个越界与溢出检查留在发布的二进制里，失败即陷入；`panic = std.debug.no_panic`，叶子不带栈回溯机制，在每个目标上构建得一样。

理由：控制台用 Zig 实现（人的定规）。叶子是纯函数，所以边界只有两段字节与一个结果结构，scene 由 Rust 的唯一写者产生，读不懂的 scene 是写者的缺陷而不是输入。

被否：①把渲染放进 `crates/desktop/ffi`——那个包是桌面 server 的缝，只在 Windows 上构建，控制台在三个平台上都要。②全屏备用屏的 TUI 库——控制台的 CLI 在主屏上，回滚归终端（`crates/sprawling/spec/Console.lean` §8-11）。

代价：Zig 0.17 成为在每个平台上构建工作区的前提，CI 的每个构建作业都装它（`.github/actions/zig`）。

重开参数：Zig 换版本时两份钉一起改；一个平台的 Rust 目标 Zig 不能交叉编译时。
-/

/-! D2 scene 的词表只有一处定义

决定：`src/part.rs` 是词表的定义；库把它声明为模块，`build.rs` 以 `include!` 读进同一份文件，按它渲染出 `zig/part.zig` 应有的枚举，文件里没有这段文字时拒绝构建，并打印应写下的文字。scene 是小端序，每段文字是一个 `u32` 长度加字节，缺席的时刻写 `u32::MAX`，缺席的数写 `u64::MAX`。

理由：Rust 写、Zig 读，两边各写一份就是同一事实的两个家；构建时比对使分歧当场变红。
-/

/-! D3 颜色只取终端自己的调色板

决定：叶子只用默认前景、淡色（SGR 2）、粗体、蓝（ACCENT：在动或被选中——正在工作的 run、菜单里光标所在的那一项）与黄（ALERT：需要人或出了缺——待批的请求、失败的调用、撞到上限的 run），不写 256 色或真彩色。

理由：控制台跟着人在终端里选的主题走，暗色亮色都成立；两种色的含义与 WebUI 的 `docs/frontend-method.md` §7B 相同，终端这一面与页面像同一个产品。
-/

/-! D4 没有自己会动的东西

决定：活动区只在有事发生时重画——一条记录、一个键、一次缩放；正在工作的那一行写「since 时刻」，不跳秒、不转圈。

理由：一个在输出时滚到底部的终端里，每秒一次的重画会让人读不了回滚；旧 conhost 在选中文字时会暂停输出。城在工作这件事另有 OSC 7501 告诉终端（`crates/sprawling/spec/Console/ProgramStatus.lean` D77）。
-/

/-! D5 记录里的控制字符到不了终端

决定：每个 C0 与 C1 控制字符、DEL，以及每个不是 UTF-8 的字节，都画成 U+FFFD；换行只在 scene 的多行文字里作换行。

理由：转写里是模型与人的字，其中一个 ESC 就是给终端的一条命令——清屏、改标题、写剪贴板。
-/

/-! D6 列

决定：宽度不小于 60 列时，有时刻的行在第 0 列写 `HH:MM:SS`，第 10 列写标记（`›` 人说的、`●` run 的头、`◇` 工具、`■` 结局、`?` 待批、`✓`/`✗` 已答），第 12 列起是字，折行回到第 12 列；窄于 60 列时不画时刻，标记移到第 0 列。回复的正文至多 88 列宽，约等于 WebUI 的阅读栏。工具行的耗时右对齐在 `min(宽, 12 + 88) − 8` 列，后面留 `failed` 的位置；正在进行的调用在同一列写 `running`。终端的最后一列从不写，因为有的终端写满它就折行。
-/

/-! D7 composer 照 WebUI 的样子

决定：字在上，下一行一条线跟着字走——实线画到字的最右端（至少两列），后面接 `╌╌╌┄┄┄` 淡出；空的时候线是淡色，有字时实线是默认前景。线下一行左边是房间，右边是平线问的模型与思考档位；斜杠菜单打开时占这一行的位置，至多五项，光标所在那项前画蓝色的 `▎`，最后一行说还有几项与哪些键。来源：`docs/frontend-method.md` §7I。
-/
