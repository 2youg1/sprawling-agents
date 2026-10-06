-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::privacy

规定 `privacy`、`privacy::answer` 与 `privacy::operation`（`crates/wire/src/` 下同名的文件）：
主机隐私页与城共享的名字、`Query::Privacy` 的答复与 `Command::PrivacyOperation` 的载荷。本文件是
`crates/wire/Spec.lean` 的一个分部；下面各节保留它们在 wire 规格里的标签 §8-85 到 §8-87，别处引作
`crates/wire/Spec.lean §8-n`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；名字集合与答复的形状由
Rust 的类型守住，每个名字背后的数据与它们进入写入路径的性质由
`crates/sprawling/spec/Privacy/Controls.lean`、`crates/sprawling/spec/Privacy.lean` 与
`crates/sprawling/spec/Privacy/Service.lean` 规定。
-/

/-!
### 8-85 隐私页的闭集

```rust
pub enum PrivacyControl { PowershellTelemetryOptout, CeipConsolidatorTask, … }  // 88 个，"powershell_telemetry_optout" …
pub enum PrivacyOriginal { K01, K02, …, K52 }                                    // "k01" … "k52"
pub enum PrivacyNotWritten { Absent, Obsolete, Undeterminable, NeedsOperationKind }
pub enum PrivacyCategory { Diagnostics, SpeechInput, LocationSensors, Search, Content,
                           ActivitySync, CloudServices, AppPermissions, WindowsAi }
pub enum PrivacyEdition { Home, Pro, Enterprise, Education, IotEnterprise, Server }
pub enum PrivacyBuildEffect { Documented, Uncertain, NoCurrentEffect }
pub enum PrivacyEditionFit { Honoured, Ignored, NotStated }
pub enum PrivacyScope { User, Machine }
pub enum PrivacySettlement { Applied, NotApplied, Restored, Abandoned }
pub enum PrivacyFaultCode { Identity, Clock, History, Unreadable, Unresolved, Changed, TargetAbsent,
                            NothingOwned, Conflict, NothingUnresolved, HistoryFull, Expired,
                            AccessDenied, ElevationDeclined, NotApplied, ReadbackMismatch,
                            Unknown, ReceiptLost }
// 每个闭集带 `ALL`：成员按声明次序，即页面次序。
```

**控制表只有一份。** 一个控制的目标路径、写入值、版本清单只在二进制的 `privacy::controls` 定义一次，
只随 `Query::Privacy` 的答复到页面（§8-86）；页面上的文字只在客户端的 `lang.json` 定义一次，按这些
名字的拼写取。名字集合因此各只有一份成员表，每个读者从这里取。

`PrivacySettlement` 也是磁盘上 `Reconciled` 一行的结论（crates/sprawling/spec/Privacy/State.lean），
`PrivacyFaultCode` 是 `bin::privacy::fault` 的稳定码（crates/sprawling/spec/Privacy.lean §12）：
二进制直接用这两个类型，所以磁盘、错误主语与线上是同一份拼写；`PrivacyFaultCode::as_str` 与 serde
的拼写由 `privacy` 旁的测试对齐。

**声明次序就是页面次序。** `PrivacyControl` 先按 `PrivacyCategory` 的次序、类别内按控制表次序声明；
`ALL` 由同一个宏从变体表生成，所以加了变体却漏掉次序的状态不存在。

**被否**：把控制的路径与写入值放进 wire 枚举的属性——客户端就能读到并复制它们，控制表成了两份；
用字符串而非闭集——页面的 `lang.json` 缺一个键时就不会在类型检查时失败。
-/

/-!
### 8-86 `Query::Privacy` 与 `PrivacyAnswer`

```rust
Query::Privacy                                   // 无字段
Answer::Privacy(Box<PrivacyAnswer>)
pub struct PrivacyAnswer { host: PrivacyHost, controls: Vec<PrivacyControlEntry>,
                           not_written: Vec<PrivacyNotWrittenEntry>, history: PrivacyHistory,
                           outcomes: Vec<PrivacyOutcome> }
pub enum PrivacyHost { Windows(PrivacyWindows), Unreadable { error: AxError }, NotWindows }
pub struct PrivacyWindows { edition_id: String, edition: Option<PrivacyEdition>, build: String,
                            display_version: Option<String> }
pub struct PrivacyControlEntry { control, category, originals: Vec<PrivacyOriginalLine>,
                                 target: PrivacyTarget, scope: PrivacyScope, editions: PrivacyEditions,
                                 host_fit: PrivacyEditionFit, build_effect: PrivacyBuildEffect,
                                 current: PrivacyCurrent, written: Option<PrivacyValue> }
pub struct PrivacyNotWrittenEntry { line: PrivacyOriginalLine, reason: PrivacyNotWritten,
                                    alternatives: Vec<PrivacyControl> }
pub enum PrivacyTarget { RegistryValueHklm { path, name }, RegistryValueHkcu { path, name },
                         EnvironmentVariableUser { name }, ScheduledTaskEnabled { path, name } }  // tag "kind"
pub enum PrivacyCurrent { Read { value: PrivacyValue }, AccessDenied, Failed { error: AxError }, NotRead }
pub enum PrivacyValue { Absent, Dword { number }, Text { text }, Raw { kind, hex }, TaskAbsent,
                        TaskEnabled { definition_sha256 }, TaskDisabled { definition_sha256 } }  // tag "value"
pub enum PrivacyHistory { Disclosed { owned: Vec<PrivacyIntent>, unresolved: Option<PrivacyIntent> },
                          Withheld { error: AxError }, Unreadable { error: AxError } }
pub struct PrivacyIntent { operation: u64, control, original: PrivacyValue, modified: PrivacyValue,
                           restore_of: Option<u64> }
```

- **答复是主机此刻的一次读。** `controls` 每个控制一条、按页面次序；`not_written` 每个不写原项一条、
  按原项次序，带研究定下的原因与最接近的控制。原项原文随答复走（`PrivacyOriginalLine.text`），
  因为它是人自己写下的句子，不是界面文字，`lang.json` 里没有它。
- **版本适用由主机判定。** `host_fit` 由控制的版本清单与主机 `EditionID` 的前缀规则一处算出
  （crates/sprawling/spec/Privacy/Controls.lean D64），页面不重算；主机不是 Windows 或版本记录读不成时，
  每个控制都是 `NotStated`。
- **`written` 是 apply 在 `current` 之上会留下的值**，由控制表的写入值算出；`current` 没读成或主机
  没有该任务时缺席。
- **历史按问的人披露。** 空历史没有 owner，答空的 `Disclosed`，不核对身份；非空历史只在实时身份通过
  owner 的核对之后答 `Disclosed`，否则答 `Withheld`，答复里没有任何记录下的值（原值只在 `Disclosed` 里，所以
  「未经核对却带着原值」不可表示）。日志被另一个操作占着或已损坏答 `Unreadable`，其余部分照常作答。
- **主机不是 Windows 时什么也不读**：`host` 为 `NotWindows`，每个控制的 `current` 为 `NotRead`。
- `outcomes` 是主机保留的操作结果（§8-87）；同一份答复里一并给出，所以页面问一次就同时看到结果与
  它之后的当前值。
-/

/-!
### 8-87 `Command::PrivacyOperation`

```rust
Command::PrivacyOperation(PrivacyRequest)
pub struct PrivacyRequest { action: PrivacyAction, idem: IdemKey }
pub enum PrivacyAction { Apply { control, expected: PrivacyValue },
                         Restore { control, expected: PrivacyValue },
                         Reconcile { expected: PrivacyValue } }
pub struct PrivacyOutcome { idem: IdemKey, action: PrivacyAction, result: PrivacyResult }
pub enum PrivacyResult { Running, Applied { operation }, Restored { operation }, AlreadyWritten,
                         Reconciled { operation, settlement: PrivacySettlement },
                         Refused { code: PrivacyFaultCode, error: AxError } }
```

- reach `client`、class `LocalOnly`（`spec/Command/Kind.lean` §19-2）：只有城自己的监听接它，远程设备
  不论权限都带不进来；`Ask { query: Privacy }` 同样 `LocalOnly`（crates/sprawling/spec/Outside/Conduit.lean）。
- **命令不等结果。** 监听接下命令即记一条 `Running`，答复为空；操作在主机上一个接一个执行，结束后
  该 idem 的结果改为终态。页面再问 `Query::Privacy` 时按自己铸的 idem 取回结果，不读全局的拒绝帧，
  所以别的命令的拒绝不会改写隐私页的状态。载荷读不成（`expected` 不是唯一拼写）时命令当场以
  `E_INVALID_ARGS` 拒绝，不记结果。
- **同一 idem 只执行一次。** 已有结果的 idem 再到，不再执行，页面读到的仍是第一次的结果
  （crates/sprawling/spec/Privacy/Service.lean）。
-/

/-! D49 线上的值是主机读到的原样，页面原样送回作 expected

`PrivacyValue` 对每个快照只有一种拼写：四字节的 `REG_DWORD` 写作 `Dword`，字节恰为 UTF-16 加一个结尾 NUL
的 `REG_SZ` 写作 `Text`，其余注册表值写作 `Raw`（类型码与小写十六进制字节），任务带定义摘要的十六进制。
所以页面显示的值可以直接读，送回时主机把它换回字节、与 fresh read 逐字节比较；换回之后再换出不等于
原拼写的（例如一个本该写作 `Dword` 的 `Raw`）拒绝。
**理由**：确认绑定的是人看到的值（crates/sprawling/spec/Privacy/Confirmation.lean D55），绑定必须是
精确的；同时页面要把值显示给人看，一份只给字节的值让每个读者各写一遍解码。
**被否**：①只送显示用的有损形式（DWORD 数字、文字、任务启用与否）并在主机比较显示形式——任务摘要与
不规范的字符串会丢，两个不同的快照显示相同时，确认就绑不住；②值与显示形式各送一份——同一事实两份，
页面可能送回与显示不一致的那一份。
**重开参数**：控制表出现 `REG_DWORD`、`REG_SZ` 之外的写入类型时，再考虑给它一种可读的拼写。
-/

/-! D50 没有「全部恢复」的帧：页面为每个仍拥有的控制各发一次 Restore

restore-all 是逐个控制的单次恢复（crates/sprawling/spec/Privacy.lean D60），每次机器作用域的写入各要一次
管理员批准；页面按 `PrivacyHistory::Disclosed.owned` 为每一条发一次 `Restore`，各带自己的 idem 与它显示的
当前值，主机按到达次序一个接一个执行，每个结果各自可读。
**被否**：`RestoreAll { expected: Vec<…> }` 一帧——要再定义一种「一批结果」的形状，表达的仍是同样的
一串单次恢复，UAC 次数也不变。
**重开参数**：若机器作用域的写入可以合成一次提升而不违反「一次只有一个未结操作」（D60 的被否方案），
再考虑一帧批量。
-/
