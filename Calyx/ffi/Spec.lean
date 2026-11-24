-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-! # console_ffi 的规格

`sprawling-console-ffi`（库名 `console_ffi`，目录 `Calyx/ffi`）是控制台的渲染器：一片 Zig 叶子（`Calyx/ffi/zig/leaf.zig`）把 Rust 写好的一份 scene 读成终端一帧的字节，Rust 一侧只借出 scene 与输出缓冲，读回帧长与光标所在的行。控制台本身（CLI 与安静宿主的生命周期、按键、会话、读记录）在 `Sprawling/app/spec/Console.lean` §8-11；那里决定屏上有什么，这里决定它长什么样。

渲染是一个纯函数——同一份 scene、同一个宽度得到同样的字节——叶子本身没有状态机；Rust 缓冲重试由下列状态机刻画。这里也证明它排版的一条性质：写进一行的字恒不越过这一行剩下的列（§2），终端的最后一列因此从不被写（D6）。其余由 `console_ffi::tests` 守住：每一帧写进一个终端模拟器（`vt100`），比对整屏文字、光标位置与下一帧擦除的行数；Zig 一侧的单测由 `zig test Calyx/ffi/zig/leaf.zig` 跑。
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

/-- Rust 自动扩容阈值；不是调用方容量的上限。 -/
def retryLimit : Nat := 64 * 1024 * 1024

def initialCapacity (reserved : Nat) : Nat := max reserved (16 * 1024)

inductive DrawStatus where
  | drawn | full | malformed
  deriving DecidableEq, Repr

inductive DrawState where
  | trying (capacity : Nat)
  | drawn | malformed | tooLong
  deriving DecidableEq, Repr

/-- 一次叶子调用；终态不再调用叶子。Full 才能扩容。 -/
def drawStep (state : DrawState) (status : DrawStatus) : DrawState :=
  match state, status with
  | .trying _, .drawn => .drawn
  | .trying _, .malformed => .malformed
  | .trying capacity, .full =>
    if capacity < retryLimit then .trying (min (capacity * 2) retryLimit)
    else .tooLong
  | state, _ => state

/-- 每次重试严格增长且不超过阈值，因此不能无限重试。 -/
theorem retry_progress (capacity : Nat) (positive : 0 < capacity)
    (below : capacity < retryLimit) :
    capacity < min (capacity * 2) retryLimit ∧
    min (capacity * 2) retryLimit ≤ retryLimit := by
  omega

/-- 沿连续 Full 调用逐次扩容，未到阈值不能拒绝。 -/
theorem full_retries_below_limit (capacity : Nat) (below : capacity < retryLimit) :
    drawStep (.trying capacity) .full = .trying (min (capacity * 2) retryLimit) := by
  simp [drawStep, below]

/-- 至少达到阈值且仍为 Full 才拒绝；该容量仍可以成功画出。 -/
theorem refusal_boundary (capacity : Nat) :
    (drawStep (.trying capacity) .full = .tooLong ↔ retryLimit ≤ capacity) ∧
    drawStep (.trying capacity) .drawn = .drawn ∧
    drawStep (.trying capacity) .malformed = .malformed := by
  by_cases h : capacity < retryLimit <;> simp [drawStep, h] <;> omega

/-- 调用方可以预留超过阈值的容量，第一轮照常使用。 -/
theorem larger_reservation_is_used (reserved : Nat) (large : retryLimit ≤ reserved) :
    initialCapacity reserved = reserved := by
  unfold initialCapacity retryLimit at *
  omega

/-- 供 Rust 边界检查对照的容量轨迹；逐次翻倍，最后封顶。 -/
def capacityTrace (capacity : Nat) : Nat → List DrawState
  | 0 => [.trying capacity]
  | n + 1 => .trying capacity ::
    (match drawStep (.trying capacity) .full with
    | .trying next => capacityTrace next n
    | terminal => [terminal])

#eval capacityTrace (initialCapacity 0) 13
#eval capacityTrace (retryLimit - 1) 2
#eval capacityTrace (retryLimit + 1) 1

end ConsoleFfi

/-! ## 8 接口

`draw` 只在叶子返回 `Full` 且缓冲小于 64 MiB 时重试，每次至多翻倍并止于
64 MiB；调用方预留的更大容量照常使用，因此这不是硬上限。`Malformed` 直接返回，
不重试。容量至少 64 MiB 时仍返回 `Full` 才答 `TooLong`；用户文字及其转写可以
正常触发此结果，提示应减少单帧文字。`Malformed` 提示报告缺陷，两者均不停止城市服务。`Screen::frame` 显示相应恢复提示，保留未显示的转写供下一帧重试；成功后才推进转写位置。连续相同拒绝只提示一次，避免每个事件重复刷屏。

一致性检查：`lake build Calyx.ffi.Spec` 检查状态机性质并输出容量轨迹；将轨迹与 `src/leaf.rs` 的初始容量、Full 分支逐次对照。Rust 的 `tests/edges.rs` 通过真实叶子检查扩容、调用方大容量及 Malformed 清空输出；Zig 链接测试检查 URI 不带终端命令。


```rust
// console_ffi
pub fn draw(scene: &Scene, into: &mut Vec<u8>) -> Result<Frame, Refused>;
pub struct Frame { pub cursor_row: u16 }               // 光标在活动区第一行之下几行
pub enum Refused { Malformed, TooLong }

// console_ffi::scene
pub struct TimeOfDay(u32);                             // 本地午夜以来的秒
pub enum Entry { Banner{..}, You{..}, Head{..}, Reasoning{..}, Tool{..}, Reply{..}, Note{..}, Ended{..}, Resolved{..} }
pub struct Live<'a> { pub waiting, pub working, pub calling, pub asking, pub replying, pub composer, pub menu }
pub struct Inline<'a> { pub columns: u16, pub rows: u16, pub erase: u16, pub previous: Option<Part>, pub entries: &'a [Entry], pub live: Option<Live<'a>> }
pub struct Quiet<'a> { pub columns: u16, pub rows: u16, pub url, pub code, pub key, pub transient }
impl Scene { pub fn inline(&Inline) -> Scene; pub fn quiet(&Quiet) -> Scene; }

// console_ffi::zone
pub fn seconds_since_midnight(millis: u64) -> Option<u32>; // 系统本地时区里那一刻的午夜以来秒
```

- **主屏一帧**：从上一帧光标所在的行向上 `erase` 行回到活动区第一行，清到屏底，写下新的转写行，再画活动区，把光标放回 composer。`rows` 是可见窗口的高度，活动区的位置由 D8 决定。整帧包在 `ESC [ ? 2026 h` 与 `ESC [ ? 2026 l` 之间；不认 2026 的终端忽略它，画出同样的字节。
- **转写行只写一次**，进终端自己的回滚，搜索、选中、复制都归终端。人说的一行前写 OSC 133 `A`，记得这个标记的终端能在人说过的话之间跳。
- **安静宿主**：备用屏中央两行——地址、配对码（为超出这台电脑的 serve 现铸的 key 跟在配对码后面）——至多再一行临时行，在下方隔一行。
-/

/-! D1 渲染器是一片 Zig 叶子，在它自己的包里；渲染路径的 `unsafe` 只在 leaf 那一次调用

决定：控制台的排版与转义序列由 Zig 写成，静态链接进二进制；叶子只读借来的 scene、只写借来的缓冲，不读时钟、不开文件、不留状态；包的 lint 表与工作区相同，只有 `unsafe_code` 是 `deny`，`console_ffi::leaf` 里那一次调用在最窄处以 `#[expect(unsafe_code, reason = …)]` 放开，旁边一行 `SAFETY:` 写使它成立、可能为假的前提。`build.rs` 用 `zig-version` 钉住的 Zig 按目标三元组 `zig build-lib`，非 Windows 目标加 `-fPIC`；仓库里构建时这份钉与 `Sprawling/desktop/ffi/zig-version` 必须相同。`ReleaseSafe` 让叶子的每个越界与溢出检查留在发布的二进制里，失败即陷入；`panic = std.debug.no_panic`，叶子不带栈回溯机制，在每个目标上构建得一样。

理由：控制台用 Zig 实现（人的定规）。叶子是纯函数，所以边界只有两段字节与一个结果结构，scene 由 Rust 的唯一写者产生，读不懂的 scene 是写者的缺陷而不是输入。

被否：①把渲染放进 `Sprawling/desktop/ffi`——那个包是桌面 server 的缝，只在 Windows 上构建，控制台在三个平台上都要。②全屏备用屏的 TUI 库——控制台的 CLI 在主屏上，回滚归终端（`Sprawling/app/spec/Console.lean` §8-11）。

代价：Zig 0.17 成为在每个平台上构建工作区的前提，CI 的每个构建作业都装它（`.github/actions/zig`）。

`build.rs` 只在 `zig/`、`src/part.rs` 或两份钉变了时重跑，不随 `PATH` 重跑：叶子由钉住的那个版本编出，Zig 换了目录不改变它，而 CI 每个作业把 Zig 解到一个随机目录，桌面上编辑器与终端的 `PATH` 也不同，随 `PATH` 重跑就是每次重编叶子与依赖它的每个包。被否：跟踪 `PATH`——它只在 Zig 换目录时触发，同一目录里换了版本照样察觉不到；版本由钉与重跑时的版本检查守。

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

决定：活动区只在已有脏状态时重画——一条记录、一条流式增量、一个键、一次粘贴、一次缩放、
一次结束；壳可用显示节拍（`Calyx/Spec.lean` D11，≤360 Hz）推迟呈现，但不发明新状态。
正在工作的那一行写「since 时刻」，不跳秒、不转圈。进行中的助手正文是活动区 `Replying`，
不是定时动画。

理由：一个在输出时滚到底部的终端里，无事件的周期重画会让人读不了回滚；旧 conhost 在选中
文字时会暂停输出。城在工作这件事另有 OSC 7501 告诉终端（`Sprawling/app/spec/Console/ProgramStatus.lean` D77）。
高吞吐流式输出的合并呈现由壳拥有，叶子仍不读时钟（D1）。

被否：①工作行每秒刷新——妨碍回滚；②无脏状态的 vsync 空转——发明运动；③增量写入转写——
与「转写只写一次」冲突。
-/

/-! D14 活动区可含进行中的助手正文 `Replying`

决定：词表增加 `Replying = 36`；`Live.replying` 是可选正文，画在 Asking/Calling 之后、
Composer 之前，排版与 `Reply` 相同（plain、折行、无时刻标记）。缺席时不写该 tag。
Rust `scene` 与 Zig `write`/`live` 同形；`part.rs` 仍是唯一词表权威（D2）。

理由：流式增量必须可擦写，只能进每帧重画的活动区；完整后再进转写 `Reply`（`Calyx/Spec.lean` D11）。

被否：①复用 `Note`/`Reply` 进转写——回滚留下半句；②把草稿塞进 `Working` 行——语义是
run 状态不是正文。
-/

/-! D5 记录里的控制字符到不了终端

决定：进入字符渲染器的 C0、C1、DEL 和无效 UTF-8 画成 U+FFFD；排版可先解释换行与空白。OSC 8 URI 在输出前删除这些控制字符和无效 UTF-8，正常 UTF-8 保留。

理由：转写里是模型与人的字，其中一个 ESC 就是给终端的一条命令——清屏、改标题、写剪贴板。
-/

/-! D6 列

决定：宽度不小于 60 列时，有时刻的行在第 0 列写 `HH:MM:SS`，第 10 列写标记（`›` 人说的、`●` run 的头、`◇` 工具、`■` 结局、`?` 待批、`✓`/`✗` 已答），第 12 列起是字，折行回到第 12 列；窄于 60 列时不画时刻，标记移到第 0 列。回复的正文至多 88 列宽，约等于 WebUI 的阅读栏。工具行的耗时右对齐在 `min(宽, 12 + 88) − 8` 列，后面留 `failed` 的位置；正在进行的调用在同一列写 `running`。终端的最后一列从不写，因为有的终端写满它就折行。
-/

/-! D7 composer 照 WebUI 的样子

决定：字在上，下一行一条线跟着字走——实线画到字的最右端（至少两列），后面接 `╌╌╌┄┄┄` 淡出；空的时候线是淡色，有字时实线是默认前景。线下一行左边是房间，右边是平线问的模型与思考档位；斜杠菜单打开时占这一行的位置，至多五项，光标所在那项前画蓝色的 `▎`，最后一行说还有几项与哪些键。来源：`docs/frontend-method.md` §7I。
-/

/-! D8 窗口尚有空行时活动区贴底

决定：写完本帧的新转写后，先算活动区完整的高度；它的第一行若能向下移而让整个活动区仍在可见窗口内，就移到能让最后一行落在窗口底行的位置，否则从转写之后继续，由终端照常滚动。

理由：空城里把 composer 紧跟在横幅后面会让人以为输入区浮在上方；贴底后，人的视线与终端里常见的输入位置一致。

被否：①改用全屏备用屏——会失去主屏与终端自己的回滚，违背 D1；②只在启动时补固定数量的空行——缩放、多行输入与菜单改变高度后位置就错了。`console_ffi::tests::the_composer_sits_at_the_bottom_while_the_window_has_room` 在终端模拟器里比对整屏与光标；既有逐帧测试守住后续转写与擦除。
-/

/-! D15 Mei 视口是另一次整帧绘制，主屏增量帧不变

决定：`viewport.draw` 把一帧 scene 画成可见窗口。Mei 的正文通栏、使用默认前景的明暗，转写裁成当前阅读范围，活动区锚在窗口底部。帧叶从参数取指示符的中性灰阶与阅读位置，不读时钟。它返回实际绘制的运行点坐标，壳可在两次整帧之间仅重画该格并恢复光标。`inline_frame` 的擦除、贴底与「Type /web to open the WebUI」提示保持原样，城控制台仍只走那一条。`Ending::Failed` 的线上值是 3，代表进程失败。

理由：Mei 要在尺寸变化后重排整段对话，又不能改掉城已经对照过的主屏字节。同一片叶子里两条绘制，scene 词表不增加新的 part。

被否：把增量帧改成每帧清屏——城的回滚和既有 vt100 对照都会变。

视口由布局把原始日志序号、卡片片段、源行位置与正文列投影到当前物理格。正文列不计时间栏与消息标记。普通文本与 Markdown 以源行的字节起点识别同一行。软折行只推进同一逻辑行内的位置。表格可见字符按源行与源列定位，绘制造成的分栏、填充和续行不改变其位置。定位与选区重用这一投影。组合符随所属基字符复制，省略号经过正常字符路径。原始行决定 Markdown 块结构，行内公式保留代码跨度及链接目标。表格的转义管道作为单个可见字符测宽和绘制。Mermaid 边保留两端完整标识，横向放不下时改为纵向，标识可续行。

表格选区按源行及单元格的顺序复制，在相邻选中单元格之间使用表格分隔符。高亮只投影源范围内的单元格字符。续行造成的物理顺序不参与剪贴板排序。
-/

/-! D16 Unix 的渲染归档携带 Zig 编译器运行时以提供安全构建所生成的栈探针。Windows 保留宿主工具链的栈探针，归档省略 Zig 运行时，因为一同链接的壳已打包它，COFF 链接器拒绝重复的弱默认符号。两者均保持 ReleaseSafe。理由：正文渲染的栈帧增长可触发栈探测，宿主链接需要完整实现；固定正文缓冲大小或关闭栈探测不能保证该接口。 -/

/-! D9 入口提示说明人的动作与结果

决定：主屏横幅的入口提示写成「Type /web to open the WebUI」，其中 `/web` 用粗体，其余字保持淡色。Mei 视口不画这句（`Calyx/Spec.lean` D16）。

理由：这句话同时说出要输入什么与会打开什么；粗体只强调命令本身，不占用 ACCENT 的「在动或被选中」语义（D3）。

被否：只把 `page` 换成 `WebUI`——仍没有说明人要输入命令；给整句换蓝色或黄色——静态提示既不是「在动或被选中」，也不是「需要人或出了缺」。`console_ffi::tests::a_session_reads_as_one_left_edge_with_times_beside_it` 比对整句与命令的 SGR 样式。
-/

/-! D10 docs.rs 构建不编译 Zig 叶子

决定：`build.rs` 在环境变量 `DOCS_RS` 已设时直接成功返回，不读钉、不起 `zig`、不发出链接指令；并以 `cargo::rerun-if-env-changed=DOCS_RS` 登记该变量，使随后未设该变量的构建重跑脚本、而不是沿用这次跳过。docs.rs 的沙箱无 Zig、无外网；`cargo rustdoc` 只为库生成文档，不链接原生叶子，安全包装的 rustdoc 不依赖那片静态库。

理由：crates.io 读者打开 docs.rs 上的 `sprawling` 应看到 rustdoc；失败原因若是缺 Zig，文档就不可达。

被否：①在 docs.rs 装 Zig——沙箱无网络且无 Zig；②发出指向空归档的链接指令——rustdoc 不链接，空归档是多余步骤。

代价：docs.rs 上的文档描述的是安全 Rust 包装的接口；叶子的机器码与 `zig test` 行为不在那里验证。

重开参数：docs.rs 开始要求最终链接原生库，或文档需要展示叶子的符号。
-/

/-! D11 Calyx 的 Zig 由 `Calyx/build.zig` 构建；本包链接叶并宿主窄 Rust 面

决定：`Calyx/ffi/build.rs` 在 `Calyx/` 下调用 `zig build`（`Calyx/build.zig`），于
`ReleaseSafe` 产出帧叶静态库与壳静态库/可执行文件所需产物。壳实现在 `Calyx/shell`
（`Calyx/Spec.lean` D7）；本包保留 scene 词表的 Rust 定义与叶的安全 `draw`，供测试与
任何仍从 Rust 构图的宿主使用。壳在进程内直接写 scene、调叶，不经 Rust 画帧。docs.rs
跳过（D10）同时跳过 Zig 构建。

理由：根 `build.zig` 是 Calyx 下 Zig 的单一构建权威；叶与壳同图避免三元组/PIC/钉检查分叉。

被否：①继续仅 `zig build-lib` 单文件编叶——无法承载整壳；②为壳另开第三 FFI crate——在
`unsafe` 仍须落在 `LEAVES` 内时扩大例外面而无收益。
-/

/-! D12 本地时刻经 `zone` 读系统时区，chrono 不进树

决定：`console_ffi::zone::seconds_since_midnight(millis)` 把 UTC 毫秒换成系统本地时区里那一刻的午夜以来秒：Windows 经 `FileTimeToSystemTime`＋`GetTimeZoneInformation`＋`SystemTimeToTzSpecificLocalTime`（OS 按那一刻自己的夏令时规则换算），Unix 经 `libc::localtime_r`；这几个调用是 `unsafe` 绑定，住在本包，各带一行 `SAFETY:`。`Calyx` 侧的 `console::local_time::local` 只调它。

理由：RUSTSEC-2026-0543（chrono 全部版本停止维护，2026-11-17 发布）经 Calyx 的直依赖进锁，release 的 advisories 门按设计在任何暴露下保持红色；console 场景时刻是 chrono 在树里的最后一个用途，其余时间需求已由自研 UTC 时钟（`runtime::clock`）与 std 承担。不再多养一个日历库，也不要一个清除条件不在我们手里的 ignore。

定规：系统与 agent 读得到的事实恒为 UTC——唯一时间权威是 `runtime::clock` 的 ISO UTC（`Sprawling/runtime/Spec.lean` §8-10），记录、打戳行与 ISO 串不是 UTC 就是带显式偏移的 UTC；本地时刻只允许渲染给人的表盘（console 条目的 `HH:MM:SS`，D6），由 UTC 毫秒换算，不落盘、不进任何 agent 读得到的面。本模块是这条边界上唯一的例外，重开它需要先说动这条定规。

被否：①`deny.toml` 登记对 RUSTSEC-2026-0543 的 ignore——chrono 留在锁里，清除条件在上游；②chrono 换 `time`/`jiff`——`runtime::clock` 已覆盖 UTC 数学，这两个 crate 只为取本地偏移而来；③取「当前偏移」做算术——跨夏令时边界的旧时刻会差一小时，而 `FileTimeToLocalFileTime` 与 `localtime_r` 按时刻换算，与 chrono 的旧行为等价。

剩余限制（写明，不当作已解决）：只读系统配置的那一个本地时区，不做多时区；两个平台调用失败时该时刻不渲染（与旧 chrono 答 `None` 相同）。
-/

/-! D13 新增与变更的 Rust 文件零 `unsafe`，D1 与 D12 的既有调用按文件迁移

决定：`xtask unsafe` 对变更文件拒绝任何真实 `unsafe` token；D1、D11、D12 中的 `unsafe` 与 `SAFETY:` 形状只适用于未触及旧实现。触及文件须以安全接口或重设计的进程边界完成同一契约，不得迁移 token 或添加 suppression。旧债清零后，本包 lint 差异与 guard 中对应记录删除。

被否：立即把本包 `unsafe_code` 改为 `forbid`——现有 Zig 与系统绑定尚未迁移时只会使构建失败，不构成替代实现。
-/
