-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# 验收世界：替身把一个请求放进哪个 run，与按序走、停在第一处失败

规定验收世界 `Sprawling.Acceptance`（`tools/adversary/src/Sprawling/Acceptance/Script.lean`、
`tools/adversary/src/Sprawling/Acceptance/Walk.lean`）依赖的性质。

**替身按 run 分开作答**（citysim D11、`tools/citysim/Spec.lean` §8-13）。一个请求带回它这个
run 至今拿到过的调用 id；替身由其中最靠后的那一个认出它是哪个 run 的第几条之后，答那个 run 的
下一条。一个 id 都没带的请求开启下一个还没开启的 run。下面的参照定义只陈述这条放置规则，不陈述
id 怎样从正文里读出来——那一半在 citysim 的 Rust 里，由它自己的测试守着。

由放置规则推出验收世界要用的三件事：一个续轮的位置与它之前到过什么无关，所以两个 run 怎么交错、
一个请求被重发几次、城被杀之后接着送同一段对话，答的都是同一条；第一轮按到达的次序依次开启
脚本里的 run，夹在中间的续轮不占位置；往脚本后面追加 run 不改动任何已有 run 的回答。

**按序走、停在第一处失败。** 每一步站在前面几步留下的城上，接着走只会把一个原因报成许多个；
报出来的是第一步失败的那一步，它之前的每一步都通过了。
-/

/-! ## 完整客户端的 HTTP 验收接口

本段是环境接口说明，不是证明。验收世界 firstDay 在注册 provider 前从正在服务的二进制获取首页，
要求首页引用同源的 JavaScript 与 CSS，并逐一获取这些引用，拒绝空内容、回退首页和伪装为脚本或样式的 HTML；资源的 Content-Type 须与引用类型相符。
地址从 Ground 的实际端口读，资源地址从首页读，不另声明 bundle 路径或文件名。
占位页、只有 index.html、缺脚本或样式的应用均须失败；curl 不存在、HTTP 拒绝、超时同样失败，
恢复是补齐验收环境或重建完整应用。`acceptance client` 用同一 Stage.serving 与 clientDelivered 单独检查调用者指定的二进制，成功退零、拒绝退一，并在所有路径清理临时城与服务进程；firstDay 仍在 provider 注册前调用这一步。fresh 作业把 HTML 写入已构建 bundle 的脚本后重建二进制，以真实 HTTP 返回验证拒绝，之后原始归档仍用于完整 walk。`acceptance client-ui <script> <output>` 在同一次 Stage.serving 中先执行 clientDelivered，再把实际端口和证据目录交给调用者指定的 browser 脚本；脚本失败使验收失败，截图及结果由脚本写入 output，服务进程在所有路径结束。browser 连接这次服务的 loopback 地址，Settings 的地址与文字读 client 的 route/tree/groups/lang 权威，截图从实际运行页面获取。焦点验收枚举本次 Settings 原生 modal 内当前可通过 Tab 到达的 light DOM 控件，以元素身份固定清单与起点，关闭的 `details` 中不可见的后代不入清单，其可见的 `summary` 仍入清单；原生可见性检查与 CSS visibility、disabled、inert 和 tabindex 一起决定候选，布局矩形本身不能证明可到达，透明度为零也不取消键盘可到达性。正反两个方向各走到起点再次出现；每个方向必须先逐一到达清单全部控件且没有提前重复，遍历期间清单改变即拒绝，固定次数采样不能代替遍历。每次观察必须是同一个仍在 top layer 的 modal；文档持焦时 activeElement 必须在该框内，框外只接受 document.hasFocus=false 且 activeElement 为 BODY 的浏览器 chrome 停点，下一次同方向 Tab 必须使文档重新持焦并回到该 modal。非 BODY 框外元素、文档持焦的 BODY、关闭或更换 modal、连续 chrome 停点、跳过控件和不返回起点均使验收失败。清单遇到 shadow DOM、iframe 或原生 radio 等尚未覆盖的 sequential focus scope 时明确拒绝，不据不完整枚举声称全控件遍历。检查输出两个方向的清单、逐步观察和闭环结果，失败也保留已取得的观察。这些是 browser 环境验收要求，不是 HTML 引擎的 Lean 证明；关闭按钮、Esc、backdrop、combobox 与像素仍由实际运行验收判断。runtime 选择 fresh 同一矩阵与归档生命周期并附加此步骤，不替代完整 walk。OS 写入只允许可丢弃 runner。现有首次起城、skills 与历史恢复步骤继续走同一 acceptance。
-/

/-! ## gauge 的进程边界验收

本段是环境接口说明，不是证明。`acceptance gauge <output>` 对 SPRAWLING_BIN 指定的真实归档二进制
测量相同样本数的 status、无效 call 与自行终止的 shell 子进程，分别要求所有 run 成功、失败、取消；
gauge 自己必须完成测量并报告 spread，run 行与 spread 的 failed 必须一致。样本、主机类别、单位由 gauge
输出，不重算 percentile；stdout、stderr 与命令保存到 output。取消只作用于测量的可丢弃 shell 子进程，
不终止构建或 gauge。本检查不声称覆盖人中断 gauge 自身后的清理。
-/

/-! ## 安装后版本的验收接口

本段是环境接口说明，不是证明。`acceptance version <name> <version>` 经 Door 运行
调用者指定的二进制 `status`，要求退出零且输出首行以 `<name> <version> (` 开头。
名称与版本由调用者读取待验源码的 Cargo 清单，不在检查器定义发行版本或成熟度。
缺二进制、命令失败、输出不符均退出非零并说明失败，后续 walk 不运行。
-/

namespace Adversary.Acceptance

/-- 一份已从 HTTP 获取的资源；字段是浏览器可执行内容的环境观察，不描述 JS 语法。 -/
structure DeliveredAsset where
  nonempty : Bool
  html : Bool
  mimeMatches : Bool

/-- 每个首页引用都须通过内容准入，任一坏响应使整组拒绝。 -/
def admittedAssets (assets : List DeliveredAsset) : Bool :=
  assets.all fun asset => asset.nonempty && !asset.html && asset.mimeMatches

/-- 任意长度响应序列被准入后，每个成员都非空、不是 HTML，且 MIME 与引用类型相符。 -/
theorem admitted_assets_are_executable_content (assets : List DeliveredAsset)
    (asset : DeliveredAsset) (member : asset ∈ assets) (held : admittedAssets assets = true) :
    (asset.nonempty = true ∧ asset.html = false) ∧ asset.mimeMatches = true := by
  have checked := List.all_eq_true.mp held asset member
  cases h : asset.html <;> simp_all [Bool.and_eq_true]

/-- 坏响应前后追加任何响应不能把拒绝变成准入。 -/
theorem a_bad_asset_is_rejected_in_every_surrounding_sequence
    (before after : List DeliveredAsset) (asset : DeliveredAsset)
    (bad : (asset.nonempty && !asset.html && asset.mimeMatches) = false) :
    admittedAssets (before ++ asset :: after) = false := by
  simp [admittedAssets, List.all_append, List.all_cons, bad]

/-- 一个请求带回来的东西：一个脚本 id 都没带，或它最近拿到的是第 `run` 个 run 的第 `reply` 条。 -/
inductive Ask where
  | opening
  | after (run reply : Nat)
deriving DecidableEq, Repr

/-- 一个请求开启几个 run：第一轮开启一个，续轮一个都不开启。 -/
def opens : Ask → Nat
  | .opening => 1
  | .after _ _ => 0

/-- 已经开启了 `opened` 个 run 时，一个请求放在哪一条：(run, 第几条)。 -/
def place (opened : Nat) : Ask → Nat × Nat
  | .opening => (opened, 0)
  | .after run reply => (run, reply + 1)

/-- 一串请求按到达的次序进来，每一个放在哪一条。 -/
def placeAll : Nat → List Ask → List (Nat × Nat)
  | _, [] => []
  | opened, ask :: rest => place opened ask :: placeAll (opened + opens ask) rest

/-- 脚本在某个位置上的那条回复；位置不在脚本里时没有回复（替身以拒绝作答）。 -/
def replyAt (runs : List (List α)) (at_ : Nat × Nat) : Option α :=
  runs[at_.1]?.bind (·[at_.2]?)

/-- 一个续轮的位置不取决于此前开启过多少个 run：重发的请求、城回来之后接着送的同一段对话，
都放在同一条上。 -/
theorem a_continuing_request_is_placed_whatever_was_opened (one other run reply : Nat) :
    place one (.after run reply) = place other (.after run reply) := rfl

/-- 不论前面交错着什么，一个带回第 `run` 个 run 第 `reply` 条的请求放在那个 run 的下一条。 -/
theorem every_run_is_answered_from_its_own_replies (opened : Nat) (asks : List Ask) (index run reply : Nat)
    (arrived : asks[index]? = some (.after run reply)) :
    (placeAll opened asks)[index]? = some (run, reply + 1) := by
  induction asks generalizing opened index with
  | nil => simp at arrived
  | cons ask rest ih =>
    cases index with
    | zero =>
      simp only [List.getElem?_cons_zero, Option.some.injEq] at arrived
      subst arrived
      rfl
    | succ index =>
      simp only [List.getElem?_cons_succ] at arrived
      simp only [placeAll, List.getElem?_cons_succ]
      exact ih (opened + opens ask) index arrived

/-- 第一轮按到达的次序开启 run：一个第一轮开启的是第「它之前到过几个第一轮」个 run，夹在中间的
续轮不占位置。 -/
theorem openings_take_the_runs_in_order (opened : Nat) (asks : List Ask) (index : Nat)
    (arrived : asks[index]? = some .opening) :
    (placeAll opened asks)[index]? = some (opened + ((asks.take index).map opens).sum, 0) := by
  induction asks generalizing opened index with
  | nil => simp at arrived
  | cons ask rest ih =>
    cases index with
    | zero =>
      simp only [List.getElem?_cons_zero, Option.some.injEq] at arrived
      subst arrived
      simp [placeAll, place]
    | succ index =>
      simp only [List.getElem?_cons_succ] at arrived
      simp only [placeAll, List.getElem?_cons_succ, ih (opened + opens ask) index arrived,
        List.take_succ_cons, List.map_cons, List.sum_cons, Nat.add_assoc]

/-- 往脚本后面追加 run，已有的每个 run 答的仍是原来那一条：检查走到半路才写下的 run 改不动
已经在答的那些（citysim D15）。 -/
theorem a_grown_script_answers_the_runs_it_held_alike (runs more : List (List α)) (run reply : Nat)
    (held : run < runs.length) :
    replyAt (runs ++ more) (run, reply) = replyAt runs (run, reply) := by
  simp [replyAt, List.getElem?_append_left held]

/-- 两个逐条相同的 run，第几条答的都一样：两个同时开启的 run 谁先到替身无从分辨，脚本把它们写成
一样，到达的次序就不改变任何一个拿到什么。 -/
theorem alike_runs_answer_alike (runs : List (List α)) (one other reply : Nat)
    (alike : runs[one]? = runs[other]?) :
    replyAt runs (one, reply) = replyAt runs (other, reply) := by
  simp [replyAt, alike]

/-- 按序走：每一步答通过或失败；走到第一处失败就停，报出它的位置。 -/
def firstBroken : List Bool → Option Nat
  | [] => none
  | true :: rest => (firstBroken rest).map (· + 1)
  | false :: _ => some 0

/-- 报出来的那一步确实失败了，它之前的每一步都通过了。 -/
theorem the_reported_step_broke_and_every_earlier_one_held (steps : List Bool) (index : Nat)
    (reported : firstBroken steps = some index) :
    steps[index]? = some false ∧ ∀ earlier, earlier < index → steps[earlier]? = some true := by
  induction steps generalizing index with
  | nil => simp [firstBroken] at reported
  | cons step rest ih =>
    cases step with
    | false =>
      simp only [firstBroken, Option.some.injEq] at reported
      subst reported
      exact ⟨rfl, fun earlier below => absurd below (Nat.not_lt_zero earlier)⟩
    | true =>
      simp only [firstBroken, Option.map_eq_some_iff] at reported
      obtain ⟨inner, found, rfl⟩ := reported
      obtain ⟨broke, held⟩ := ih inner found
      refine ⟨by simpa using broke, fun earlier below => ?_⟩
      cases earlier with
      | zero => rfl
      | succ earlier => simpa using held earlier (by omega)

/-- 每一步都通过时，没有一步被报出来。 -/
theorem a_walk_that_held_reports_nothing (steps : List Bool) (held : steps.all id = true) :
    firstBroken steps = none := by
  induction steps with
  | nil => rfl
  | cons step rest ih =>
    simp only [List.all_cons, Bool.and_eq_true, id] at held
    obtain ⟨first, others⟩ := held
    subst first
    simp [firstBroken, ih others]

end Adversary.Acceptance
