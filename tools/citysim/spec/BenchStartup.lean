-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# citysim::bench_startup

规定 `just bench-startup` 的测量 Main `bin/bench_startup`（`tools/citysim/src/bin/bench_startup.rs`）与它的模块：`samples`、`actions`，以及 `actions` 下的 `archive`、`first_byte`、`footprint`、`history`（都在 `tools/citysim/src/bin/bench_startup/` 下）。本文件是 `tools/citysim/Spec.lean` 的一个分部；下面各节保留它们在 citysim 规格里的标签 §8-5、§8-5-1，别处引作 `tools/citysim/Spec.lean §8-5`。

能写成定理的是读数本身不说假话的两件：可疑样本只标注不剔除，每个样本都留在读数里，标注只看它是否超过中位的倍数（`every_sample_is_kept_and_marked_by_the_cut`）；主导子步是中位最大的那一个，没有子步切分时答 `None`（`the_dominant_step_has_the_largest_middle`、`no_steps_have_no_dominant`）。分位怎样取是 sprawling 的 `Spread`（`crates/sprawling/Spec.lean` §8-129-2），倍数 `SUSPICIOUS_TIMES` 也住那里，模型把中位与倍数之积当参数；计时与文件计数由 `samples`、`footprint`、`actions` 的测试守着（§16）。
-/

/-!
### 8-5 bench_startup 族：四动作压档的测量面（只实测，不优化）

四动作各给三件套（能否进 p99≤1ms／极限读数／主导成本件），本族只测不优化。本族是 `just bench` 的同族仪表：同一 citysim bin 面、同一 wall-clock 口径（测而不门，读数标注机器类属）。四行读数落在 `tools/xtask/budgets.toml` 的 `[install]`／`[startup]`／`[raise_city]`／`[open_session]`，无预算键故不门；机器类属、样本数与四个动作的子指标数字都写在行内，本规格不重抄它们。

```rust
// tools/citysim/src/bin/bench_startup.rs —— measuring Main：本族唯一计时采样点（`stamp()`，同 bin::bench 先例）；`SAMPLES` 是每动作次数的唯一之家，循环、预分配与报告同读它
// tools/citysim/src/bin/bench_startup/samples.rs —— shape: decision（时间以 Duration 入参；无时钟、无 I/O）
pub struct Samples { /* 一次构造点持有 ≥1 个 Duration；invalid state 不可拼写 */ }
pub use sprawling::monitor::spread::Share;   // P50, P95, P99：产品自己的那一个
pub enum SampleKind { Plain, Suspicious }   // > SUSPICIOUS_TIMES × p50 标可疑：Defender 实时扫描等外扰的标注位
pub enum Tier { Within, Outside }           // 第二档：p99 ≤ 1 ms
impl Samples {
    pub fn of(head: Duration, tail: Vec<Duration>) -> Samples;
    pub fn p(&self, Share) -> Duration;     // nearest-rank
    pub fn floor(&self) -> Duration;  pub fn peak(&self) -> Duration;
    pub fn kind_at(&self, usize) -> SampleKind;
    pub fn suspicious(&self) -> usize;
    pub fn tier(&self) -> Tier;
}
// tools/citysim/src/bin/bench_startup/actions.rs —— shape: adapter（薄驱动产品公面，口径即边界；两处跨进程动作走的都是产品自己的路径）
pub const SAMPLES: usize;
pub struct PerSample { pub processes: u64, pub files: u64, pub barriers: u64 }
pub struct Action { pub total: Samples, pub steps: Vec<(&'static str, Samples)>, pub per_sample: PerSample }
pub fn install(scratch: &Path, archive_path: &Path) -> Result<Action, AxError>;
pub fn startup(binary: &Path) -> Result<Action, AxError>;
pub fn raise_city(scratch: &Path) -> Result<Action, AxError>;
pub fn open_session(city: &Path) -> Result<Action, AxError>;
pub fn dominant(steps: &[(&'static str, Samples)]) -> Option<&'static str>;
// tools/citysim/src/bin/bench_startup/actions/archive.rs —— shape: adapter（发行档的格式读写都在这里：摘要、解包、写出 fixture 条目；`sha2`／`zip` 各在 workspace 清单里一个名字，打包步与本次读数不会对它们生出第二种拼法）
// tools/citysim/src/bin/bench_startup/actions/footprint.rs —— shape: adapter（一次 walk 供所有读者：一个样本在一棵树下写了多少文件、多少账行、某个名字在哪、以及样本之间如何把目录清平）
```

**四个计时边界**（起止、子步、决定见 D2／D3）。样本 200／动作（≥100）；p50/p95/p99 取 nearest-rank；读数全样本给出（基线永不减除），可疑样本只标注不剔除。

| 动作 | 起点 | 终点 | 子步 |
|---|---|---|---|
| ① install | 归档已就位（zip 在 scratch，网络不计） | 解出的可执行文件拉起 `version` 应答并退出 | 归档摘要校验（sha256）｜解包写可执行文件｜落位确认（进程创建＋运行到退出） |
| ② startup | `CreateProcess` 发出 | 轻命令 `version` 退出被观察到 | 进程创建（spawn 返回）｜程序运行（返回→退出） |
| ③ raise_city | `assembly::init_city` 调用发出 | 调用返回（创世记录落盘，每条账各带自己的屏障） | 无子步切分（不插桩产品）；主导件由计数×地板归因 |
| ④ open_session | `RunWorker::handle(Command::OpenSession)` 发出 | 调用返回（`session_opened` 已落账，房内下一 run 可开工即可接输入） | 同上 |

**子指标拆分（各自计数，先行）**：进程创建（①1／样、②1／样、③0、④0，时间取子步）；文件创建（`PerSample.files`：①＝解包写出的文件数，③＝创世城市树的文件数，④＝首样本前后 city 文件数之差；②自身不落盘，记 0 而不是把它被指向的那棵树算进来）与耐久屏障（`PerSample.barriers`，一账一屏障，地板引用 `just bench` 的 `durability_barrier` 行，不另起第二仪表）；验签哈希＝0（签名动作未接，记 0 并注明，见 D3）；Defender 实时扫描干扰＝可疑样本数与下标（`SampleKind`），①③ 每样本全新首触（必扫），② 复用同一映像（首样本后转热）。

**失败**：产品公面的失败原样抛 `AxError`，不新增码；测量自体的失败（被测二进制不在构建档目录等）用既有码走三段式（动作/主体/`AxCode`/recovery）。

**红**：`samples.rs` 的 nearest-rank 分位、可疑标注、第二档判定三个测试先行，跑一次见红再实现。`footprint` 三条（计数只数文件不数目录、按名找文件不论深度且不认目录、账行按行数而非按文件数）与 `actions` 一条（主导子步取中位最大者，无子步切分答 `None`）守的是**读数本身**：数错一个文件或指错一个主导件，报告就在说假话。

D2 **计时边界取「动作的可观察端点」，进程动作以退出为端点。** 四动作里两个跨进程（安装的落位确认、启动）：端点是被拉起进程**退出被观察到**，因为「可接受命令」在产品外部可观察的最短证据就是一条轻命令被应答完毕。落盘动作（建城、开 session）以公面调用**返回**为端点，因为返回即账本已带自身屏障落盘（落账先于效果）。被击败的备选：以进程内部时点（参数解析完成、监听就绪）为端点——那要在产品里插桩，为测量加一条不发货的分支，改写被测路径。

D3 **安装边界含归档摘要校验、不含 PATH 写入。** 摘要校验（sha256）是 `install.sh`／`install.ps1` 从归档就位到解包之间必经的一步，删掉它测的就是不验摘要的安装，故计为安装的子步并单列读数。签名验签不在这次测量里：发行件签名的验签侧有设计而签名动作未接（`tools/xtask/Spec.lean` §8-29），读数里这一子步记 **0** 并注明；签名动作接上之后，此子步只增不删。PATH 写入（`sprawling install` 的注册表写与桌面广播）在边界外：它是一次性的桌面状态写入，第二次运行幂等（`PathEdit::AlreadyPresent`），计进每样本会把桌面状态写入误报成安装成本。被击败的备选：整段 `install.sh` 全测——含网络下载与 shell 启动，而网络不在本族的计时口径内。

D4 **计量主语是 Rust measuring Main，不是 tools/adversary/ 也不是 criterion。** 四动作零行为断言，只计时；`tools/adversary/` 量化行为轨迹，Lean 侧不为墙钟定价。criterion 会是第二套仪表：本族挂 `just bench` 族，同一 wall-clock 口径（测而不门）。它拉起产品二进制——被测动作本身即进程边界；boundary 门判的是**检查**站哪一侧，其越过面 token（`CARGO_BIN_EXE`／`SPRAWLING_BIN` 等）本族一个不写，被测二进制取自构建档目录（`cargo build` 同时放置两个产物的地方），`just bench-startup` 先构建后测量，故不接手工路径也不会测到旧产物。被击败的备选：把四动作写进 `tools/adversary/`——那里没有秒表也没有本仓词汇，量出来的东西无法与 `just bench` 对表。

D5 **被测可执行文件的名字在本 crate 只重述一处，注释点名它的权威。** `executable_name()` 拼的是 `install.rs` 装出来的那个名字：`INSTALLED_STEM` 加本平台后缀。该事实的权威是 `tools/xtask/src/platform.rs` 每平台的 `binary` 字段，`cargo xtask artifact` 把发行侧的四种拼法（工作流矩阵、两个安装脚本、npm shim）钉在它上面；citysim 这一处不在那四种之内，它是唯一需要这个名字的**测量**读者。够不到权威的原因是位置而非取舍：`install` 模块住在 `crates/sprawling/src/main.rs`，二进制的模块不可 import，而 `xtask` 是工具不是依赖。本 crate 内只留这一处拼写——`shipped_binary` 找的路径名与 `archive_of` 写出的 zip 成员名都读它。**重开参数**：这个名字若移进 `sprawling` lib 成为公共面，本函数改为读它，重述随之删除。
-/

/-!
#### 8-5-1 首字节：`sprawling serve` 拉起到第一个字节（`ttfb`）

四个动作之外的第五行读数，量的是人开一座城要等多久：`CreateProcess` 发出（`serve <城> 127.0.0.1:<端口> --no-console --no-open`，`SPRAWLING_OPEN=never`）到对 `GET /` 读到第一个字节。端点取第一个字节而不是端口开始监听，因为人看见的是页面，而一个接受了连接却还答不出页的服务在人眼里仍是没开。轮询间隔 2 ms，单样本上限 300 s。

三座夹具城，同一个历史形状、三种长度：

| 城 | 记录 | 样本 |
|---|---|---|
| `empty` | `init` 出来的 3 条 | 20 |
| `l100k` | 2,000 个 run × 50 条 | 5 |
| `l400k` | 8,000 个 run × 50 条 | 3 |

一个 run 是 `run_started`、八个回合（`prompt_assembled`、`model_called`、`model_returned`、`tool_called`、`tool_result`、`checkpoint_committed`）与 `run_frozen`，正文长度与实测城市的记录相近。这是每回合一条 `prompt_assembled` 的账本形状：产品写的是每 run 一条 `prompt_assembled` 加每回合一条 `prompt_shape_compared`（`crates/runtime/Spec.lean` §8-39 第 5 条），而每回合一条的账本仍被读入，故夹具是合法输入，其折叠代价与一座真正工作过的城同量级，但不逐条同形。账本经 `storage::JsonlLedger::append_all` 按每批 10,000 条写入：分段、链与字节规范都是产品自己的，本族不拼一行账。

**夹具城留在 `<构建档目录>/../bench-cities/<名>`**，下次复用：40 万条是 376 MB，每次重写要付的时间比量它还多。复用只看那座城在不在；`xtask mem --city` 读的就是同一座城（`tools/xtask/Spec.lean` §8-30），于是首字节与启动峰值出自同一份历史。

**每座夹具城的读数旁打印它账本的摘要**（`citysim::ledger_digest`，D8），两条首字节读数只在摘要相等时可比。每个样本那次 `serve` 的标准错误写进 `<构建档目录>/../bench-cities/<名>.serve.log`（后一个样本覆盖前一个），报告里打印这个路径。其中以 `opened the city in` 开头的那一行是产品自己拆出的开城各段耗时（`crates/sprawling/Spec.lean` §8-121），以 `the history is proved` 开头的那一行是后台证明走完、写者开始接受命令的时刻（就绪时刻 M3，`crates/sprawling/Spec.lean` §8-122）：本族不解析它们，只把它们和首字节读数放在同一次开城旁边给人读。所以一个样本量完首字节之后并不立刻停掉 `serve`，而是等日志里出现证明的结局（`the history is proved` 或 `the ledger stopped taking writes`），至多 300 s；首字节读数在这之前已经取下，不受这段等待影响。

```rust
// tools/citysim/src/bin/bench_startup/actions/history.rs —— shape: adapter（一座有历史的夹具城：init 之后经产品的 Ledger 写入）
pub enum History { Empty, Runs(u32) }
pub fn fixture_city(cities: &Path, name: &str, history: History) -> Result<PathBuf, AxError>;
// tools/citysim/src/bin/bench_startup/actions/first_byte.rs —— shape: adapter
pub fn first_byte(binary: &Path, city: &Path, samples: usize, log: &Path) -> Result<Samples, AxError>;
```
-/

namespace Citysim.BenchStartup

/-- 一个样本看起来像动作本身还是像外扰（`samples::SampleKind`）。 -/
inductive SampleKind where
  | Plain
  | Suspicious
  deriving DecidableEq, Repr

/-- 每个样本按到达的次序标注（`Samples::of`）：超过 `cut`（中位乘 `SUSPICIOUS_TIMES`）的标可疑，其余平常。时间以微秒计的自然数代 `Duration`。 -/
def marks (cut : Nat) (samples : List Nat) : List SampleKind :=
  samples.map (fun time => if cut < time then .Suspicious else .Plain)

/-- 可疑样本只标注不剔除：每个样本都有一个标注，标在它到达的位置上，可疑当且仅当它超过 `cut`。 -/
theorem every_sample_is_kept_and_marked_by_the_cut (cut : Nat) (samples : List Nat) :
    (marks cut samples).length = samples.length
      ∧ ∀ (index time : Nat), samples[index]? = some time →
          ((marks cut samples)[index]? = some SampleKind.Suspicious ↔ cut < time) := by
  refine ⟨by simp [marks], ?_⟩
  intro index time at_
  by_cases above : cut < time <;> simp [marks, List.getElem?_map, at_, above]

/-- 两个子步里中位较大的那一个；相等时取后来的，同 Rust 的 `max_by`。 -/
def pick (best next : String × Nat) : String × Nat :=
  if best.2 ≤ next.2 then next else best

/-- 主导子步：中位最大的那一个；没有子步切分时没有（`actions::dominant`）。 -/
def dominant : List (String × Nat) → Option String
  | [] => none
  | first :: rest => some (rest.foldl pick first).1

theorem pick_is_at_least_both (best next : String × Nat) :
    best.2 ≤ (pick best next).2 ∧ next.2 ≤ (pick best next).2 := by
  unfold pick
  split <;> omega

theorem the_fold_is_at_least_every_step (rest : List (String × Nat)) :
    ∀ best, ∀ step ∈ best :: rest, step.2 ≤ (rest.foldl pick best).2 := by
  induction rest with
  | nil => intro best step member; simp at member; subst member; exact Nat.le_refl _
  | cons next more ih =>
    intro best step member
    have both := pick_is_at_least_both best next
    have later := ih (pick best next)
    simp only [List.foldl_cons]
    rcases List.mem_cons.mp member with rfl | member
    · exact Nat.le_trans both.1 (later _ List.mem_cons_self)
    · rcases List.mem_cons.mp member with rfl | member
      · exact Nat.le_trans both.2 (later _ List.mem_cons_self)
      · exact later step (List.mem_cons_of_mem _ member)

theorem the_fold_is_one_of_the_steps (rest : List (String × Nat)) :
    ∀ best, rest.foldl pick best ∈ best :: rest := by
  induction rest with
  | nil => intro best; simp
  | cons next more ih =>
    intro best
    simp only [List.foldl_cons]
    have chosen := ih (pick best next)
    rcases List.mem_cons.mp chosen with same | member
    · rw [same]
      unfold pick
      split <;> simp
    · exact List.mem_cons_of_mem _ (List.mem_cons_of_mem _ member)

/-- 主导子步是某一个子步，它的中位不小于任何一个子步的中位：报告指的主导件就是最贵的那一段。 -/
theorem the_dominant_step_has_the_largest_middle (first : String × Nat) (rest : List (String × Nat)) :
    ∃ chosen ∈ first :: rest, dominant (first :: rest) = some chosen.1
      ∧ ∀ step ∈ first :: rest, step.2 ≤ chosen.2 :=
  ⟨rest.foldl pick first, the_fold_is_one_of_the_steps rest first, rfl,
    the_fold_is_at_least_every_step rest first⟩

/-- 没有子步切分的动作（建城、开 session）没有主导子步，报告不编一个出来。 -/
theorem no_steps_have_no_dominant (steps : List (String × Nat)) :
    dominant steps = none ↔ steps = [] := by
  cases steps <;> simp [dominant]

end Citysim.BenchStartup
