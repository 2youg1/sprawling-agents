-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# kernel::skill

规定 `skill`（`crates/kernel/src/skill.rs`）：一件 skill 包的 canonical manifest。city 在准入时写它，runtime 在读取时凭它找到每个文件的 CAS digest；包 digest 是 manifest 字节的 BLAKE3，不再是整包规范串的 digest。
-/

/-!
## 1 需求分解

manifest 列出包内每一项的相对路径、种类（目录或文件）与文件 digest；`SKILL.md` 恰有一个文件项。索引只引用 manifest digest，运行读取仍需完整读取并校验 manifest 与目标文件。
-/

/-!
## 2 验收标准

同一组项目生成的字节确定；相对路径为空、含 `.`／`..`、反斜杠或冒号即拒；重名即拒；`SKILL.md` 缺失或重复即拒。`file(path)` 只回答该路径的文件 digest，不回答目录或别的名字。Rust 的 roundtrip、边界测试与下面的查找性质共同验收。
-/

/-!
## 3 假设与歧义

manifest 的 JSON 字段次序由 Rust struct 声明固定，数组在构造时排序；这里没有浮点数或平台路径分隔符。目录项只用于将来的只读 projection，本身不带 digest。非 canonical 但语义相同的 JSON 不作为写入形式；CAS 地址已固定实际字节。
-/

/-!
## 4 现状分析

旧实现把整包规范串作为一个 CAS blob，无法只校验并读取一个文件。新 manifest 保留同样的路径与字节边界，但把每个文件独立寻址，包身份改为 manifest 的地址。
-/

/-!
## 5 权威来源

#131 要求每文件一个 CAS 对象加 canonical manifest，且包 digest 为 manifest digest。`kernel::layout::SKILL_FILE` 仍是 `SKILL.md` 文件名的唯一权威。
-/

/-!
## 6 命名

`SkillManifest` 是清单；`SkillItem` 是一项；`SkillItemKind` 是目录或文件；`SkillManifest::file` 是 verified lookup 前的索引读取，真正的字节校验由调用方 `Cas::get` 完成。
-/

/-!
## 7 模块边界

形状 2 值类型：只持有项目、验证和编码，不做 I/O。city 构造，runtime 解码；两者都不能各自发明一份包文法。
-/

/-!
## 8 接口先行

```rust
pub struct SkillManifest { /* version、items —— 私有，构造点守不变量 */ }
pub enum SkillItem { Directory { path: String }, File { path: String, digest: B3Hash } }
impl SkillManifest {
    pub fn new(items: Vec<SkillItem>) -> Result<Self, AxError>;
    pub fn encode(&self) -> Result<Vec<u8>, AxError>;
    pub fn decode(bytes: &[u8]) -> Result<Self, AxError>;
    pub fn file(&self, path: &str) -> Option<&B3Hash>;
    pub fn document(&self) -> Option<&B3Hash>;
}
```
-/

namespace Kernel.Skill

/-- 一项的目录或文件种类；digest 与文件共存。 -/
inductive ItemKind where
  | directory
  | file
  deriving DecidableEq, Repr

/-- 模型只关心路径、种类与 digest。 -/
structure Item where
  path : String
  kind : ItemKind
  digest : Nat
  deriving DecidableEq, Repr

/-- 按路径和文件种类查找；目录与同名文件不会混答。 -/
def lookup (path : String) : List Item → Option Nat
  | [] => none
  | item :: rest =>
    if item.path = path ∧ item.kind = .file then some item.digest else lookup path rest

/-- 查到的 digest 必属于一个同名文件项。 -/
theorem lookup_some_requested (path : String) (items : List Item) (digest : Nat) :
    lookup path items = some digest →
      items.any (fun item => item.path = path ∧ item.kind = .file ∧ item.digest = digest) := by
  induction items with
  | nil => simp [lookup]
  | cons item rest ih =>
    simp only [lookup]
    split
    · intro found
      injection found with found
      subst found
      simp_all
    · intro found
      simp_all

end Kernel.Skill

/-!
## 9 工作流程

city 排序并验证项目 → 编码 manifest → 分别登记文件与 manifest → runtime 完整读取并验证 manifest → 按 `file` 找 digest → 完整读取并验证目标文件。
-/

/-!
## 10 实现逻辑

构造时排序使编码确定；`new` 与 `decode` 共用验证。`decode` 的错误码是 `E_CAS_CORRUPT`，因为坏 manifest 不能授权任何文件；源包构造的错误是 `E_INVALID_ARGS`。
-/

/-!
## 11 边界枚举

空路径、点与父段、反斜杠、冒号、重名、目录与文件同名、缺 `SKILL.md`、重复 `SKILL.md`、未知版本与截断 JSON 都必须拒绝。任意查找只返回同名文件项，如上定理所证。
-/

/-!
## 12 错误处理

源包无效不安装；manifest 无法解码或形状无效按损坏处理；找不到路径是普通 `None`，调用方另作 miss，不伪造空文件。
-/

/-!
## 13 依赖选型

只复用 `serde`／`serde_json` 与 `B3Hash`，不引入新依赖。JSON 的确定性来自 struct 字段顺序和构造时排序，而不是 map 的迭代顺序。
-/

/-!
## 14 硬编码声明

`version = 1` 是当前 manifest 文法；`SKILL.md` 的名字从 `kernel::layout::SKILL_FILE` 读取，不在 Rust 或 Lean 再写一份。
-/

/-!
## 15 影响面

city 的安装与书架索引、runtime 的 `read`、usage 的版本 digest 与审核绑定都改为 manifest digest。已有 `skill_shelved`／`skill_audited` 的历史保留，迁移后新绑定必须指向 manifest。
-/

/-!
## 16 测试与约束

Rust 测 canonical roundtrip、全部拒绝形状、查找和 UTF-8；Lean 证明查找不返回别项。`cargo nextest run -p sprawling-kernel skill` 与 `lake build crates.kernel.spec.Skill` 验收；没有声称 Lean 证明 Rust 实现等价。
-/

/-!
## 17 文档关系

`crates/city/spec/Library/Install.lean` 决定写入顺序，`crates/city/spec/Library.lean` 决定索引，`crates/runtime/spec/Tools/Read.lean` 决定完整校验读取，`crates/storage/spec/Cas.lean` 决定 digest 与 `E_CAS_CORRUPT`。
-/
