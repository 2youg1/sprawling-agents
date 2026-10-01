-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import crates.runtime.spec.Tools.ChosenPath

/-!
# runtime::tools::bound_reader

规定 `tools::bound_reader`（`crates/runtime/src/` 下同名的文件）。模型点名的字节，经读界判过后按字节读的那一扇门。本文件是 `crates/runtime/Spec.lean` 的一个分部；下面每一节保留它在 runtime 规格里的标签 §8-n，别处引作 `crates/runtime/Spec.lean §8-n`。
-/

/-!
### 8-59 runtime::tools::bound_reader：模型点名的字节，经读界判过后按字节读（形状 4 适配器；sprawling-SPEC 8-131、8-142）


**问题**：`read` 只交文本。城的工具 `ocr` 与 `transcribe` 要的是一张图、一段录音的字节，它们在读界之内的任意一栋楼里，或在连接器存进 CAS 的块里（§8-27-10、D15）。判「这条模型选的路径能不能读」的是 `chosen_path`（§8-30-1），在 accounting 里再写一份判定就是第二个权威。本节把那份判定公开成一扇按字节读的门，城里读别楼文件与 `cas:` 块的工具都只经它。

```rust
// runtime::tools::bound_reader（形状 4 适配器）
#[derive(Clone)]
pub struct BoundReader { /* city_root、bound、block_store —— runtime::tools 内可见 */ }
impl BoundReader {
    pub fn new(city_root: &Path, bound: ReadBound, block_store: &Path) -> BoundReader;
    // asked：城相对路径、城内的绝对路径、`cas:` 或 `file:` Locator。action 是拒词里点名的那件工具。
    pub fn open(&self, asked: &str, action: &'static str) -> Result<Opened, AxError>;
}
pub struct Opened { /* named、字节的来源 —— 私有 */ }
impl Opened { pub fn named(&self) -> &Named; }
impl std::io::Read for Opened { … }
pub enum Named { File(Address), Block(B3Hash) }   // 路径与 `file:` 是 File，`cas:` 是 Block

// runtime::pipeline::connector（形状 1 判定）：一张图的唯一认法
pub fn png_picture(bytes: &[u8]) -> Result<ImageRef, AxError>;
```

- **判定是 `read` 的那一套，次序也一样。** 以 `cas:`／`file:` 开头的走 `read::locator`（§8-29-5）：`cas:` 块按 `judged_at` 选出的楼过 `admit`，`file:` 按自己的地址过 `admit`。其余走 `within_city`、`admit`、`land`（§8-30-1），打开之后经 `still_judged` 核对打开的就是判过的那个文件。拒绝与 `read` 同码：文法不对＝`E_INVALID_ARGS`；reserved subtree、读界关上的楼、城外的绝对路径、经链接出城＝`E_GATE_DENIED`；不存在的 `cas:` 块（存块时没有记下任何楼）＝`E_GATE_DENIED`；判定时不在的文件与目录＝`E_INVALID_ARGS`；在而打不开＝`E_STORAGE_FATAL`。拒词的 action 是调用它的那件工具，所以模型读到的是自己哪一次调用被拒。
- **catalog 名不经这扇门。** 一件 skill 由人放进楼的阅览室，读它是 `read` 的事；一张图或一段录音不在 catalog 里，`ocr` 与 `transcribe` 收到一个 catalog 名，按路径判，多半是 `E_INVALID_ARGS`。
- **`read` 的 Locator 也经它。** `ReadTool` 持一个 `BoundReader`，`cas:`／`file:` 读出字节再按 UTF-8 交文本。路径仍走 `read` 自己那条路：没命中时它要列 `nearby`（§8-29-4），那是读文本的答复；按字节读的只说「不在」，恢复语指向 `search`。
- **门不设上限，上限归收字节的那个值。** `Opened` 是一个 `Read`：文件按需读，读多少由收它的值定，`gateway::Recording::read_from` 读到它的上限多一字节为止（`crates/gateway/Spec.lean` §8-33）。`cas:` 块与 `file:` 的字节在打开时已整份在内存里，因为 `Cas::get` 与 `storage::blob_at` 只交整份；它们是这座城自己存下的，大小受存它的那条路约束。`ocr` 把一个文件整份读进来再判 `IMAGE_MAX_BYTES`，与 `read` 整份读一个文件相同：`ImageMaxBytes` 不交出它的数，按上限读要 kernel 给它一个读法。
- **`Named` 说字节从哪里来，容器怎么认归调用方。** 文件有名字，块没有。录音的容器在文件上看扩展名、在块上看开头的字节（`crates/gateway/Spec.lean` §8-34），那是 gateway 那张表的事，本模块不认容器。
- **一张图只有一种认法：`png_picture`。** 连接器存图（§8-27-10）与城工具 `ocr` 读图都调它：字节在 `IMAGE_MAX_BYTES` 之内、PNG 头读得出宽高时，答一个指向这些字节的 `cas:` 哈希的 `ImageRef`；超限是 `IMAGE_MAX_BYTES` 自己的拒绝，头读不出是 `E_INVALID_ARGS`，主题说出为什么。连接器把拒绝写成那一行 `[picture left out: <主题>; <恢复语>]`。
- **验收**：`runtime::tools::bound_reader` 的测试。门对读界外的路径、reserved subtree、不存在的 `cas:` 定位符各拒一次，码与 `read` 对同一个参数的拒绝相同；楼里的文件与为本楼存下的块按字节读回，`named` 分别是 `File` 与 `Block`。
-/

namespace Runtime.Tools.BoundReader

open Runtime.Tools.ChosenPath

/-- 模型写下的参数，按门怎么读它分三种：一条城内路径、一个 `cas:` 块（以它的哈希识别）、一个 `file:` Locator（`<addr>@<oid>`）。哪一种由前缀定：以 `cas:` 或 `file:` 开头的是 Locator，其余是路径，城内地址不含冒号，所以两者不相交。 -/
inductive Asked where
  | path (written : String)
  | cas (hash : Nat)
  | file (written : String)

/-- `bound_reader::Named`：字节从哪里来。 -/
inductive Named (Address : Type) where
  | File (addr : Address)
  | Block (hash : Nat)

/-- 块仓记下的来源：一个块为哪几栋楼存过（`Cas::origins`，`crates/storage/Spec.lean` §8-3）。 -/
structure Store (Address : Type) where
  origins : Nat → List Address

/-- 来源里第一栋读界开着的楼。 -/
def first_open {Address : Type} (rules : Rules Address) : List Address → Option Address
  | [] => none
  | building :: rest => match rules.bound building with
    | .Open => some building
    | .Confidential => first_open rules rest
    | .RulesUnreadable => first_open rules rest

/-- `read::locator::judged_at`：`cas:` 块按哪栋楼判读取界的唯一判定处。取读者能读的第一栋；一栋都读不了就取第一条来源，让判定按那栋楼的理由拒绝；没有来源的块拒绝。 -/
def judged_at {Address : Type} (rules : Rules Address) (origins : List Address) :
    Except Code Address :=
  match first_open rules origins with
  | some building => .ok building
  | none => match origins with
    | [] => .error .GateDenied
    | first :: _ => .ok first

/-! D16 按字节读的门在 runtime，交出一个 `Read` 与字节的来处

**决定**：`chosen_path` 的判定经 `BoundReader` 公开（§8-59）；它收模型写下的参数，判过之后交回一个 `Opened`：一个 `Read`，加上 `Named` 说它是一个文件还是一个块。`read` 读 Locator 也经它。

**理由**：判一条模型选的路径要依次做四件事（换成城里的拼写、文法与保留区与读界、解开链接再判、打开后核对没换过），漏掉任何一件就是从侧门把 `admit` 拒掉的东西交出去；把这四步交给每个调用方去拼，等于每个调用方都要记得它们。交一个 `Read` 而不是一份字节，是因为上限属于收字节的那个值：录音的上限在 `gateway::Recording`，图的上限在 `IMAGE_MAX_BYTES`，门若收一个上限参数，这两个数就要被抄到调用点。`Named` 是因为录音的容器在文件上与在块上认法不同，调用方要知道它拿到的是哪一种。

**被否**：①把 `chosen_path` 的几个函数直接公开，让 accounting 自己拼——四步的次序与 `still_judged` 都会在 crate 外再写一遍；②门收一个上限、交回 `Vec<u8>`——上限的数离开它的值，而 `ImageMaxBytes` 本来就不交出它的数；③门放在 accounting——它照样要 `chosen_path` 公开，判定仍会分在两个 crate。

**重开参数**：一个要列目录或要 `nearby` 的按字节读取者出现时，把 `read` 的没命中答复挪进这扇门。
-/

/-- `BoundReader::open`：判定是 `read` 的那一套，次序也一样。路径走 `admit` 再走 `land`，判定时不在的不打开；`file:` 按自己的地址过 `admit`，读的是那一次提交里的字节；`cas:` 按 `judged_at` 选出的楼过同一道判定。`open` 在 Lean 里是关键字，故写作 `«open»`。 -/
def «open» {Address : Type} (rules : Rules Address) (disk : Disk Address)
    (store : Store Address) : Asked → Except Code (Named Address)
  | .cas hash => match judged_at rules (store.origins hash) with
    | .error code => .error code
    | .ok building => match judge rules building with
      | .error code => .error code
      | .ok _ => .ok (.Block hash)
  | .file written => match admit rules written with
    | .error code => .error code
    | .ok addr => .ok (.File addr)
  | .path written => match admit rules written with
    | .error code => .error code
    | .ok addr => match land rules disk addr with
      | .error code => .error code
      | .ok (.Present _) => .ok (.File addr)
      | .ok (.Absent _) => .error .InvalidArgs

/-- `first_open` 找到的楼是来源之一，而且读界开着。 -/
theorem first_open_is_an_open_origin {Address : Type} (rules : Rules Address) :
    ∀ (origins : List Address) (building : Address),
      first_open rules origins = some building →
        building ∈ origins ∧ rules.bound building = .Open
  | [], _, found => by simp [first_open] at found
  | head :: rest, building, found => by
    cases verdict : rules.bound head with
    | Open =>
      simp [first_open, verdict] at found
      subst found
      exact ⟨by simp, verdict⟩
    | Confidential =>
      simp [first_open, verdict] at found
      have later := first_open_is_an_open_origin rules rest building found
      exact ⟨by simp [later.1], later.2⟩
    | RulesUnreadable =>
      simp [first_open, verdict] at found
      have later := first_open_is_an_open_origin rules rest building found
      exact ⟨by simp [later.1], later.2⟩

/-- `judged_at` 选的楼恒是这个块的来源之一。 -/
theorem judged_at_names_an_origin {Address : Type} (rules : Rules Address)
    (origins : List Address) (building : Address)
    (chosen : judged_at rules origins = .ok building) : building ∈ origins := by
  unfold judged_at at chosen
  cases found : first_open rules origins with
  | some open_ =>
    simp [found] at chosen
    subst chosen
    exact (first_open_is_an_open_origin rules origins open_ found).1
  | none =>
    simp [found] at chosen
    cases origins with
    | nil => simp at chosen
    | cons first rest =>
      simp at chosen
      subst chosen
      simp

/-- **一个块只在为它存过的、读界开着的楼里读得到。** 归属按存块时记下的来源判，不按账本里谁写了这个哈希判：模型写的文字会落进带地址的行，那样的归属可以伪造。 -/
theorem a_block_is_read_only_at_a_building_it_was_put_for {Address : Type}
    (rules : Rules Address) (disk : Disk Address) (store : Store Address) (hash : Nat)
    (named : Named Address) (opened : «open» rules disk store (.cas hash) = .ok named) :
    ∃ building ∈ store.origins hash,
      rules.is_reserved building = false ∧ rules.bound building = .Open := by
  simp only [«open»] at opened
  cases chosen : judged_at rules (store.origins hash) with
  | error _ => simp [chosen] at opened
  | ok building =>
    cases judged : judge rules building with
    | error _ => simp [chosen, judged] at opened
    | ok answer =>
      obtain ⟨_, notReserved, open_⟩ :=
        judge_answers_the_address_it_was_given rules building answer judged
      exact ⟨building, judged_at_names_an_origin rules _ building chosen, notReserved, open_⟩

/-- 存块时没有记下任何楼的块，`E_GATE_DENIED`。 -/
theorem a_block_put_for_no_building_is_refused {Address : Type} (rules : Rules Address)
    (disk : Disk Address) (store : Store Address) (hash : Nat)
    (unclaimed : store.origins hash = []) :
    «open» rules disk store (.cas hash) = .error .GateDenied := by
  simp [«open», judged_at, first_open, unclaimed]

/-- **门对一条路径的拒绝与 `read` 同码。** `read` 拒在 `admit` 上的，门以同一个码拒；模型读到的是同一句话，只是 action 换成了调用它的那件工具。 -/
theorem a_path_read_refuses_is_refused_with_the_same_code {Address : Type}
    (rules : Rules Address) (disk : Disk Address) (store : Store Address) (written : String)
    (code : Code) (refused : admit rules written = .error code) :
    «open» rules disk store (.path written) = .error code ∧
      «open» rules disk store (.file written) = .error code := by
  simp [«open», refused]

/-- **按路径打开的文件，是在它真实落下的地方被判过、判定时在场的那一个。** -/
theorem a_file_opened_by_path_was_judged_where_it_lands {Address : Type}
    (rules : Rules Address) (disk : Disk Address) (store : Store Address) (written : String)
    (addr : Address) (opened : «open» rules disk store (.path written) = .ok (.File addr)) :
    admit rules written = .ok addr ∧
      ∃ real, disk.real addr = .inside real ∧ rules.is_reserved real = false ∧
        rules.bound real = .Open ∧ disk.present real = true := by
  simp only [«open»] at opened
  cases admitted : admit rules written with
  | error _ => simp [admitted] at opened
  | ok found =>
    cases landed : land rules disk found with
    | error _ => simp [admitted, landed] at opened
    | ok located =>
      obtain ⟨real, where_, notReserved, open_, shape⟩ :=
        what_lands_was_judged_where_it_lands rules disk found located landed
      cases located with
      | Present _ =>
        simp [admitted, landed] at opened
        subst opened
        cases present : disk.present real with
        | true => exact ⟨rfl, real, where_, notReserved, open_, present⟩
        | false => simp [present] at shape
      | Absent _ => simp [admitted, landed] at opened

end Runtime.Tools.BoundReader
